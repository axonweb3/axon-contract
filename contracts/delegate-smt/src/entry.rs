// Import from `core` instead of from `std` since we are in no-std mode
use alloc::{collections::BTreeSet, vec::Vec};
use ckb_type_id::{load_type_id_from_script_args, validate_type_id};
use core::result::Result;
use sparse_merkle_tree::{CompiledMerkleProof, H256};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::Entity,
    debug,
    high_level::{load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{
    // checkpoint,
    delegate_reader::{self, DelegateInfoDelta},
    Cursor,
};
use util::{
    error::Error,
    helper::*,
    smt::{u64_to_h256, verify_2layer_smt, LockInfo},
};

use crate::types::WithdrawAmountMap;

pub fn main() -> Result<(), Error> {
    debug!("start delegate smt type script");
    // check type id is unique
    let type_id = load_type_id_from_script_args(0)?;
    debug!("type_id: {:?}", type_id);
    validate_type_id(type_id)?;

    let script = load_script()?;
    if !find_script_input(&script) {
        debug!("delegate smt cell creation");
        return Ok(());
    }

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let (mode, delegate_smt_update_infos) = {
                let witness_input_type = witness.input_type().to_opt();
                if witness_input_type.is_none() {
                    return Err(Error::WitnessInputTypeError);
                }
                let value: delegate_reader::DelegateSmtWitness =
                    Cursor::from(witness_input_type.unwrap().raw_data().to_vec()).into();
                (value.mode(), value.update_info())
            };
            debug!("delegate smt mode:{}", mode);

            let delegate_smt_type_id = {
                let delegate_smt_type_id = load_cell_type_hash(0, Source::GroupInput)?;
                if delegate_smt_type_id.is_none() {
                    return Err(Error::TypeScriptEmpty);
                }
                delegate_smt_type_id.unwrap()
            };
            debug!("delegate_smt_type_id: {:?}", delegate_smt_type_id);
            let delegate_smt_data = get_delegate_smt_data(&delegate_smt_type_id, Source::Input)?;
            let metadata_type_id: [u8; 32] = delegate_smt_data
                .metadata_type_id()
                .as_slice()
                .try_into()
                .unwrap();
            debug!("metadata_type_id: {:?}", metadata_type_id);

            match mode {
                0 => {
                    // kicker update delegate smt cell
                    let type_ids = get_type_ids(
                        &metadata_type_id.as_slice().try_into().unwrap(),
                        Source::CellDep,
                    )?;

                    let delegate_smt_type_hash = get_script_hash(
                        &type_ids.delegate_smt_code_hash(),
                        &type_ids.delegate_smt_type_id(),
                    );
                    if delegate_smt_type_hash != delegate_smt_type_id {
                        return Err(Error::DelegateSmtTypeIdMismatch);
                    }

                    debug!("delegate_smt_type_hash: {:?}", delegate_smt_type_hash);
                    let checkpoint_script_hash = get_script_hash(
                        &type_ids.checkpoint_code_hash(),
                        &type_ids.checkpoint_type_id(),
                    );
                    debug!("checkpoint_script_hash: {:?}", checkpoint_script_hash);
                    update_delegate_smt(
                        &delegate_smt_update_infos,
                        &checkpoint_script_hash.to_vec(),
                        &type_ids.xudt_type_hash(),
                        &metadata_type_id,
                        &type_ids.stake_at_code_hash(),
                        &type_ids.delegate_at_code_hash(),
                        &type_ids.withdraw_code_hash(),
                    )?;
                }
                1 => {
                    // election
                    elect_validators(&metadata_type_id.as_slice().try_into().unwrap())?;
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

fn update_delegate_info(
    delegator: [u8; 20],
    delegate_info_delta: &DelegateInfoDelta,
    delegate_infos_set: &mut BTreeSet<LockInfo>,
    delegate_withdraw_infos: &mut WithdrawAmountMap,
) -> Result<(), Error> {
    // get this delegator's old delegate amount in smt tree from delegate_infos_set
    let delegate_info = delegate_infos_set
        .iter()
        .find(|delegate_info| delegator == delegate_info.addr);
    let mut delegate_info_clone: Option<LockInfo> = None;
    let mut old_delegate = 0u128;
    if let Some(delegate_info) = delegate_info {
        old_delegate = delegate_info.amount;
        delegate_info_clone = Some(LockInfo {
            addr: delegate_info.addr,
            amount: delegate_info.amount,
        })
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

    let delegate_info_obj = LockInfo {
        addr: delegator,
        amount: old_delegate,
    };
    delegate_infos_set.insert(delegate_info_obj);
    debug!("delegate_info_obj: {:?}", delegate_info_obj);

    if redeem_amount > 0 {
        delegate_withdraw_infos.insert(delegator, redeem_amount);
    }

    Ok(())
}

fn verify_delegator_selection(
    delegate_infos_set: &BTreeSet<LockInfo>,
    new_epoch_root: [u8; 32],
    new_epoch_proof: Vec<u8>,
    epoch: u64,
    max_delegator_size: u32,
) -> Result<(), Error> {
    // sort delegator by amount
    let iter = delegate_infos_set.iter();
    let mut top = iter.take(3 * max_delegator_size as usize);
    let mut new_delegate_infos_set = BTreeSet::new();
    while let Some(elem) = top.next() {
        new_delegate_infos_set.insert((*elem).clone());
    }

    let new_epoch_root: H256 = new_epoch_root.into();
    let new_epoch_proof = CompiledMerkleProof(new_epoch_proof);
    let result = verify_2layer_smt(
        &new_delegate_infos_set,
        u64_to_h256(epoch + 2),
        new_epoch_root,
        new_epoch_proof,
    )?;
    debug!(
        "verify_2layer_smt new delegate_infos_set result: {}",
        result
    );
    if !result {
        return Err(Error::DelegateSmtVerifySelectionError);
    }

    Ok(())
}

fn is_output_lock_info_reset(output_delegate_info_delta: &DelegateInfoDelta) -> Result<(), Error> {
    let output_delegate = bytes_to_u128(&output_delegate_info_delta.amount());
    let output_increase = output_delegate_info_delta.is_increase() == 0;
    let output_inaugutation_epoch = output_delegate_info_delta.inauguration_epoch();

    if output_delegate != 0 || !output_increase || output_inaugutation_epoch != 0 {
        return Err(Error::IllegalDefaultDelegateInfo);
    }

    Ok(())
}

fn update_delegate_smt(
    delegate_smt_update_infos: &delegate_reader::DelegateSmtUpdateInfo,
    checkpoint_type_id: &Vec<u8>,
    xudt_type_hash: &Vec<u8>,
    metadata_type_id: &[u8; 32],
    stake_at_code_hash: &Vec<u8>,
    delegate_at_code_hash: &Vec<u8>,
    withdraw_code_hash: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update delegate smt root mode");
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

    // construct old delegate smt root & verify
    let epoch = get_current_epoch(&checkpoint_type_id)?;
    debug!("get_current_epoch: {}", epoch);
    let stake_group_infos = delegate_smt_update_infos.all_stake_group_infos();
    let mut delegate_withdraw_infos = WithdrawAmountMap::new();
    for i in 0..stake_group_infos.len() {
        // verify old delegate info
        let stake_group_info = stake_group_infos.get(i);
        let staker = stake_group_info.staker();
        let delegate_infos = stake_group_info.delegate_infos();
        let mut delegate_infos_set = BTreeSet::new();
        for i in 0..delegate_infos.len() {
            let delegate_info = delegate_infos.get(i);
            let delegate_info_obj = LockInfo {
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
        let old_epoch_proof: CompiledMerkleProof = CompiledMerkleProof(old_epoch_proof);
        let old_epoch_root = get_delegate_smt_root_from_cell_data(
            staker.as_slice().try_into().unwrap(),
            &old_delegate_smt_data,
        )?;
        let old_epoch_root: H256 = old_epoch_root.into();
        let result = verify_2layer_smt(
            &delegate_infos_set,
            u64_to_h256(epoch + 2),
            old_epoch_root,
            old_epoch_proof,
        )?;
        debug!(
            "verify_2layer_smt old delegate_infos_set result: {}",
            result
        );

        // update old delegate info to new delegate info based on input delegate at cells
        let xudt_type_hash: [u8; 32] = xudt_type_hash.as_slice().try_into().unwrap();
        // debug!("xudt_type_hash: {:?}", xudt_type_hash);
        // get this staker's delegate update infos
        let update_infos = get_delegate_update_infos(
            &staker,
            &xudt_type_hash,
            delegate_at_code_hash,
            Source::Input,
        )?;
        // update old delegate infos to new delegate infos
        for (delegator_addr, delegate_at_lock_hash, delegate_info_delta) in update_infos {
            let inauguration_epoch = delegate_info_delta.inauguration_epoch();
            if inauguration_epoch < epoch + 2 {
                return Err(Error::StaleDelegateInfo);
            }

            // after updated to smt cell, the output delegate should be reset
            let output_delegate_info_delta =
                get_delegate_delta(&staker, &delegate_at_lock_hash, Source::Output)?;
            is_output_lock_info_reset(&output_delegate_info_delta)?;

            // get the delegator's new delegate info for this staker
            update_delegate_info(
                delegator_addr,
                &delegate_info_delta,
                &mut delegate_infos_set,
                &mut delegate_withdraw_infos,
            )?;
        }

        // get proof of new_delegates from witness, verify delete_stakes is zero
        let new_proof = stake_group_info.delegate_new_epoch_proof();
        let new_epoch_root = get_delegate_smt_root_from_cell_data(
            staker.as_slice().try_into().unwrap(),
            &new_delegate_smt_data,
        )?;
        let max_delegator_size = get_delegator_size(&staker, metadata_type_id, stake_at_code_hash)?;
        debug!("max_delegator_size: {}", max_delegator_size);

        verify_delegator_selection(
            &delegate_infos_set,
            new_epoch_root,
            new_proof,
            epoch,
            max_delegator_size,
        )?;
    }

    // get input & output withdraw AT cell, we need to update this after withdraw script's finish
    verify_withdraw_amount(
        delegate_withdraw_infos,
        metadata_type_id,
        withdraw_code_hash,
    )?;

    Ok(())
}

fn get_delegator_size(
    staker: &Vec<u8>,
    metadata_type_id: &[u8; 32],
    stake_at_code_hash: &Vec<u8>,
) -> Result<u32, Error> {
    let delegate_requirement =
        get_delegate_requirement(staker, metadata_type_id, stake_at_code_hash)?;

    Ok(bytes_to_u32(
        &delegate_requirement
            .max_delegator_size()
            .as_slice()
            .to_vec(),
    ))
}

fn verify_withdraw_amount(
    delegate_withdraw_infos: WithdrawAmountMap,
    metadata_type_id: &[u8; 32],
    withdraw_code_hash: &Vec<u8>,
) -> Result<(), Error> {
    for addr in delegate_withdraw_infos.map.keys() {
        let undelegate_amount = delegate_withdraw_infos.map.get(addr).unwrap();
        debug!(
            "verify_withdraw_amount addr:{:?}, amount:{}",
            addr, undelegate_amount
        );
        let withdraw_lock_hash =
            calc_withdrawal_lock_hash(withdraw_code_hash, addr, metadata_type_id);
        let (input_amount, input_info) =
            get_withdraw_at_data_by_lock_hash(&withdraw_lock_hash, Source::Input)?;
        let (output_amount, output_info) =
            get_withdraw_at_data_by_lock_hash(&withdraw_lock_hash, Source::Output)?;
        debug!(
            "undelegate_amount:{}, input_amount: {}, output_amount: {}",
            undelegate_amount, input_amount, output_amount
        );
        if input_amount + undelegate_amount != output_amount {
            return Err(Error::BadUnstake);
        }
        if input_info.lock().version() != output_info.lock().version() {
            return Err(Error::WithdrawUpdateDataError);
        }
    }

    Ok(())
}

fn elect_validators(metadata_type_id: &[u8; 32]) -> Result<(), Error> {
    let input_metadata_cell_cnt =
        get_cell_count_by_type_hash(&metadata_type_id.to_vec(), Source::Input);
    if input_metadata_cell_cnt != 1 {
        return Err(Error::BadInputMetadataCellCount);
    }
    let output_metadata_cell_cnt =
        get_cell_count_by_type_hash(&metadata_type_id.to_vec(), Source::Output);
    if output_metadata_cell_cnt != 1 {
        return Err(Error::BadOutputMetadataCellCount);
    }
    Ok(())
}
