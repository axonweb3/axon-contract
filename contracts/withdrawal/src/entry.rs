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
    high_level::{
        load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_script, load_witness_args,
        QueryIter,
    },
};

use util::{error::Error, helper::*};
use protocol::{reader as axon, Cursor};

enum MODE {
    ACP,
    BURN,
    UNLOCK,
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract parameters from lock_args
    let withdrawal_args: axon::WithdrawalLockArgs = Cursor::from(args.to_vec()).into();
    let admin_identity = withdrawal_args.admin_identity();
    let checkpoint_cell_type_hash = withdrawal_args.checkpoint_cell_type_hash();
    let node_identity = withdrawal_args.node_identity();

    // identify contract mode by witness
    let mode = match load_witness_args(0, Source::GroupInput) {
        Ok(witness) => {
            let value = witness.input_type().to_opt();
            if value.is_none() || value.as_ref().unwrap().len() != 1 {
                return Err(Error::BadWitnessInputType);
            }
            if value.unwrap().raw_data().to_vec().first().unwrap() == &0 {
                MODE::BURN
            } else {
                MODE::UNLOCK
            }
        }
        Err(_) => MODE::ACP,
    };

    // extract AT script_hash from type_script
    let at_type_hash = {
        let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
        if type_hash.is_none() {
            return Err(Error::TypeScriptEmpty);
        }
        type_hash.unwrap()
    };

    // mode select
    match mode {
        MODE::BURN => {
            debug!("burn mode");
            // check admin signature
            if !secp256k1::verify_signature(&admin_identity.content()) {
                return Err(Error::SignatureMismatch);
            }
            let mut at_cell_count = 0;
            QueryIter::new(load_cell_type_hash, Source::Output).for_each(|type_hash| {
                if type_hash.unwrap_or([0u8; 32]) == at_type_hash {
                    at_cell_count += 1;
                }
            });
            if at_cell_count != 0 {
                return Err(Error::ATCellShouldEmpty);
            }
        }
        MODE::UNLOCK => {
            debug!("unlock mode");
            if node_identity.is_none() {
                return Err(Error::NodeIdentityEmpty);
            }
            // check normal signature
            if !secp256k1::verify_signature(&node_identity.unwrap().content()) {
                return Err(Error::SignatureMismatch);
            }
            // load checkpoint cell_data from celldeps
            let mut find_checkpoint = false;
            QueryIter::new(load_cell_type_hash, Source::CellDep)
                .enumerate()
                .map(|(i, type_hash)| {
                    if type_hash.unwrap_or([0u8; 32]) == checkpoint_cell_type_hash.as_slice() {
                        assert!(find_checkpoint == false);
                        find_checkpoint = true;
                        let checkpoint_period = {
                            let data = load_cell_data(i, Source::CellDep);
                            if let Err(_) = data {
                                return Err(Error::BadCheckpointCelldep);
                            }
                            let checkpoint_data: axon::CheckpointLockCellData =
                                Cursor::from(data.unwrap()).into();
                            bytes_to_u64(&checkpoint_data.period())
                        };
                        let withdrawal_period = {
                            let data = load_cell_data(0, Source::GroupInput);
                            if data.is_err() || data.as_ref().unwrap().len() != 24 {
                                return Err(Error::BadWithdrawalData);
                            }
                            bytes_to_u64(&data.unwrap()[16..].to_vec())
                        };
                        if withdrawal_period > checkpoint_period {
                            return Err(Error::BadWithdrawalPeriod);
                        }
                    }
                    Ok(())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if !find_checkpoint {
                return Err(Error::CheckpointCelldepEmpty);
            }
        }
        MODE::ACP => {
            debug!("acp mode");
            if node_identity.is_none() {
                return Err(Error::NodeIdentityEmpty);
            }
            // check input and output total amount in withdrawal cell_data
            let withdrawal_lock_hash: [u8; 32] = load_cell_lock_hash(0, Source::GroupInput)?;
            let input_total_sudt =
                get_total_sudt_by_script_hash(&withdrawal_lock_hash, &at_type_hash, Source::Input)?;
            let output_total_sudt = get_total_sudt_by_script_hash(
                &withdrawal_lock_hash,
                &at_type_hash,
                Source::Output,
            )?;
            if output_total_sudt < input_total_sudt {
                return Err(Error::TotalSudtAmountMismatch);
            }
        }
    }

    Ok(())
}
