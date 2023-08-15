// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec;
use alloc::vec::Vec;
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_script, load_witness_args},
};

use axon_types::{
    withdraw_reader::{self, WithdrawInfos},
    Cursor,
};
use util::{error::Error, helper::*};

// #[derive(PartialEq, Eq)]
#[derive(Debug)]
struct WithdrawInfo {
    unlock_epoch: u64,
    amount: u128,
}
impl PartialEq for WithdrawInfo {
    fn eq(&self, other: &WithdrawInfo) -> bool {
        self.unlock_epoch == other.unlock_epoch && self.amount == other.amount
    }
}
impl Eq for WithdrawInfo {}

fn get_withdraw_infos(
    current_epoch: u64,
    unlock_amount: u128,
    lock1_amount: u128,
    lock2_amount: u128,
) -> Vec<WithdrawInfo> {
    let mut new_withdraw_infos = vec![];
    if unlock_amount != 0 {
        new_withdraw_infos.push(WithdrawInfo {
            unlock_epoch: current_epoch,
            amount: unlock_amount,
        });
    }
    if lock1_amount != 0 {
        new_withdraw_infos.push(WithdrawInfo {
            unlock_epoch: current_epoch + 1,
            amount: lock1_amount,
        });
    }
    if lock2_amount != 0 {
        new_withdraw_infos.push(WithdrawInfo {
            unlock_epoch: current_epoch + 2,
            amount: lock2_amount,
        });
    }

    new_withdraw_infos
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    let withdraw_args: withdraw_reader::WithdrawArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = withdraw_args.metadata_type_id();

    let type_ids = get_type_ids(
        &metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    if metadata_type_id
        != get_script_hash(&type_ids.metadata_code_hash(), &type_ids.metadata_type_id())
    {
        return Err(Error::MisMatchMetadataTypeId);
    }

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
    // we may firstly need to check only 1 withdraw at cell exist in input and output
    let (in_amount, in_data) =
        get_withdraw_at_data_by_lock_hash(&withdraw_at_lock_hash, Source::Input)?;
    let (out_amount, out_data) =
        get_withdraw_at_data_by_lock_hash(&withdraw_at_lock_hash, Source::Output)?;

    let checkpoint_type_id = get_script_hash(
        &type_ids.checkpoint_code_hash(),
        &type_ids.checkpoint_type_id(),
    );
    let epoch = get_current_epoch(&checkpoint_type_id.to_vec())?;
    // get input normal at cell and output noram at cell, verify amount increased by unlock_amount.
    let input_total_amount = get_xudt_by_type_hash(&type_ids.xudt_type_hash(), Source::Input)?;
    let output_total_amount = get_xudt_by_type_hash(&type_ids.xudt_type_hash(), Source::Output)?;
    debug!(
        "epoch: {:?}, in_amount: {}, out_amount: {}, input_total_amount: {}, output_total_amount: {}",
        epoch, in_amount, out_amount,input_total_amount, output_total_amount
    );

    if withdraw_witness.is_none() {
        // ACP mode, someone unstake or undelgate
        debug!("acp mode");
        if out_amount <= in_amount {
            return Err(Error::OutLessThanIn);
        }
        let increased_amount = out_amount - in_amount;

        let in_data = in_data.lock().withdraw_infos();
        let mut unlock_amount: u128 = 0u128; // can be withdraw immediately
        let mut lock1_amount: u128 = 0u128; // can be withdraw in current epoch + 1
        let mut lock2_amount: u128 = increased_amount; // can be withdraw in current epoch + 2
        update_withdraw_amounts(
            epoch,
            in_data,
            &mut unlock_amount,
            &mut lock1_amount,
            &mut lock2_amount,
        )?;
        if in_amount != unlock_amount + lock1_amount + lock2_amount - increased_amount {
            return Err(Error::WithdrawTotalAmount);
        }
        let new_withdraw_infos =
            get_withdraw_infos(epoch, unlock_amount, lock1_amount, lock2_amount);

        let out_data = out_data.lock().withdraw_infos();
        let mut unlock_amount: u128 = 0u128; // can be withdraw immediately
        let mut lock1_amount: u128 = 0u128; // can be withdraw in current epoch + 1
        let mut lock2_amount: u128 = 0u128; // can be withdraw in current epoch + 2
        update_withdraw_amounts(
            epoch,
            out_data,
            &mut unlock_amount,
            &mut lock1_amount,
            &mut lock2_amount,
        )?;
        if out_amount != unlock_amount + lock1_amount + lock2_amount {
            return Err(Error::WithdrawTotalAmount);
        }
        let out_withdraw_infos =
            get_withdraw_infos(epoch, unlock_amount, lock1_amount, lock2_amount);

        if new_withdraw_infos != out_withdraw_infos {
            return Err(Error::WrongOutWithdraw);
        }

        if input_total_amount > output_total_amount {
            return Err(Error::WithdrawTotalAmount);
        }
    } else {
        // unlock mode,
        debug!("unlock mode");
        let in_data = in_data.lock().withdraw_infos();
        let mut unlock_amount: u128 = 0u128; // can be withdraw immediately
        let mut lock1_amount: u128 = 0u128; // can be withdraw in current epoch + 1
        let mut lock2_amount: u128 = 0u128; // can be withdraw in current epoch + 2
        update_withdraw_amounts(
            epoch,
            in_data,
            &mut unlock_amount,
            &mut lock1_amount,
            &mut lock2_amount,
        )?;
        debug!(
            "unlock_amount: {}, lock1_amount: {}, lock2_amount: {}",
            unlock_amount, lock1_amount, lock2_amount
        );
        if in_amount != unlock_amount + lock1_amount + lock2_amount {
            return Err(Error::WithdrawTotalAmount);
        }
        let new_withdraw_infos = get_withdraw_infos(epoch, 0, lock1_amount, lock2_amount);

        if in_amount - out_amount != unlock_amount {
            return Err(Error::WithdrawTotalAmount);
        }

        let out_data = out_data.lock().withdraw_infos();
        if out_data.len() > 2 {
            return Err(Error::WithdrawWrongRecordSize);
        }

        let mut lock1_amount: u128 = 0u128; // can be withdraw in current epoch + 1
        let mut lock2_amount: u128 = 0u128; // can be withdraw in current epoch + 2
                                            // unlock_amount is uesless here
        update_withdraw_amounts(
            epoch,
            out_data,
            &mut unlock_amount,
            &mut lock1_amount,
            &mut lock2_amount,
        )?;
        debug!(
            "out_amount: {}, lock1_amount: {}, lock2_amount: {}",
            out_amount, lock1_amount, lock2_amount
        );
        if out_amount != lock1_amount + lock2_amount {
            return Err(Error::WithdrawTotalAmount);
        }
        let out_withdraw_infos = get_withdraw_infos(epoch, 0, lock1_amount, lock2_amount);
        debug!(
            "new_withdraw_infos: {:?}, out_withdraw_infos: {:?}",
            new_withdraw_infos, out_withdraw_infos
        );
        if new_withdraw_infos != out_withdraw_infos {
            return Err(Error::WrongOutWithdraw);
        }

        if input_total_amount < output_total_amount {
            return Err(Error::WithdrawTotalAmount);
        }
    }

    Ok(())
}

fn update_withdraw_amounts(
    epoch: u64,
    data: WithdrawInfos,
    unlock_amount: &mut u128,
    lock1_amount: &mut u128,
    lock2_amount: &mut u128,
) -> Result<(), Error> {
    if data.len() > 3 {
        return Err(Error::WithdrawWrongRecordSize);
    }
    for i in 0..data.len() {
        let withdraw_info = data.get(i);
        let amount = bytes_to_u128(&withdraw_info.amount());
        if amount == 0 {
            return Err(Error::WithdrawZeroAmount);
        }
        if withdraw_info.unlock_epoch() <= epoch {
            *unlock_amount += bytes_to_u128(&withdraw_info.amount());
        } else if withdraw_info.unlock_epoch() == epoch + 1 {
            *lock1_amount += bytes_to_u128(&withdraw_info.amount());
        } else if withdraw_info.unlock_epoch() == epoch + 2 {
            *lock2_amount += bytes_to_u128(&withdraw_info.amount());
        } else {
            return Err(Error::WrongLockEpoch);
        }
    }
    Ok(())
}
