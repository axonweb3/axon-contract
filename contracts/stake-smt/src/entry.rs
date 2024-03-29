// Import from `core` instead of from `std` since we are in no-std mode
use alloc::{collections::BTreeSet, vec::Vec};
use axon_types::metadata_reader;
use axon_types::stake_reader::StakeAtCellLockData;
use axon_types::stake_reader::StakeInfoDelta;
use axon_types::stake_reader::StakeInfos;
use axon_types::stake_reader::StakeSmtCellData;
use axon_types::stake_reader::StakeSmtUpdateInfo;
use ckb_type_id::load_type_id_from_script_args;
use ckb_type_id::validate_type_id;
use core::result::Result;
use sparse_merkle_tree::CompiledMerkleProof;
use sparse_merkle_tree::H256;
use util::smt::u64_to_h256;
use util::smt::verify_2layer_smt;
use util::smt::LockInfo;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{stake_reader, Cursor};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    debug!("start stake smt type script");
    // check type id is unique
    let type_id = load_type_id_from_script_args(0)?;
    debug!("type_id: {:?}", type_id);
    validate_type_id(type_id)?;

    let script = load_script()?;
    let stake_smt_type_id = calc_script_hash(&script).to_vec();
    debug!("stake_smt_type_id = {:?}", stake_smt_type_id);
    let input_stake_smt_count = get_cell_count_by_type_hash(&stake_smt_type_id, Source::Input);
    if input_stake_smt_count == 0 {
        debug!("stake smt cell creation");
        return Ok(());
    }

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let (mode, stake_smt_update_infos) = {
                let witness_input_type = witness.input_type().to_opt();
                if witness_input_type.is_none() {
                    return Err(Error::WitnessInputTypeError);
                }
                let value: stake_reader::StakeSmtWitness =
                    Cursor::from(witness_input_type.unwrap().raw_data().to_vec()).into();
                (value.mode(), value.update_info())
            };
            debug!("stake smt mode:{}", mode);

            let source = if mode == 1 {
                Source::Input
            } else {
                Source::CellDep
            };

            // debug!("load_cell_type_hash");
            let stake_smt_type_id = {
                let stake_smt_type_id = load_cell_type_hash(0, Source::GroupInput)?;
                if stake_smt_type_id.is_none() {
                    return Err(Error::TypeScriptEmpty);
                }
                stake_smt_type_id.unwrap()
            };
            // debug!("stake_smt_type_id: {:?}", stake_smt_type_id);
            let stake_smt_data = get_stake_smt_data(&stake_smt_type_id, Source::Input)?;
            // debug!("get metadata_type_id from stake_smt_data");
            let metadata_type_id = stake_smt_data.metadata_type_id();
            // debug!("metadata_type_id to [u8; 32]");
            let metadata_type_id: [u8; 32] = metadata_type_id.as_slice().try_into().unwrap();

            debug!("get type_ids");
            let type_ids = get_type_ids(&metadata_type_id, source)?;

            match mode {
                0 => {
                    // kicker update stake smt cell
                    update_stake_smt(&stake_smt_update_infos, &type_ids, &stake_smt_type_id)?;
                }
                1 => {
                    elect_validators(&metadata_type_id)?;
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

fn verify_withdraw_amount(
    unstake_amount: u128,
    metadata_type_id: &[u8; 32],
    addr: &[u8; 20],
    withdraw_code_hash: &Vec<u8>,
) -> Result<(), Error> {
    if unstake_amount > 0 {
        let withdraw_lock_hash =
            calc_withdrawal_lock_hash(withdraw_code_hash, addr, metadata_type_id);
        let (input_amount, input_info) =
            get_withdraw_at_data_by_lock_hash(&withdraw_lock_hash, Source::Input)?;
        let (output_amount, output_info) =
            get_withdraw_at_data_by_lock_hash(&withdraw_lock_hash, Source::Output)?;
        debug!(
            "unstake_amount:{}, input_amount: {}, output_amount: {}",
            unstake_amount, input_amount, output_amount
        );
        if input_amount + unstake_amount != output_amount {
            return Err(Error::BadUnstake);
        }
        if input_info.lock().version() != output_info.lock().version() {
            return Err(Error::WithdrawUpdateDataError);
        }
    }

    Ok(())
}

// the old_stake_info_set is the stake infos of old epoch， the stake_info_delta is the stake change of this epoch
// if the staker's stake is changed in this epoch, we need to update the stake_infos_set
// if not, we need to add the staker's stake info to stake_infos_set
fn update_stake_info(
    addr: [u8; 20],
    metadata_type_id: &[u8; 32],
    withdraw_code_hash: &Vec<u8>,
    stake_info_delta: &StakeInfoDelta,
    // stake_infos_set: &mut BTreeSet<LockInfo>,
) -> Result<(), Error> {
    let input_increase = stake_info_delta.is_increase() == 1;
    // calculate the stake of output
    let mut unstake_amount = 0u128;
    if !input_increase {
        unstake_amount = bytes_to_u128(&stake_info_delta.amount());
    }

    // get input & output withdraw AT cell, we need to update this after withdraw script's finish
    debug!(
        "verify_withdraw_amount withdraw_code_hash: {:?}, addr: {:?}, metadata_type_id: {:?}, unstake_amount: {}",
        withdraw_code_hash, addr, metadata_type_id, unstake_amount
    );
    verify_withdraw_amount(unstake_amount, metadata_type_id, &addr, withdraw_code_hash)?;

    Ok(())
}

fn verify_old_stake_infos(
    epoch: u64,
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    old_stake_smt_data: StakeSmtCellData,
    stake_infos_set: &BTreeSet<LockInfo>,
) -> Result<(), Error> {
    let epoch_root: [u8; 32] = old_stake_smt_data.smt_root().as_slice().try_into().unwrap(); // get from input smt cell
    let epoch_root: H256 = epoch_root.into();
    let epoch_proof = CompiledMerkleProof(stake_smt_update_infos.old_epoch_proof());
    debug!(
        "epoch_root: {:?}, epoch_proof: {:?}",
        epoch_root, epoch_proof
    );
    let result = verify_2layer_smt(
        &stake_infos_set,
        u64_to_h256(epoch),
        epoch_root,
        epoch_proof,
    )?;
    if !result {
        return Err(Error::StakeSmtVerifyOldError);
    }
    debug!("verify_old_stake_infos verify_2layer_smt result:{}", result);
    Ok(())
}

fn get_selected_unselected_staker(
    new_stake_info_set: &BTreeSet<LockInfo>,
    metadata_type_id: &[u8; 32],
) -> Result<(BTreeSet<LockInfo>, BTreeSet<LockInfo>), Error> {
    // sort stakes by amount
    let quorum_size = get_quorum_size(
        metadata_type_id,
        util::stake::EpochClass::CURRENT,
        Source::CellDep,
    )?;
    let mut top_3quorum = new_stake_info_set.iter().take(3 * quorum_size as usize);
    let mut select_stake_info_set = BTreeSet::new();
    while let Some(elem) = top_3quorum.next() {
        select_stake_info_set.insert((*elem).clone());
        debug!("select_stake_infos_set : {:x?}", *elem);
    }

    let mut delete_stake_info_set = BTreeSet::<LockInfo>::new();
    if new_stake_info_set.len() > select_stake_info_set.len() {
        let mut iter = new_stake_info_set.iter();
        let deleted_size = new_stake_info_set.len() - select_stake_info_set.len();
        for _ in 0..deleted_size {
            if let Some(elem) = iter.next_back() {
                delete_stake_info_set.insert(elem.clone());
                debug!("deleted_stake_infos : {:x?}", *elem);
            }
        }
    }

    debug!(
        "stake_infos_set len: {}, select_stake_infos_set.len():{}, deleted stake infos size: {}, quorum: {}",
        new_stake_info_set.len(),
        select_stake_info_set.len(),
        delete_stake_info_set.len(),
        quorum_size
    );

    Ok((select_stake_info_set, delete_stake_info_set))
}

fn verify_staker_selection(
    select_stake_info_set: &BTreeSet<LockInfo>,
    new_stake_smt_data: &StakeSmtCellData,
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    epoch: u64,
) -> Result<(), Error> {
    // get proof of new_stakes from Stake AT cells' witness of input,
    // verify delete_stakes is default
    // verify the new stake infos is equal to on-chain calculation
    let new_epoch_root: [u8; 32] = new_stake_smt_data.smt_root().as_slice().try_into().unwrap(); // get from output smt cell
    let new_epoch_root: H256 = new_epoch_root.into();
    let new_epoch_proof = stake_smt_update_infos.new_epoch_proof();
    let new_epoch_proof = CompiledMerkleProof(new_epoch_proof);
    let result = verify_2layer_smt(
        &select_stake_info_set,
        u64_to_h256(epoch),
        new_epoch_root,
        new_epoch_proof,
    )?;
    debug!(
        "verify_staker_selection epoch: {}, verify_2layer_smt result:{}",
        epoch, result
    );
    if !result {
        return Err(Error::StakeSmtVerifySelectionError);
    }

    Ok(())
}

pub fn transform_to_set(stake_infos: &StakeInfos) -> BTreeSet<LockInfo> {
    let mut stake_infos_set = BTreeSet::new();
    for i in 0..stake_infos.len() {
        let stake_info = &stake_infos.get(i);
        let stake_info_obj = LockInfo {
            addr: stake_info.addr().try_into().unwrap(),
            amount: bytes_to_u128(&stake_info.amount()),
        };
        stake_infos_set.insert(stake_info_obj);
    }
    stake_infos_set
}

fn is_output_lock_info_reset(output_stake_at_data: &StakeAtCellLockData) -> Result<(), Error> {
    let output_stake_info = output_stake_at_data.delta();
    let output_stake = bytes_to_u128(&output_stake_info.amount());
    let output_decrease: bool = output_stake_info.is_increase() == 0;
    let output_inaugutation_epoch = output_stake_info.inauguration_epoch();

    if output_stake != 0 || !output_decrease || output_inaugutation_epoch != 0 {
        return Err(Error::IllegalDefaultStakeInfo);
    }

    Ok(())
}

fn update_stake_smt(
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    type_ids: &metadata_reader::TypeIds,
    cell_type_id: &[u8; 32],
) -> Result<(), Error> {
    debug!("smt cell update stake smt root mode");
    let xudt_type_hash = type_ids.xudt_type_hash();
    let stake_smt_type_id = get_script_hash(
        &type_ids.stake_smt_code_hash(),
        &type_ids.stake_smt_type_id(),
    );
    // this is stake smt cell
    let checkpoint_script_hash = get_script_hash(
        &type_ids.checkpoint_code_hash(),
        &type_ids.checkpoint_type_id(),
    );
    let withdraw_code_hash = type_ids.withdraw_code_hash();
    let metadata_type_id =
        get_script_hash(&type_ids.metadata_code_hash(), &type_ids.metadata_type_id());

    if stake_smt_type_id != cell_type_id.as_slice() {
        return Err(Error::StakeSmtTypeIdMismatch);
    }
    let old_stake_smt_data = get_stake_smt_data(&cell_type_id, Source::Input)?;
    let new_stake_smt_data = get_stake_smt_data(&cell_type_id, Source::Output)?;
    if old_stake_smt_data.version() != new_stake_smt_data.version()
        || old_stake_smt_data.metadata_type_id() != new_stake_smt_data.metadata_type_id()
    {
        return Err(Error::StakeSmtUpdateDataError);
    }

    // construct old stake smt root & verify
    let current_epoch = get_current_epoch(&checkpoint_script_hash.to_vec())?;
    let min_inguaration_epoch = current_epoch + 2;
    let old_stake_info_set = transform_to_set(&stake_smt_update_infos.all_stake_infos());
    debug!(
        "current epoch:{}, old stake_infos_set len: {}",
        current_epoch,
        old_stake_info_set.len()
    );
    verify_old_stake_infos(
        min_inguaration_epoch,
        &stake_smt_update_infos,
        old_stake_smt_data,
        &old_stake_info_set,
    )?;

    // get delta stake infos by parsing Stake AT cells' data
    let stake_deltas = get_stake_deltas(
        &xudt_type_hash.as_slice().try_into().unwrap(),
        &type_ids.stake_at_code_hash(),
        Source::Input,
    )?;
    debug!("stake_deltas.len():{}", stake_deltas.len());
    let mut new_stake_info_set = BTreeSet::new();
    for (staker_addr, _stake_at_lock_hash, stake_info_delta) in &stake_deltas {
        let inauguration_epoch = stake_info_delta.inauguration_epoch();
        if inauguration_epoch < min_inguaration_epoch {
            return Err(Error::StaleStakeInfo); // kicker shouldn't update stale stake info
        }
        debug!(
            "staker_addr:{:?}, inauguration_epoch: {:?}",
            staker_addr, inauguration_epoch
        );
        if let Some(entry) = old_stake_info_set
            .iter()
            .find(|info| info.addr == *staker_addr)
        {
            let mut amount = entry.amount;
            if stake_info_delta.is_increase() == 1 {
                amount += bytes_to_u128(&stake_info_delta.amount());
            } else {
                let unstake_amount = bytes_to_u128(&stake_info_delta.amount());
                if amount < unstake_amount {
                    return Err(Error::UnstakeTooMuch);
                }
                amount -= unstake_amount;
            }
            let new_lock_info = LockInfo {
                addr: *staker_addr,
                amount: amount,
            };
            new_stake_info_set.insert(new_lock_info);
        } else {
            // this staker has not been updated to stake smt yet, a new entry
            let new_lock_info = LockInfo {
                addr: *staker_addr,
                amount: bytes_to_u128(&stake_info_delta.amount()),
            };
            new_stake_info_set.insert(new_lock_info);
        }
    }
    for old_info in &old_stake_info_set {
        if let Some(_) = new_stake_info_set
            .iter()
            .find(|new_info| new_info.addr == old_info.addr)
        {
            debug!("staker already updated: {:?}", old_info.addr);
        } else {
            new_stake_info_set.insert(old_info.clone());
        }
    }

    let (select_stake_info_set, delete_stake_info_set) =
        get_selected_unselected_staker(&new_stake_info_set, &metadata_type_id)?;
    debug!("verify_staker_selection");
    verify_staker_selection(
        &select_stake_info_set,
        &new_stake_smt_data,
        &stake_smt_update_infos,
        min_inguaration_epoch,
    )?;

    for select_stake_info in select_stake_info_set {
        if let Some(delta) = stake_deltas
            .iter()
            .find(|delta| delta.0 == select_stake_info.addr)
        {
            let stake_at_lock_hash = delta.1;
            // after updated to smt cell, the output stake should be reset
            let (_, output_stake_at_data) =
                get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
            debug!("is_output_lock_info_reset, staker: {:?}", delta.0);
            is_output_lock_info_reset(&output_stake_at_data)?;
            debug!("update_stake_info");
            update_stake_info(
                delta.0,
                &metadata_type_id,
                &withdraw_code_hash,
                &delta.2,
                // &mut old_stake_info_set,
            )?;
        } else {
            debug!(
                "select staker {:?} no change this time",
                select_stake_info.addr
            );
        }
    }

    for delete_stake_info in delete_stake_info_set {
        debug!("delete staker {:?}", delete_stake_info);
        let mut withdraw_amount = delete_stake_info.amount;
        if let Some(delta) = stake_deltas
            .iter()
            .find(|delta| delta.0 == delete_stake_info.addr)
        {
            let delta = &delta.2;
            if delta.is_increase() == 1 {
                withdraw_amount -= bytes_to_u128(&delta.amount());
            } else {
                withdraw_amount += bytes_to_u128(&delta.amount());
            }
        }
        // withdraw all smt amount of delete staker
        verify_withdraw_amount(
            withdraw_amount,
            &metadata_type_id,
            &delete_stake_info.addr,
            &withdraw_code_hash,
        )?;
        //keep the stake at cell not changed, todo
    }

    Ok(())
}

fn elect_validators(metadata_type_id: &[u8; 32]) -> Result<(), Error> {
    debug!("smt cell elect validators mode");
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
