// Import from `core` instead of from `std` since we are in no-std mode
use alloc::{collections::BTreeSet, vec::Vec};
use axon_types::metadata_reader;
use axon_types::stake_reader::StakeAtCellData;
use axon_types::stake_reader::StakeInfoDelta;
use axon_types::stake_reader::StakeInfos;
use axon_types::stake_reader::StakeSmtCellData;
use axon_types::stake_reader::StakeSmtUpdateInfo;
use ckb_std::ckb_types::packed::WitnessArgs;
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
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{stake_reader, Cursor};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract stake_args
    let stake_args: stake_reader::StakeArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = stake_args.metadata_type_id();
    let staker_identity = stake_args.stake_addr();

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let value = witness.input_type().to_opt();
            if value.is_none() || value.as_ref().unwrap().len() != 1 {
                return Err(Error::BadWitnessInputType);
            }

            let input_type = *value.unwrap().raw_data().to_vec().first().unwrap();
            let source = if input_type == 2 {
                Source::Input
            } else {
                Source::CellDep
            };
            let type_ids = get_type_ids(&metadata_type_id.as_slice().try_into().unwrap(), source)?;
            if metadata_type_id != type_ids.metadata_type_id() {
                return Err(Error::MisMatchMetadataTypeId);
            }

            debug!("stake input_type:{}", input_type);
            match input_type {
                0 => {
                    // update stake at cell
                    // extract stake at cell lock hash
                    let stake_at_lock_hash = { load_cell_lock_hash(0, Source::Input)? };
                    // debug!("stake_at_lock_hash:{:?}", stake_at_lock_hash);
                    update_stake_at_cell(
                        &staker_identity.unwrap(),
                        &stake_at_lock_hash,
                        &type_ids.checkpoint_type_id(),
                        &type_ids.xudt_type_hash(),
                    )?;
                }
                1 => {
                    // kicker update stake smt cell
                    update_stake_smt(&staker_identity, &witness, &type_ids)?;
                }
                2 => {
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

pub fn update_stake_at_cell(
    staker_identity: &Vec<u8>,
    stake_at_lock_hash: &[u8; 32],
    checkpoint_type_id: &Vec<u8>,
    xudt_type_hash: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update stake info in stake at cell");
    // if !secp256k1::verify_signature(&staker_identity) {
    //     return Err(Error::SignatureMismatch);
    // }

    check_xudt_type_hash(xudt_type_hash)?;

    let total_input_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Input)?;
    let total_output_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Output)?;
    if total_input_at_amount != total_output_at_amount {
        return Err(Error::InputOutputAtAmountNotEqual);
    }
    debug!(
        "input_at_amount:{}, output_at_amount:{}",
        total_input_at_amount, total_output_at_amount
    );

    let (input_amount, input_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Input)?;
    debug!("input_amount:{}", input_amount);
    let (output_amount, output_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
    debug!("output_amount:{}", output_amount);
    if input_stake_at_data.version() != output_stake_at_data.version()
        || input_stake_at_data.metadata_type_id() != output_stake_at_data.metadata_type_id()
    {
        return Err(Error::UpdateDataError);
    }

    let input_stake_info = input_stake_at_data.delta();
    let input_stake = bytes_to_u128(&input_stake_info.amount());
    let input_increase = input_stake_info.is_increase() == 1;

    let output_stake_info = output_stake_at_data.delta();
    let output_stake = bytes_to_u128(&output_stake_info.amount());
    let output_increase = output_stake_info.is_increase() == 1;
    let output_inaugutation_epoch = output_stake_info.inauguration_epoch();
    debug!(
        "input_stake:{}, output_stake:{}, output_inaugutation_epoch:{}",
        input_stake, output_stake, output_inaugutation_epoch
    );

    let epoch = get_current_epoch(checkpoint_type_id)?;
    debug!("epoch:{}", epoch);
    if output_inaugutation_epoch != epoch + 2 {
        return Err(Error::BadInaugurationEpoch);
    }

    if input_increase {
        if output_increase {
            if output_stake - input_stake != output_amount - input_amount {
                return Err(Error::BadStakeStakeChange);
            }
        } else {
            if input_stake != input_amount - output_amount {
                return Err(Error::BadStakeRedeemChange);
            }
            if output_stake > input_amount {
                return Err(Error::RedeemExceedLimit);
            }
        }
    } else {
        if output_increase {
            if output_stake != output_amount - input_amount {
                return Err(Error::BadStakeChange);
            }
        } else {
            if output_stake > input_amount {
                return Err(Error::RedeemExceedLimit);
            }
        }
    }
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

fn is_output_lock_info_reset(output_stake_at_data: &StakeAtCellData) -> Result<(), Error> {
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
    staker_identity: &Option<Vec<u8>>,
    witness: &WitnessArgs,
    type_ids: &metadata_reader::TypeIds,
) -> Result<(), Error> {
    debug!("update stake smt root mode");
    let xudt_type_hash = type_ids.xudt_type_hash();
    let stake_smt_type_id = type_ids.stake_smt_type_id();
    if staker_identity.is_none() {
        // this is stake smt cell
        let checkpoint_type_id = type_ids.checkpoint_type_id();
        let withdraw_code_hash = type_ids.withdraw_code_hash();
        let metadata_type_id = type_ids.metadata_type_id().as_slice().try_into().unwrap();
        // get old_stakes & proof from Stake AT cells' witness of input
        let stake_smt_update_infos = {
            let witness_lock = witness.lock().to_opt();
            if witness_lock.is_none() {
                return Err(Error::WitnessLockError);
            }
            let value: stake_reader::StakeSmtUpdateInfo =
                Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
            value
        };
        let cell_type_id = {
            let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
            if type_hash.is_none() {
                return Err(Error::TypeScriptEmpty);
            }
            type_hash.unwrap()
        };
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
        let epoch = get_current_epoch(&checkpoint_type_id)?;
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
    } else {
        // staker AT cell
        // only need to verify input and output both contain the Stake SMT cell of the Chain
        let input_smt_cell = get_cell_count_by_type_hash(&stake_smt_type_id, Source::Input);
        if input_smt_cell != 1 {
            return Err(Error::BadInputStakeSmtCellCount);
        }
        let output_smt_cell = get_cell_count_by_type_hash(&stake_smt_type_id, Source::Output);
        if output_smt_cell != 1 {
            return Err(Error::BadOutputStakeSmtCellCount);
        }
        check_xudt_type_hash(&xudt_type_hash)?;
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
