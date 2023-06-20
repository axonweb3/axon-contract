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
        if input_info.version() != output_info.version()
            || input_info.metadata_type_id() != output_info.metadata_type_id()
        {
            return Err(Error::WithdrawUpdateDataError);
        }
    }

    Ok(())
}

// the old_stake_infos_set is the stake infos of old epoch， the stake_info_delta is the stake change of this epoch
// if the staker's stake is changed in this epoch, we need to update the stake_infos_set
// if not, we need to add the staker's stake info to stake_infos_set
fn update_stake_info(
    addr: [u8; 20],
    metadata_type_id: &[u8; 32],
    withdraw_code_hash: &Vec<u8>,
    stake_info_delta: &StakeInfoDelta,
    stake_infos_set: &mut BTreeSet<LockInfo>,
) -> Result<(), Error> {
    // get this staker's old stake amount in smt tree from stake_infos_set
    let mut old_stake = 0u128;
    let stake_info = stake_infos_set
        .iter()
        .find(|stake_info| addr == stake_info.addr);

    // the staker's info should be updated, so we deleted it from stake_infos_set first, we will insert it in the future
    if let Some(stake_info) = stake_info {
        old_stake = stake_info.amount;
        let stake_info_clone = LockInfo {
            addr: stake_info.addr,
            amount: stake_info.amount,
        };
        stake_infos_set.remove(&stake_info_clone);
    }

    let delta_stake = bytes_to_u128(&stake_info_delta.amount());
    let input_increase = stake_info_delta.is_increase() == 1;
    // calculate the stake of output
    let mut unstake_amount = 0u128;
    if input_increase {
        old_stake += delta_stake;
    } else {
        if delta_stake > old_stake {
            // unstake amount larger than staked amount, set to 0
            unstake_amount = old_stake;
            old_stake = 0;
        } else {
            unstake_amount = delta_stake;
            old_stake -= delta_stake;
        }
    }

    let stake_info_obj = LockInfo {
        addr: addr,
        amount: old_stake,
    };
    stake_infos_set.insert(stake_info_obj);

    // get input & output withdraw AT cell, we need to update this after withdraw script's finish
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
    debug!("epoch_proof:{:?}", epoch_proof);
    verify_2layer_smt(
        &stake_infos_set,
        u64_to_h256(epoch),
        epoch_root,
        epoch_proof,
    )?;
    Ok(())
}

fn verify_staker_seletion(
    stake_infos_set: &BTreeSet<LockInfo>,
    new_stake_smt_data: &StakeSmtCellData,
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    epoch: u64,
    metadata_type_id: &[u8; 32],
) -> Result<(), Error> {
    // sort stakes by amount
    let quorum_size = get_quorum_size(metadata_type_id, Source::CellDep)?;
    let iter = stake_infos_set.iter();
    let mut top_3quorum = iter.take(3 * quorum_size as usize);
    let mut new_stake_infos_set = BTreeSet::new();
    while let Some(elem) = top_3quorum.next() {
        new_stake_infos_set.insert((*elem).clone());
    }

    // get proof of new_stakes from Stake AT cells' witness of input,
    // verify delete_stakes is default
    // verify the new stake infos is equal to on-chain calculation
    let new_epoch_root: [u8; 32] = new_stake_smt_data.smt_root().as_slice().try_into().unwrap(); // get from output smt cell
    let new_epoch_root: H256 = new_epoch_root.into();
    let new_epoch_proof = stake_smt_update_infos.new_epoch_proof();
    let new_epoch_proof = CompiledMerkleProof(new_epoch_proof);
    verify_2layer_smt(
        &new_stake_infos_set,
        u64_to_h256(epoch),
        new_epoch_root,
        new_epoch_proof,
    )?;

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
    let output_increase: bool = output_stake_info.is_increase() == 1;
    let output_inaugutation_epoch = output_stake_info.inauguration_epoch();

    if output_stake != 0 || !output_increase || output_inaugutation_epoch != 0 {
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
    let epoch = get_current_epoch(&checkpoint_script_hash.to_vec())?;
    debug!("current epoch:{}", epoch);
    let mut stake_infos_set = transform_to_set(&stake_smt_update_infos.all_stake_infos());
    verify_old_stake_infos(
        epoch,
        &stake_smt_update_infos,
        old_stake_smt_data,
        &stake_infos_set,
    )?;

    // get delta stake infos by parsing Stake AT cells' data
    let update_infos = get_stake_update_infos(
        &xudt_type_hash.as_slice().try_into().unwrap(),
        Source::Input,
    )?;
    for (staker_addr, stake_at_lock_hash, stake_info_delta) in update_infos {
        let inauguration_epoch = stake_info_delta.inauguration_epoch();
        if inauguration_epoch < epoch + 2 {
            return Err(Error::StaleStakeInfo); // kicker shouldn't update stale stake info
        }

        // after updated to smt cell, the output stake should be reset
        let (_, output_stake_at_data) =
            get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
        is_output_lock_info_reset(&output_stake_at_data)?;

        update_stake_info(
            staker_addr,
            &metadata_type_id,
            &withdraw_code_hash,
            &stake_info_delta,
            &mut stake_infos_set,
        )?;
    }

    verify_staker_seletion(
        &stake_infos_set,
        &new_stake_smt_data,
        &stake_smt_update_infos,
        epoch,
        &metadata_type_id,
    )?;

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
