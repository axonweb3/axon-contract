// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    debug, ckb_constants::Source, high_level::{
		load_script, load_cell_type_hash, load_cell_capacity, load_cell_data, load_witness_args, QueryIter
	}, ckb_types::{
		bytes::Bytes, prelude::*
	},
};

use crate::error::Error;
use protocol::{
	axon, Cursor
};

fn get_info_by_type_hash(type_hash: &Vec<u8>, source: Source) -> Result<(u64, axon::CheckpointLockCellData), Error> {
	let mut capacity = 0u64;
	let mut celldata = None;
	QueryIter::new(load_cell_type_hash, source)
		.enumerate()
		.map(|(i, cell_type_hash)| {
			if cell_type_hash.unwrap_or([0u8; 32]) != type_hash[..] {
				return Ok(())
			}
			if celldata.is_some() {
				return Err(Error::CheckpointCellError)
			}
			match load_cell_capacity(i, source) {
				Ok(value) => capacity = value,
				Err(err)  => return Err(Error::from(err))
			}
			match load_cell_data(i, source) {
				Ok(value) => celldata = Some(axon::CheckpointLockCellData::from(Cursor::from(value))),
				Err(err)  => return Err(Error::from(err))
			}
			Ok(())
		})
		.collect::<Result<Vec<_>, _>>()?;
	if celldata.is_none() {
		return Err(Error::CheckpointCellError)
	}
	Ok((capacity, celldata.unwrap()))
}

fn get_capacity_by_type_hash(type_hash: &Vec<u8>, source: Source) -> Result<u64, Error> {
	let mut capacity = 0u64;
	QueryIter::new(load_cell_type_hash, source)
		.enumerate()
		.map(|(i, cell_type_hash)| {
			if cell_type_hash.unwrap_or([0u8; 32]) == type_hash[..] {
				match load_cell_capacity(i, source) {
					Ok(value) => capacity += value,
					Err(err)  => return Err(Error::from(err))
				}
			}
			Ok(())
		})
		.collect::<Result<Vec<_>, _>>()?;
	Ok(capacity)
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:?}", args);

	let checkpoint_args: axon::CheckpointLockArgs = Cursor::from(args.to_vec()).into();
	let admin_identity = checkpoint_args.admin_identity();
	let type_id_hash = checkpoint_args.type_id_hash();

	// check input and output capacity and data from checkpoint cells
	let (input_checkpoint_capacity, input_checkpoint_data) = get_info_by_type_hash(&type_id_hash, Source::Input)?;
	let (output_checkpoint_capacity, output_checkpoint_data) = get_info_by_type_hash(&type_id_hash, Source::Output)?;
	if input_checkpoint_capacity != output_checkpoint_capacity {
		return Err(Error::CheckpointCapacityMismatch);
	}
	if input_checkpoint_data.version() != output_checkpoint_data.version()
		|| input_checkpoint_data.period_interval() != output_checkpoint_data.period_interval()
		|| input_checkpoint_data.era_period() != output_checkpoint_data.era_period()
		|| input_checkpoint_data.base_reward() != output_checkpoint_data.base_reward()
		|| input_checkpoint_data.half_period() != output_checkpoint_data.half_period()
		|| input_checkpoint_data.sudt_type_hash() != output_checkpoint_data.sudt_type_hash()
		|| input_checkpoint_data.stake_type_hash() != output_checkpoint_data.stake_type_hash() {
		return Err(Error::CheckpointDataMismatch);
	}

	// check this is wether admin mode or checkpoint mode
	let witness_args = load_witness_args(0, Source::GroupInput)?;
	let checkpoint_witness_lock: axon::CheckpointLockWitnessLock = {
		let witness_lock = witness_args.lock().to_opt();
		if witness_lock.is_none() {
			return Err(Error::WitnessLockEmpty);
		}
		let bytes: Bytes = witness_lock.unwrap().unpack();
		Cursor::from(bytes.to_vec()).into()
	};
	
	// admin mode
	if checkpoint_witness_lock.signature().is_some() {
		if !secp256k1::verify_signature(&mut admin_identity.content()) {
			return Err(Error::SignatureMismatch);
		}
		let sudt_type_hash = input_checkpoint_data.sudt_type_hash();
		let input_at_amount = get_capacity_by_type_hash(&sudt_type_hash, Source::Input)?;
		let output_at_amount = get_capacity_by_type_hash(&sudt_type_hash, Source::Output)?;
		if input_at_amount < output_at_amount {
			return Err(Error::ATAmountMismatch);
		}
	// checkpoint mode
	} else {
		if checkpoint_witness_lock.checkpoint().is_none() {
			return Err(Error::WitnessLockError);
		}
		if input_checkpoint_data.state() != output_checkpoint_data.state()
			|| input_checkpoint_data.unlock_period() != output_checkpoint_data.unlock_period()
			|| input_checkpoint_data.stake_type_hash() != output_checkpoint_data.stake_type_hash() {
			return Err(Error::CheckpointDataMismatch);
		}

	}

    Ok(())
}
