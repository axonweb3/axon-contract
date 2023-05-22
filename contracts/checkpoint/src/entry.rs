// Import from `core` instead of from `std` since we are in no-std mode
use core::{convert::TryInto, result::Result};

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_script, load_witness_args},
};

use axon_types::{
    checkpoint_reader::{self as axon, CheckpointCellData},
    Cursor,
};
use bit_vec::BitVec;
use keccak_hash::keccak;
use rlp::{Rlp, RlpStream};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("args len: {}", args.len());

    let checkpoint_args: axon::CheckpointArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = checkpoint_args.metadata_type_id();
    let type_ids = get_type_ids(
        metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;

    debug!("get_checkpoint_by_type_id");
    // check input and output capacity and data from checkpoint cells
    let (input_checkpoint_capacity, input_checkpoint_data) =
        get_checkpoint_by_type_id(&type_ids.checkpoint_type_id(), Source::Input)?;
    let (output_checkpoint_capacity, output_checkpoint_data) =
        get_checkpoint_by_type_id(&type_ids.checkpoint_type_id(), Source::Output)?;
    debug!("checkpoint_capacity");
    if input_checkpoint_capacity != output_checkpoint_capacity {
        return Err(Error::CheckpointCapacityMismatch);
    }

    // verify_multsig(&output_checkpoint_data)?;

    verify_checkpoint_data(&input_checkpoint_data, &output_checkpoint_data)?;

    Ok(())
}

fn verify_checkpoint_data(
    input_checkpoint_data: &CheckpointCellData,
    output_checkpoint_data: &CheckpointCellData,
) -> Result<(), Error> {
    if input_checkpoint_data.version() != output_checkpoint_data.version()
        || input_checkpoint_data.metadata_type_id() != output_checkpoint_data.metadata_type_id()
    {
        return Err(Error::CheckpointDataMismatch);
    }

    // check checkpoint data with decoded rlp data
    let input_period = input_checkpoint_data.period();
    let input_epoch = input_checkpoint_data.epoch();
    if input_epoch == 0 {
        debug!("input_checkpoint_data epoch = 0");
        return Err(Error::CheckpointDataError);
    }
    let output_period = output_checkpoint_data.period();
    let output_epoch = output_checkpoint_data.epoch();

    debug!("input_checkpoint_data metadata_type_id");
    let metadata_type_id: [u8; 32] = input_checkpoint_data
        .metadata_type_id()
        .as_slice()
        .try_into()
        .unwrap();
    // let metadata_type_id = *metadata_type_id;
    let epoch_len = get_epoch_len(&metadata_type_id, Source::CellDep)?;
    if input_period == epoch_len {
        if output_period != 0 || output_epoch != input_epoch + 1 {
            debug!(
                "output_period = {}, output_epoch = {}, input_epoch = {}",
                output_period, output_epoch, input_epoch
            );
            return Err(Error::CheckpointDataError);
        }
    } else {
        if output_period != input_period + 1 || output_epoch != input_epoch {
            debug!(
                "output_period = {}, output_epoch = {}, input_period = {}, input_epoch = {}",
                output_period, output_epoch, input_period, input_epoch
            );
            return Err(Error::CheckpointDataError);
        }
    }

    Ok(())
}

fn verify_multsig(output_checkpoint_data: &CheckpointCellData) -> Result<(), Error> {
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    // extract proposal and proof data from witness lock
    let (proposal, proof) = {
        let witness_lock = witness_args.lock().to_opt();
        if witness_lock.is_none() {
            return Err(Error::WitnessLockError);
        }
        let value: axon::CheckpointWitness =
            Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
        (value.proposal(), value.proof())
    };

    // get hash of proposal and check equality with hash in proof
    let proof_rlp = Rlp::new(&proof);
    let block_hash: Vec<u8> = proof_rlp.val_at(2).map_err(|_| Error::ProofRlpError)?;
    let proposal_hash = keccak(proposal.clone()).as_bytes().to_vec();
    if proposal_hash != block_hash {
        return Err(Error::ProofRlpError);
    }

    // the following mulsig check of l2 validators is mock！！
    // get validate stake_infos from stake cell in cell_dep and check pBFT consensus validation
    let epoch = output_checkpoint_data.epoch();
    let metadata_type_id = Vec::<u8>::new();
    let bls_pub_keys = get_current_validators(
        &metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    let nodes_bitmap = {
        let bitmap: Vec<u8> = proof_rlp.val_at(4).map_err(|_| Error::ProofRlpError)?;
        BitVec::from_bytes(bitmap.as_slice())
    };
    let active_num = nodes_bitmap.iter().filter(|b| *b).count();
    debug!(
        "epoch = {}, nodes = {}/{}",
        epoch,
        active_num,
        bls_pub_keys.len()
    );
    if active_num <= bls_pub_keys.len() * 2 / 3 {
        return Err(Error::ProofRlpError);
    }

    // prepare signing message and check blst signature validation
    let height: u64 = proof_rlp.val_at(0).map_err(|_| Error::ProofRlpError)?;
    let round: u64 = proof_rlp.val_at(1).map_err(|_| Error::ProofRlpError)?;
    debug!("height = {}, round = {}", height, round);
    let mut message = RlpStream::new();
    message
        .begin_list(4)
        .append(&height)
        .append(&round)
        .append(&2u8)
        .append(&block_hash);
    let signature: Vec<u8> = proof_rlp.val_at(3).map_err(|_| Error::ProofRlpError)?;
    if signature.len() != 96 {
        return Err(Error::ProofRlpError);
    }
    let active_pubkeys = nodes_bitmap
        .into_iter()
        .enumerate()
        .filter_map(|(i, flag)| {
            if flag {
                if let Some(pub_key) = bls_pub_keys.get(i) {
                    return Some(Ok(*pub_key));
                } else {
                    return Some(Err(Error::ProofRlpError));
                }
            }
            None
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !blst::verify_blst_signature(&active_pubkeys, &signature, &message.as_raw().to_vec()) {
        return Err(Error::SignatureMismatch);
    }

    Ok(())
}
