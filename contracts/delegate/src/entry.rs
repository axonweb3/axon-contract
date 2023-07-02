// Import from `core` instead of from `std` since we are in no-std mode
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
    // checkpoint,
    delegate_reader::{self},
    Cursor,
};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract delegate_args
    let delegate_args: delegate_reader::DelegateArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = delegate_args.metadata_type_id();
    let delegator_identity = delegate_args.delegator_addr();

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
    let witness_args = load_witness_args(0, Source::Input);
    match witness_args {
        Ok(witness) => {
            let mode = {
                let witness_lock = witness.lock().to_opt();
                if witness_lock.is_none() {
                    return Err(Error::WitnessLockError);
                }
                debug!(
                    "witness_lock data len:{:?}",
                    witness_lock.clone().unwrap().raw_data().len()
                );
                let value: delegate_reader::DelegateAtWitness =
                    Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
                // debug!("witness mode: {}", value.mode());
                value.mode()
            };
            debug!("delegate at mode: {}", mode);

            match mode {
                0 => {
                    // update delegate at cell
                    // extract delegate at cell lock hash
                    let delegate_at_lock_hash = { load_cell_lock_hash(0, Source::Input)? };
                    let checkpoint_script_hash = get_script_hash(
                        &type_ids.checkpoint_code_hash(),
                        &type_ids.checkpoint_type_id(),
                    );
                    debug!("checkpoint_script_hash: {:?}", checkpoint_script_hash);
                    update_delegate_at_cell(
                        &delegator_identity,
                        &delegate_at_lock_hash,
                        &checkpoint_script_hash.to_vec(),
                        &type_ids.xudt_type_hash(),
                    )?;
                }
                1 => {
                    // kicker update delegate smt cell
                    let delegate_smt_type_hash = get_script_hash(
                        &type_ids.delegate_smt_code_hash(),
                        &type_ids.delegate_smt_type_id(),
                    );
                    debug!("delegate_smt_type_hash: {:?}", delegate_smt_type_hash);
                    let checkpoint_script_hash = get_script_hash(
                        &type_ids.checkpoint_code_hash(),
                        &type_ids.checkpoint_type_id(),
                    );
                    debug!("checkpoint_script_hash: {:?}", checkpoint_script_hash);
                    update_delegate_smt(&delegate_smt_type_hash.to_vec())?;
                }
                _ => {
                    return Err(Error::UnknownMode);
                }
            }
        }
        Err(_) => {
            return Err(Error::UnknownMode);
        }
    };

    Ok(())
}

pub fn update_delegate_at_cell(
    delegator_identity: &Vec<u8>,
    delegate_at_lock_hash: &[u8; 32],
    checkpoint_type_id: &Vec<u8>,
    xudt_type_hash: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update delegate info in delegate at cell");
    // if !secp256k1::verify_signature(&delegator_identity) {
    //     return Err(Error::SignatureMismatch);
    // }

    check_xudt_type_hash(xudt_type_hash)?;

    let input_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Input)?;
    let output_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Output)?;
    debug!(
        "input_at_amount: {}, output_at_amount: {}",
        input_at_amount, output_at_amount
    );
    if input_at_amount != output_at_amount {
        return Err(Error::InputOutputAtAmountNotEqual);
    }

    let (input_amount, input_delegate_at_data) =
        get_delegate_at_data_by_lock_hash(&delegate_at_lock_hash, Source::Input)?;
    let (output_amount, output_delegate_at_data) =
        get_delegate_at_data_by_lock_hash(&delegate_at_lock_hash, Source::Output)?;
    if input_delegate_at_data.version() != output_delegate_at_data.version()
        || input_delegate_at_data.metadata_type_id() != output_delegate_at_data.metadata_type_id()
    {
        return Err(Error::UpdateDataError);
    }
    debug!(
        "input_amount: {}, output_amount: {}",
        input_amount, output_amount
    );

    let epoch = get_current_epoch(checkpoint_type_id)?;
    debug!("epoch: {}", epoch);
    let mut at_change = 0i128;
    let input_delegate_info_deltas = input_delegate_at_data.delegator_infos();
    let output_delegate_info_deltas = output_delegate_at_data.delegator_infos();
    for i in 0..output_delegate_info_deltas.len() {
        let output_delegate_info = output_delegate_info_deltas.get(i);
        let output_delegate = bytes_to_u128(&output_delegate_info.amount());
        let output_increase: bool = output_delegate_info.is_increase() == 1;
        let output_inaugutation_epoch = output_delegate_info.inauguration_epoch();
        let staker = output_delegate_info.staker();
        if output_inaugutation_epoch != epoch + 2 {
            return Err(Error::BadInaugurationEpoch);
        }

        let mut input_delegate = 0u128;
        let mut input_increase = true;
        let mut first_delegate = true;
        for i in 0..input_delegate_info_deltas.len() {
            let input_delegate_info = input_delegate_info_deltas.get(i);
            if input_delegate_info.staker().as_slice().to_vec() == staker {
                first_delegate = false;
                input_delegate = bytes_to_u128(&input_delegate_info.amount());
                input_increase = input_delegate_info.is_increase() == 1;
                break;
            }
        }

        if first_delegate {
            // no delegate info before
            if !output_increase {
                return Err(Error::FirstRedeemError);
            }
            at_change -= output_delegate as i128; // decrease by output_delegate
        } else {
            // update existing delegate info
            if input_increase {
                if output_increase {
                    at_change += output_delegate as i128 - input_delegate as i128;
                } else {
                    // delegate decrease by input_delegate, withdraw all of previous tokens
                    at_change -= input_delegate as i128;
                    // we should check output_delegate is less than amount stored in smt?
                }
            } else {
                if output_increase {
                    at_change += output_delegate as i128;
                } else {
                    // we should check output_delegate is less than amount stored in smt?
                }
            }
        }
        if input_amount as i128 + at_change != output_amount as i128 {
            return Err(Error::BadDelegateChange);
        }
    }

    Ok(())
}

fn update_delegate_smt(delegate_smt_type_id: &Vec<u8>) -> Result<(), Error> {
    debug!("delegate at cell update delegate smt root mode");
    // delegator AT cell
    // only need to verify input and output both contain the delegate SMT cell of the Chain
    let input_smt_cell = get_cell_count_by_type_hash(&delegate_smt_type_id, Source::Input);
    if input_smt_cell != 1 {
        return Err(Error::BadInputStakeSmtCellCount);
    }
    let output_smt_cell = get_cell_count_by_type_hash(&delegate_smt_type_id, Source::Output);
    if output_smt_cell != 1 {
        return Err(Error::BadOutputStakeSmtCellCount);
    }
    Ok(())
}
