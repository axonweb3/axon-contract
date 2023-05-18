// Import from `core` instead of from `std` since we are in no-std mode
use alloc::{collections::BTreeSet, vec::Vec};
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{
    delegate_reader::{self, DelegateInfoDelta},
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

    let metadata_type_ids = get_type_ids(
        &metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    if metadata_type_id != metadata_type_ids.metadata_type_id() {
        return Err(Error::MisMatchMetadataTypeId);
    }

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let value = witness.input_type().to_opt();
            if value.is_none() || value.as_ref().unwrap().len() != 1 {
                return Err(Error::BadWitnessInputType);
            }

            let input_type = *value.unwrap().raw_data().to_vec().first().unwrap();
            if input_type == 0 {
                // update delegate at cell
                // extract delegate at cell lock hash
                let delegate_at_lock_hash = { load_cell_lock_hash(0, Source::GroupInput)? };
                update_delegate_at_cell(
                    &delegator_identity.unwrap(),
                    &delegate_at_lock_hash,
                    &metadata_type_ids.checkpoint_type_id(),
                    &metadata_type_ids.xudt_type_id(),
                )?;
            } else if input_type == 1 {
                // kicker update stake smt cell
                // get old_stakes & proof from Stake AT cells' witness of input
                let delegate_smt_update_infos = {
                    let witness_lock = witness.lock().to_opt();
                    if witness_lock.is_none() {
                        return Err(Error::WitnessLockError);
                    }
                    let value: delegate_reader::DelegateSmtUpdateInfo =
                        Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
                    value
                };
                let metadata_type_id: [u8; 32] = metadata_type_id.as_slice().try_into().unwrap();
                update_stake_smt(
                    &delegator_identity,
                    &delegate_smt_update_infos,
                    &metadata_type_ids.checkpoint_type_id(),
                    &metadata_type_ids.xudt_type_id(),
                    &metadata_type_ids.delegate_smt_type_id(),
                    &metadata_type_id,
                )?;
            } else if input_type == 2 {
                // election
                elect_validators(&metadata_type_id.as_slice().try_into().unwrap())?;
            } else {
                return Err(Error::UnknownMode);
            }
        }
        Err(_) => {
            return Err(Error::UnknownMode);
        }
    };

    Ok(())
}

fn check_xudt_type_id(xudt_type_id: &Vec<u8>) -> Result<(), Error> {
    // extract AT type_id from type_script
    let type_id = {
        let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
        if type_hash.is_none() {
            return Err(Error::TypeScriptEmpty);
        }
        type_hash.unwrap()
    };

    if type_id.to_vec() != *xudt_type_id {
        return Err(Error::MismatchXudtTypeId);
    }

    Ok(())
}

pub fn update_delegate_at_cell(
    delegator_identity: &Vec<u8>,
    delegate_at_lock_hash: &[u8; 32],
    checkpoint_type_id: &Vec<u8>,
    xudt_type_id: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update delegate info in delegate at cell");
    if !secp256k1::verify_signature(&delegator_identity) {
        return Err(Error::SignatureMismatch);
    }

    check_xudt_type_id(xudt_type_id)?;

    let input_at_amount = get_xudt_by_type_hash(xudt_type_id, Source::Input)?;
    let output_at_amount = get_xudt_by_type_hash(xudt_type_id, Source::Output)?;
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

    let epoch = get_current_epoch(checkpoint_type_id)?;

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

fn update_delegate_info(
    _staker: [u8; 20],
    delegator: [u8; 20],
    delegate_at_lock_hash: &[u8; 32],
    delegate_info_delta: &DelegateInfoDelta,
    delegate_infos_set: &mut BTreeSet<DelegateInfoObject>,
) -> Result<(), Error> {
    // get this delegator's old delegate amount in smt tree from delegate_infos_set
    let delegate_info = delegate_infos_set
        .iter()
        .find(|delegate_info| delegator == delegate_info.addr);
    let mut delegate_info_clone: Option<DelegateInfoObject> = None;
    let mut old_delegate: u128;
    if let Some(delegate_info) = delegate_info {
        old_delegate = delegate_info.amount;
        delegate_info_clone = Some(DelegateInfoObject {
            addr: delegate_info.addr,
            amount: delegate_info.amount,
        })
    } else {
        // the delegator has not changed delegate in this epoch yet,
        // should be delegateor's at amount, get from delegate at cell, so we need total delegate amount for delegator's every staker
        (old_delegate, _) =
            get_delegate_at_data_by_lock_hash(&delegate_at_lock_hash, Source::Input)?;
    }

    // the staker's info should be updated, so we deleted it from stake_infos_set first, we will insert it in the future
    if delegate_info_clone.is_some() {
        delegate_infos_set.remove(&delegate_info_clone.unwrap());
    }

    let input_delegate = bytes_to_u128(&delegate_info_delta.amount());
    let input_increase = delegate_info_delta.is_increase() == 1;
    // calculate the stake of output
    let mut redeem_amount = 0u128;
    if input_increase {
        old_delegate += input_delegate;
    } else {
        if input_delegate > old_delegate {
            redeem_amount = old_delegate;
            old_delegate = 0;
        } else {
            redeem_amount = input_delegate;
            old_delegate -= input_delegate;
        }
    }

    let delegate_info_obj = DelegateInfoObject {
        addr: delegator,
        amount: old_delegate,
    };
    delegate_infos_set.insert(delegate_info_obj);

    // get input & output withdraw AT cell, we need to update this after withdraw script's finish
    if redeem_amount > 0 {
        let input_withdraw_amount = 10u128; // get from delegator input withdraw at cell
        let output_withdraw_amount = 10u128; // get from delegator input withdraw at cell
        // if output_withdraw_amount - input_withdraw_amount != redeem_amount {
        //     return Err(Error::BadRedeem);
        // }
        debug!("redeem_amount: {}", redeem_amount);
    }

    Ok(())
}

fn verify_delegator_seletion(
    delegate_infos_set: &BTreeSet<DelegateInfoObject>,
    new_epoch_root: &[u8; 32],
    new_epoch_proof: &Vec<u8>,
    epoch: u64,
    _metadata_type_id: &[u8; 32],
) -> Result<(), Error> {
    // sort delegator by amount
    let delegator_limit = 10u16; // should get from staker's delegate cell
    let iter = delegate_infos_set.iter();
    let mut top = iter.take(3 * delegator_limit as usize);
    let mut new_delegate_infos_set = BTreeSet::new();
    while let Some(elem) = top.next() {
        new_delegate_infos_set.insert((*elem).clone());
    }

    // get proof of new_stakes from Stake AT cells' witness of input,
    // verify delete_stakes is default
    // verify the new stake infos is equal to on-chain calculation
    verify_2layer_smt_delegate(
        &new_delegate_infos_set,
        epoch,
        new_epoch_proof,
        new_epoch_root,
    )?;

    Ok(())
}

fn update_stake_smt(
    delegator_identity: &Option<Vec<u8>>,
    delegate_smt_update_infos: &delegate_reader::DelegateSmtUpdateInfo,
    checkpoint_type_id: &Vec<u8>,
    xudt_type_id: &Vec<u8>,
    delegate_smt_type_id: &Vec<u8>,
    metadata_type_id: &[u8; 32],
) -> Result<(), Error> {
    debug!("update delegate smt root mode");
    if delegator_identity.is_none() {
        // this is delegate smt cell
        let type_id = {
            let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
            if type_hash.is_none() {
                return Err(Error::TypeScriptEmpty);
            }
            type_hash.unwrap()
        };
        let old_delegate_smt_data = get_delegate_smt_data(&type_id, Source::Input)?;
        let new_delegate_smt_data = get_delegate_smt_data(&type_id, Source::Output)?;
        if old_delegate_smt_data.version() != new_delegate_smt_data.version()
            || old_delegate_smt_data.metadata_type_id() != new_delegate_smt_data.metadata_type_id()
        {
            return Err(Error::UpdateDataError);
        }

        // construct old stake smt root & verify
        let epoch = get_current_epoch(&checkpoint_type_id)?;
        let stake_group_infos = delegate_smt_update_infos.all_stake_group_infos();
        for i in 0..stake_group_infos.len() {
            // verify old delegate info
            let stake_group_info = stake_group_infos.get(i);
            let staker = stake_group_info.staker();
            let delegate_infos = stake_group_info.delegate_infos();
            let mut delegate_infos_set = BTreeSet::new();
            for i in 0..delegate_infos.len() {
                let delegate_info = delegate_infos.get(i);
                let delegate_info_obj = DelegateInfoObject {
                    addr: delegate_info
                        .delegator_addr()
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    amount: bytes_to_u128(&delegate_info.amount()),
                };
                delegate_infos_set.insert(delegate_info_obj);
            }
            let old_epoch_proof = stake_group_info.delegate_old_epoch_proof();
            let old_epoch_root = get_delegate_smt_root(
                delegate_smt_type_id.as_slice().try_into().unwrap(),
                staker.as_slice().try_into().unwrap(),
                Source::GroupInput,
            )?;
            verify_2layer_smt_delegate(
                &delegate_infos_set,
                epoch,
                &old_epoch_proof,
                &old_epoch_root,
            )?;

            // update old delegate info to new delegate info based on input delegate at cells
            let delegate_at_type_id: [u8; 32] = xudt_type_id.as_slice().try_into().unwrap();
            // get this staker's delegate update infos
            let update_infos =
                get_delegate_update_infos(&staker, &delegate_at_type_id, Source::GroupInput)?;
            // update old delegate infos to new delegate infos
            for (delegator_addr, delegate_at_lock_hash, delegate_info_delta) in update_infos {
                let inauguration_epoch = delegate_info_delta.inauguration_epoch();
                if inauguration_epoch < epoch + 2 {
                    return Err(Error::StaleDelegateInfo);
                }

                // after updated to smt cell, the output delegate should be reset
                let output_delegate_info_delta =
                    get_delegate_delta(&staker, &delegate_at_lock_hash, Source::GroupOutput)?;
                let output_delegate = bytes_to_u128(&output_delegate_info_delta.amount());
                let output_increase: bool = output_delegate_info_delta.is_increase() == 1;
                let output_inaugutation_epoch = output_delegate_info_delta.inauguration_epoch();

                if output_delegate != 0 || !output_increase || output_inaugutation_epoch != 0 {
                    return Err(Error::IllegalDefaultDelegateInfo);
                }

                // get the delegator's new delegate info for this staker
                update_delegate_info(
                    staker.as_slice().try_into().unwrap(),
                    delegator_addr,
                    &delegate_at_lock_hash,
                    &delegate_info_delta,
                    &mut delegate_infos_set,
                )?;
            }

            // get proof of new_delegates from witness, verify delete_stakes is zero
            let new_proof = stake_group_info.delegate_new_epoch_proof();
            let new_epoch_root = get_delegate_smt_root(
                delegate_smt_type_id.as_slice().try_into().unwrap(),
                staker.as_slice().try_into().unwrap(),
                Source::Output,
            )?;
            verify_delegator_seletion(
                &delegate_infos_set,
                &new_epoch_root,
                &new_proof,
                epoch,
                metadata_type_id,
            )?;
        }
    } else {
        // staker AT cell
        // only need to verify input and output both contain the Stake SMT cell of the Chain
        let input_smt_cell = get_cell_count(&delegate_smt_type_id, Source::Input);
        if input_smt_cell != 1 {
            return Err(Error::BadInputStakeSmtCellCount);
        }
        let output_smt_cell = get_cell_count(&delegate_smt_type_id, Source::Output);
        if output_smt_cell != 1 {
            return Err(Error::BadOutputStakeSmtCellCount);
        }
    }
    Ok(())
}

fn elect_validators(metadata_type_id: &[u8; 32]) -> Result<(), Error> {
    let input_metadata_cell_cnt = get_cell_count(&metadata_type_id.to_vec(), Source::Input);
    if input_metadata_cell_cnt != 1 {
        return Err(Error::BadInputMetadataCellCount);
    }
    let output_metadata_cell_cnt = get_cell_count(&metadata_type_id.to_vec(), Source::Output);
    if output_metadata_cell_cnt != 1 {
        return Err(Error::BadOutputMetadataCellCount);
    }
    Ok(())
}
