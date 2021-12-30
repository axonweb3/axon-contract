// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    debug, ckb_constants::Source, high_level::{
		load_script, load_cell_lock_hash, QueryIter
	}, ckb_types::{
		bytes::Bytes, prelude::*
	},
};

use crate::error::Error;
use protocol::{
	axon, Cursor
};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

	// extract omni and checkpoint lock_hash from script_args
	let selection_args: axon::SelectionLockArgs = Cursor::from(args.to_vec()).into();
	let omni_lock_hash = selection_args.omni_lock_hash();
	let checkpoint_lock_hash = selection_args.checkpoint_lock_hash();

	// count omni and checkpoint cells count
	let mut omni_cells_count = 0;
	let mut checkpoint_cells_count = 0;

	// search omni and checkpoint cells via ckb functions
	QueryIter::new(load_cell_lock_hash, Source::Input)
		.for_each(|lock_hash| {
			if &lock_hash == omni_lock_hash.as_slice() {
				omni_cells_count += 1;
			} else if &lock_hash == checkpoint_lock_hash.as_slice() {
				checkpoint_cells_count += 1;
			}
		});

	debug!("omni = {}, checkpoint = {}", omni_cells_count, checkpoint_cells_count);

	// sum of omni and checkpoint must be 1
	if omni_cells_count + checkpoint_cells_count != 1 {
		return Err(Error::OmniCheckpointCountError);
	}

    Ok(())
}
