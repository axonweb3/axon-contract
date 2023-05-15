// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec::Vec;
use alloc::{collections::BTreeSet, vec};
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{withdraw_reader, Cursor};
use util::{error::Error, helper::*};

// #[derive(PartialEq, Eq)]
struct WithdrawInfo {
    epoch: u64,
    amount: u128,
}
impl PartialEq for WithdrawInfo {
    fn eq(&self, other: &WithdrawInfo) -> bool {
        self.epoch == other.epoch && self.amount == other.amount
    }
}
impl Eq for WithdrawInfo {}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    let withdraw_args: withdraw_reader::WithdrawArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = withdraw_args.metadata_type_id();

    let type_ids = get_type_ids(
        &metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    if metadata_type_id != type_ids.metadata_type_id() {
        return Err(Error::MisMatchMetadataTypeId);
    }
    let owner = withdraw_args.addr();

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    let withdraw_witness = {
        let witness_lock = witness_args.lock().to_opt();
        if witness_lock.is_none() {
            None
        } else {
            let value: withdraw_reader::WithdrawWitness =
            Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
            Some(value)   
        }
    };

    let withdraw_at_lock_hash = { load_cell_lock_hash(0, Source::GroupInput)? };
    let (in_amount, in_data) =
        get_withdraw_at_data_by_lock_hash(&withdraw_at_lock_hash, Source::GroupInput)?;
    let (out_amount, out_data) =
        get_withdraw_at_data_by_lock_hash(&withdraw_at_lock_hash, Source::GroupOutput)?;
    let epoch = get_current_epoch(&type_ids.checkpoint_type_id())?;

    if withdraw_witness.is_none() {
        // ACP mode, someone is unstake or undelgate
        if out_amount <= in_amount {
            return Err(Error::OutLessThanIn);
        }
        let increased_amount = out_amount - in_amount;

        let in_data = in_data.withdraw_infos();
        let mut unlock_amount: u128 = 0u128; // can be withdraw
        let mut lock1_amount: u128 = 0u128; // can be withdraw in next epoch
        let mut lock2_amount: u128 = increased_amount; // can be withdra in next + 1 epoch
        let mut new_withdraw_infos = vec![];
        for i in 0..in_data.len() {
            let withdraw_info = in_data.get(i);
            if withdraw_info.epoch() <= epoch {
                unlock_amount += bytes_to_u128(&withdraw_info.amount());
                new_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch,
                    amount: unlock_amount,
                });
            } else if withdraw_info.epoch() == epoch + 1 {
                lock1_amount = bytes_to_u128(&withdraw_info.amount());
                new_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 1,
                    amount: lock1_amount,
                });
            } else if withdraw_info.epoch() == epoch + 2 {
                lock2_amount = bytes_to_u128(&withdraw_info.amount());
                new_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 2,
                    amount: lock2_amount,
                });
            } else {
                return Err(Error::WrongLockEpoch);
            }
        }

        let out_data = out_data.withdraw_infos();
        let mut out_withdraw_infos = vec![];
        for i in 0..out_data.len() {
            let withdraw_info = out_data.get(i);
            if withdraw_info.epoch() == epoch {
                out_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch,
                    amount: bytes_to_u128(&withdraw_info.amount()),
                })
            } else if withdraw_info.epoch() == epoch + 1 {
                out_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 1,
                    amount: bytes_to_u128(&withdraw_info.amount()),
                });
            } else if withdraw_info.epoch() == epoch + 2 {
                out_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 2,
                    amount: bytes_to_u128(&withdraw_info.amount()),
                });
            } else {
                return Err(Error::WrongOutWithdrawEpoch);
            }
        }

        if new_withdraw_infos != out_withdraw_infos {
            return Err(Error::WrongOutWithdraw);
        }
    } else {
        // unlock mode,
        let in_data = in_data.withdraw_infos();
        let mut unlock_amount: u128 = 0u128; // can be withdraw
        let mut lock1_amount: u128 = 0u128; // can be withdraw in next epoch
        let mut lock2_amount: u128 = 0u128; // can be withdra in next + 1 epoch
        let mut new_withdraw_infos = vec![];
        for i in 0..in_data.len() {
            let withdraw_info = in_data.get(i);
            if withdraw_info.epoch() <= epoch {
                unlock_amount += bytes_to_u128(&withdraw_info.amount());
            } else if withdraw_info.epoch() == epoch + 1 {
                lock1_amount = bytes_to_u128(&withdraw_info.amount());
                new_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 1,
                    amount: lock1_amount,
                });
            } else if withdraw_info.epoch() == epoch + 2 {
                lock2_amount = bytes_to_u128(&withdraw_info.amount());
                new_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 2,
                    amount: lock2_amount,
                });
            } else {
                return Err(Error::WrongLockEpoch);
            }
        }

        if in_amount - out_amount != unlock_amount {
            return Err(Error::WithdrawTotalAmountError);
        }

        let out_data = out_data.withdraw_infos();
        if out_data.len() > 2 {
            return Err(Error::WrongOutWithdrawArraySize);
        }

        let mut out_withdraw_infos = vec![];
        for i in 0..out_data.len() {
            let withdraw_info = out_data.get(i);
            if withdraw_info.epoch() == epoch + 1 {
                out_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 1,
                    amount: bytes_to_u128(&withdraw_info.amount()),
                });
            } else if withdraw_info.epoch() == epoch + 2 {
                out_withdraw_infos.push(WithdrawInfo {
                    epoch: epoch + 2,
                    amount: bytes_to_u128(&withdraw_info.amount()),
                });
            } else {
                return Err(Error::WrongOutWithdrawEpoch);
            }
        }

        if new_withdraw_infos != out_withdraw_infos {
            return Err(Error::WrongOutWithdraw);
        }
    }

    // get input normal at cell and output noram at cell, verify amount increased by unlock_amount.
    let input_total_amount =
        get_xudt_by_type_hash(&type_ids.xudt_type_id(), Source::GroupInput)?;
    let output_total_amount =
        get_xudt_by_type_hash(&type_ids.xudt_type_id(), Source::GroupOutput)?;
    if input_total_amount > output_total_amount {
        return Err(Error::WrongIncreasedXudt);
    }
    Ok(())
}
