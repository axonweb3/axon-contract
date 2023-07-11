// Import from `core` instead of from `std` since we are in no-std mode
use core::{convert::TryInto, result::Result};

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
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
    let checkpoint_type_id = util::helper::calc_script_hash(&script).to_vec();
    debug!("checkpoint_type_id = {:?}", checkpoint_type_id);
    let input_checkpoint_count = get_cell_count_by_type_hash(&checkpoint_type_id, Source::Input);
    if input_checkpoint_count == 0 {
        debug!("checkpoint cell creation");
        return Ok(());
    }

    // check input and output capacity and data from checkpoint cells
    let (input_checkpoint_capacity, input_checkpoint_data) =
        get_checkpoint_by_type_id(&checkpoint_type_id, Source::Input)?;
    let (output_checkpoint_capacity, output_checkpoint_data) =
        get_checkpoint_by_type_id(&checkpoint_type_id, Source::Output)?;
    debug!("checkpoint_capacity");
    if input_checkpoint_capacity != output_checkpoint_capacity {
        return Err(Error::CheckpointCapacityMismatch);
    }

    debug!("input_checkpoint_data metadata_type_id");
    let metadata_type_id: [u8; 32] = input_checkpoint_data
        .metadata_type_id()
        .as_slice()
        .try_into()
        .unwrap();

    debug!("verify_multsig");
    verify_multsig(&metadata_type_id, &output_checkpoint_data)?;

    debug!("verify_checkpoint_data");
    verify_checkpoint_data(
        &metadata_type_id,
        &input_checkpoint_data,
        &output_checkpoint_data,
    )?;

    Ok(())
}

fn verify_checkpoint_data(
    metadata_type_id: &[u8; 32],
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
    // if input_epoch == 0 {
    //     debug!("input_checkpoint_data epoch = 0");
    //     return Err(Error::CheckpointDataError);
    // }
    let output_period = output_checkpoint_data.period();
    let output_epoch = output_checkpoint_data.epoch();

    // let metadata_type_id = *metadata_type_id;
    let epoch_len = get_epoch_len(&metadata_type_id, Source::CellDep)?;
    if input_period == epoch_len - 1 {
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

fn verify_multsig(
    metadata_type_id: &[u8; 32],
    output_checkpoint_data: &CheckpointCellData,
) -> Result<(), Error> {
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    // extract proposal and proof data from witness lock
    let (proposal, proof) = {
        let witness_input_type = witness_args.input_type().to_opt();
        if witness_input_type.is_none() {
            return Err(Error::WitnessLockError);
        }
        let value: axon::CheckpointWitness =
            Cursor::from(witness_input_type.unwrap().raw_data().to_vec()).into();
        (value.proposal(), value.proof())
    };

    // get hash of proposal and check equality with hash in proof
    let proof_rlp = Rlp::new(&proof);
    let block_hash: Vec<u8> = proof_rlp.val_at(2).map_err(|_| Error::ProofRlpError)?;
    let proposal_hash = keccak(proposal.clone()).as_bytes().to_vec();
    debug!(
        "block_hash: {:?}, proposal_hash: {:?}",
        block_hash, proposal_hash
    );
    if proposal_hash != block_hash {
        return Err(Error::CheckpointProposalHashMismatch);
    }

    // the following mulsig check of l2 validators is mock！！
    // get validate stake_infos from stake cell in cell_dep and check pBFT consensus validation
    let epoch = output_checkpoint_data.epoch();
    let bls_pub_keys = get_current_validators(metadata_type_id, Source::CellDep)?;
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
        return Err(Error::CheckpointLackOfQuorum);
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
    debug!("verify_blst_signature");
    if !blst::verify_blst_signature(&active_pubkeys, &signature, &message.as_raw().to_vec()) {
        return Err(Error::SignatureMismatch);
    }

    Ok(())
}
