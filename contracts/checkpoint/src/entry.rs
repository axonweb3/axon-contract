// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_script, load_witness_args}
};

use util::{error::Error, helper::*};
use protocol::{reader as axon, Cursor};
use keccak_hash::keccak;
use rlp::{Rlp, RlpStream};
use bit_vec::BitVec;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    let checkpoint_args: axon::CheckpointLockArgs = Cursor::from(args.to_vec()).into();
    let admin_identity = checkpoint_args.admin_identity();
    let type_id_hash = checkpoint_args.type_id_hash();

    // check input and output capacity and data from checkpoint cells
    let (input_checkpoint_capacity, input_checkpoint_data) =
        get_info_by_type_hash(&type_id_hash, Source::Input)?;
    let (output_checkpoint_capacity, output_checkpoint_data) =
        get_info_by_type_hash(&type_id_hash, Source::Output)?;
    if input_checkpoint_capacity != output_checkpoint_capacity {
        return Err(Error::CheckpointCapacityMismatch);
    }
    if input_checkpoint_data.version() != output_checkpoint_data.version()
        || input_checkpoint_data.period_interval() != output_checkpoint_data.period_interval()
        || input_checkpoint_data.era_period() != output_checkpoint_data.era_period()
        || input_checkpoint_data.base_reward() != output_checkpoint_data.base_reward()
        || input_checkpoint_data.half_period() != output_checkpoint_data.half_period()
        || input_checkpoint_data.sudt_type_hash() != output_checkpoint_data.sudt_type_hash()
        || input_checkpoint_data.stake_type_hash() != output_checkpoint_data.stake_type_hash()
        || input_checkpoint_data.withdrawal_lock_code_hash()
            != output_checkpoint_data.withdrawal_lock_code_hash()
    {
        return Err(Error::CheckpointDataMismatch);
    }

    // check this is wether admin mode or checkpoint mode
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    let is_admin_mode = {
        let input_type = witness_args.input_type().to_opt();
        if input_type.is_none() {
            return Err(Error::BadWitnessInputType);
        }
        match input_type.unwrap().raw_data().to_vec().first() {
            Some(value) => value == &0,
            None => return Err(Error::BadWitnessInputType),
        }
    };

    // get AT coins from AT cell
    let sudt_type_hash = input_checkpoint_data.sudt_type_hash();
    let input_at_amount = get_sudt_by_type_hash(&sudt_type_hash, Source::Input)?;
    let output_at_amount = get_sudt_by_type_hash(&sudt_type_hash, Source::Output)?;

    // admin mode
    if is_admin_mode {
        debug!("admin mode");
        // check admin signature
        if !blst::verify_secp256k1_signature(&admin_identity.content()) {
            return Err(Error::SignatureMismatch);
        }
        // check AT amount
        if input_at_amount < output_at_amount {
            return Err(Error::ATAmountMismatch);
        }
    // checkpoint mode
    } else {
        debug!("checkpoint mode");
        if input_checkpoint_data.state() != output_checkpoint_data.state()
            || input_checkpoint_data.unlock_period() != output_checkpoint_data.unlock_period()
        {
            return Err(Error::CheckpointDataMismatch);
        }

		// extract proposal and proof data from witness lock
		let (proposal, proof) = {
			let witness_lock = witness_args.lock().to_opt();
			if witness_lock.is_none() {
				return Err(Error::WitnessLockError);
			}
			let value: axon::CheckpointLockWitnessLock = 
				Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
			(value.proposal(), value.proof())
		};

		// get hash of proposal and check equality with hash in proof
		let proposal_rlp = Rlp::new(&proposal);
		let block_hash = proposal_rlp.at(2).map_err(|_| Error::ProposalRlpError)?;
		if keccak(proposal.clone()).as_bytes().to_vec() != block_hash.as_raw() {
			return Err(Error::BlockHashMismatch);
		}

		// get validate stake_infos from stake cell in cell_dep and check pBFT consensus validation
		let proof_rlp = Rlp::new(&proof);
		let era = bytes_to_u64(&output_checkpoint_data.era());
		let valid_nodes = get_valid_stakeinfos_from_celldeps(era, &input_checkpoint_data.stake_type_hash())?;
		let nodes_bitmap = BitVec::from_bytes(proof_rlp.at(4).map_err(|_| Error::ProofRlpError)?.as_raw());
		let active_num = nodes_bitmap.iter().filter(|b| *b).count();
		if active_num <= valid_nodes.len() * 2 / 3 {
			return Err(Error::ActiveNodesNotEnough);
		}

		// prepare signing message and check blst signature validation
		let height: u64 = proof_rlp.val_at(0).map_err(|_| Error::ProofRlpError)?;
		let round: u64 = proof_rlp.val_at(1).map_err(|_| Error::ProofRlpError)?;
		let mut message = RlpStream::new();
		message.append(&height);
		message.append(&round);
		message.append(&1u8);
		message.append(&block_hash.as_raw());
		let signature = proof_rlp.at(3).map_err(|_| Error::ProofRlpError)?.as_raw().to_vec();
		if signature.len() != 96 {
			return Err(Error::ProofRlpError);
		}
		let active_pubkeys = nodes_bitmap
			.into_iter()
			.enumerate()
			.filter_map(|(i, flag)| {
				if flag {
					if let Some(node) = valid_nodes.get(i) {
						return Some(Ok(node.bls_pub_key));
					} else {
						return Some(Err(Error::NodesBitmapMismatch));
					}
				}
				None
			})
			.collect::<Result<Vec<_>, _>>()?;
		if !blst::verify_blst_signature(&active_pubkeys, &signature, &message.as_raw().to_vec()) {
			return Err(Error::SignatureMismatch);
		}

		// check checkpoint data with decoded rlp data
		let period = bytes_to_u64(&output_checkpoint_data.period());
		let era_period = bytes_to_u32(&output_checkpoint_data.era_period()) as u64;
		if era_period == 0 {
			return Err(Error::CheckpointDataError);
		}
		let last_block_hash = proposal_rlp.at(11).map_err(|_| Error::ProposalRlpError)?;
		if u8::from(input_checkpoint_data.state()) != 1
			|| period != bytes_to_u64(&input_checkpoint_data.period()) + 1
			|| era != period / era_period
			|| output_checkpoint_data.block_hash() != block_hash.as_raw().to_vec()
			|| period * bytes_to_u32(&output_checkpoint_data.period_interval()) as u64 != height
			|| input_checkpoint_data.block_hash() != last_block_hash.as_raw().to_vec()
		{
			return Err(Error::CheckpointRlpDataMismatch);
		}

        // check AT amount
        let base_reward = bytes_to_u128(&input_checkpoint_data.base_reward());
        let period = bytes_to_u64(&input_checkpoint_data.period());
        let half_period = bytes_to_u64(&input_checkpoint_data.half_period());
        if half_period == 0 {
            return Err(Error::CheckpointDataError);
        }
		let at_amount_diff = base_reward / 2u128.pow((period / half_period) as u32);
        if output_at_amount - input_at_amount != at_amount_diff {
            return Err(Error::ATAmountMismatch);
        }

        // find node_identity and construct withdrawal lock
		let proposer_address = proposal_rlp.at(1).map_err(|_| Error::ProposalRlpError)?;
		let mut node_identity = None;
		valid_nodes
			.iter()
			.for_each(|node| {
				if node.l2_address == proposer_address.as_raw() {
					node_identity = Some(node.identity);
				}
			});
		if node_identity.is_none() {
			return Err(Error::ProposerAddressMismatch);
		}
		let withdrawal_lock_hash = calc_withdrawal_lock_hash(
			&input_checkpoint_data.withdrawal_lock_code_hash(),
			admin_identity,
			&type_id_hash,
			&node_identity.unwrap()
		);

		// check AT amount from input and output witdrawal AT cell
		let unlock_period = bytes_to_u32(&output_checkpoint_data.unlock_period()) as u64;
		let input_withdrawal_at_amount = get_withdrawal_total_sudt_amount(
			&withdrawal_lock_hash, &sudt_type_hash, 0, Source::Input
		)?;
		let output_withdrawal_at_amount = get_withdrawal_total_sudt_amount(
			&withdrawal_lock_hash, &sudt_type_hash, period + unlock_period, Source::Output
		)?;
		if output_withdrawal_at_amount - input_withdrawal_at_amount != at_amount_diff {
			return Err(Error::WithdrawalATAmountMismatch);
		}
    }

    Ok(())
}
