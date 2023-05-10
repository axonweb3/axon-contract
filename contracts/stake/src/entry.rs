// Import from `core` instead of from `std` since we are in no-std mode
use alloc::{collections::BTreeSet, vec::Vec};
use axon_types::stake_reader::StakeInfoDelta;
use axon_types::stake_reader::StakeSmtCellData;
use axon_types::stake_reader::StakeSmtUpdateInfo;
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
    stake_reader::{self as axon},
    Cursor,
};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract stake_args
    let stake_args: axon::StakeArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = stake_args.metadata_type_id();
    let staker_identity = stake_args.stake_addr();

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
                // update stake at cell
                // extract stake at cell lock hash
                let stake_at_lock_hash = { load_cell_lock_hash(0, Source::GroupInput)? };
                update_stake_at_cell(
                    &staker_identity.unwrap(),
                    &stake_at_lock_hash,
                    &metadata_type_ids.checkpoint_type_id(),
                    &metadata_type_ids.xudt_type_id(),
                )?;
            } else if input_type == 1 {
                // kicker update stake smt cell
                // get old_stakes & proof from Stake AT cells' witness of input
                let stake_smt_update_infos = {
                    let witness_lock = witness.lock().to_opt();
                    if witness_lock.is_none() {
                        return Err(Error::WitnessLockError);
                    }
                    let value: axon::StakeSmtUpdateInfo =
                        Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
                    value
                };
                let metadata_type_id: [u8; 32] = metadata_type_id.as_slice().try_into().unwrap();
                update_stake_smt(
                    &staker_identity,
                    &stake_smt_update_infos,
                    &metadata_type_ids.checkpoint_type_id(),
                    &metadata_type_ids.xudt_type_id(),
                    &metadata_type_ids.stake_smt_type_id(),
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

pub fn update_stake_at_cell(
    staker_identity: &Vec<u8>,
    stake_at_lock_hash: &[u8; 32],
    checkpoint_type_id: &Vec<u8>,
    xudt_type_id: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update stake info in stake at cell");
    if !secp256k1::verify_signature(&staker_identity) {
        return Err(Error::SignatureMismatch);
    }

    check_xudt_type_id(xudt_type_id)?;

    let input_at_amount = get_xudt_by_type_hash(xudt_type_id, Source::Input)?;
    let output_at_amount = get_xudt_by_type_hash(xudt_type_id, Source::Output)?;
    if input_at_amount != output_at_amount {
        return Err(Error::InputOutputAtAmountNotEqual);
    }

    let (input_amount, input_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Input)?;
    let (output_amount, output_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
    if input_stake_at_data.version() != output_stake_at_data.version()
        || input_stake_at_data.metadata_type_id() != output_stake_at_data.metadata_type_id()
    {
        return Err(Error::UpdateDataError);
    }

    let epoch = get_current_epoch(checkpoint_type_id)?;

    let input_stake_info = input_stake_at_data.stake_info();
    let input_stake = bytes_to_u128(&input_stake_info.amount());
    let input_increase = input_stake_info.is_increase() == 1;

    let output_stake_info = output_stake_at_data.stake_info();
    let output_stake = bytes_to_u128(&output_stake_info.amount());
    let output_increase = output_stake_info.is_increase() == 1;
    let output_inaugutation_epoch = input_stake_info.inauguration_epoch();

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

fn update_stake_info(
    addr: [u8; 20],
    stake_at_lock_hash: &[u8; 32],
    stake_info_delta: &StakeInfoDelta,
    stake_infos_set: &mut BTreeSet<StakeInfoObject>,
) -> Result<(), Error> {
    // get this staker's old stake amount in smt tree from stake_infos_set
    let stake_info = stake_infos_set
        .iter()
        .find(|stake_info| addr == stake_info.identity);
    let mut stake_info_clone: Option<StakeInfoObject> = None;
    let mut old_stake: u128;
    if let Some(stake_info) = stake_info {
        old_stake = stake_info.stake_amount;
        stake_info_clone = Some(StakeInfoObject {
            identity: stake_info.identity,
            stake_amount: stake_info.stake_amount,
        })
    } else {
        // the staker has not changed stake in this epoch yet,
        // should be staker's at amount, get from stake at cell
        (old_stake, _) = get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Input)?;
    }

    // the staker's info should be updated, so we deleted it from stake_infos_set first, we will insert it in the future
    if stake_info_clone.is_some() {
        stake_infos_set.remove(&stake_info_clone.unwrap());
    }

    let input_stake = bytes_to_u128(&stake_info_delta.amount());
    let input_increase = stake_info_delta.is_increase() == 1;
    // calculate the stake of output
    let mut redeem_amount = 0u128;
    if input_increase {
        old_stake += input_stake;
    } else {
        if input_stake > old_stake {
            redeem_amount = old_stake;
            old_stake = 0;
        } else {
            redeem_amount = input_stake;
            old_stake -= input_stake;
        }
    }

    let stake_info_obj = StakeInfoObject {
        identity: addr,
        stake_amount: old_stake,
    };
    stake_infos_set.insert(stake_info_obj);

    // get input & output withdraw AT cell, we need to update this after withdraw script's finish
    if redeem_amount > 0 {
        let input_withdraw_amount = 10u128; // get from staker input withdraw at cell
        let output_withdraw_amount = 10u128; // get from staker input withdraw at cell
        if output_withdraw_amount - input_withdraw_amount != redeem_amount {
            return Err(Error::BadRedeem);
        }
    }

    Ok(())
}

fn verify_old_stake_infos(
    epoch: u64,
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    old_stake_smt_data: StakeSmtCellData,
    stake_infos_set: &BTreeSet<StakeInfoObject>,
) -> Result<(), Error> {
    let epoch_root: [u8; 32] = old_stake_smt_data.smt_root().as_slice().try_into().unwrap(); // get from input smt cell
    let epoch_proof = stake_smt_update_infos.old_epoch_proof();
    verify_2layer_smt_stake(&stake_infos_set, epoch, &epoch_proof, &epoch_root)?;
    Ok(())
}

fn verify_staker_seletion(
    stake_infos_set: &BTreeSet<StakeInfoObject>,
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
    let new_epoch_proof = stake_smt_update_infos.new_epoch_proof();
    verify_2layer_smt_stake(
        &new_stake_infos_set,
        epoch,
        &new_epoch_proof,
        &new_epoch_root,
    )?;

    Ok(())
}

fn update_stake_smt(
    staker_identity: &Option<Vec<u8>>,
    stake_smt_update_infos: &StakeSmtUpdateInfo,
    checkpoint_type_id: &Vec<u8>,
    xudt_type_id: &Vec<u8>,
    stake_smt_type_id: &Vec<u8>,
    metadata_type_id: &[u8; 32],
) -> Result<(), Error> {
    debug!("update stake smt root mode");
    if staker_identity.is_none() {
        // this is stake smt cell
        let type_id = {
            let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
            if type_hash.is_none() {
                return Err(Error::TypeScriptEmpty);
            }
            type_hash.unwrap()
        };
        let old_stake_smt_data = get_stake_smt_data(&type_id, Source::Input)?;
        let new_stake_smt_data = get_stake_smt_data(&type_id, Source::Output)?;
        if old_stake_smt_data.version() != new_stake_smt_data.version()
            || old_stake_smt_data.metadata_type_id() != new_stake_smt_data.metadata_type_id()
        {
            return Err(Error::UpdateDataError);
        }

        // construct old stake smt root & verify
        let epoch = get_current_epoch(&checkpoint_type_id)?;
        let mut stake_infos_set = transform_to_set(&stake_smt_update_infos.all_stake_infos());
        verify_old_stake_infos(
            epoch,
            stake_smt_update_infos,
            old_stake_smt_data,
            &stake_infos_set,
        )?;

        // get proof of new_stakes from Stake AT cells' witness of input,
        // verify delete_stakes is zero
        let stake_at_type_id: [u8; 32] = xudt_type_id.as_slice().try_into().unwrap(); // get from type script
        let update_infos = get_stake_update_infos(&stake_at_type_id, Source::GroupInput)?;
        for (addr, stake_at_lock_hash, stake_info_delta) in update_infos {
            let inauguration_epoch = stake_info_delta.inauguration_epoch();
            if inauguration_epoch < epoch + 2 {
                return Err(Error::StaleStakeInfo);
            }

            // after updated to smt cell, the output stake should be reset
            let (_output_amount, output_stake_at_data) =
                get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
            let output_stake_info = output_stake_at_data.stake_info();
            let output_stake = bytes_to_u128(&output_stake_info.amount());
            let output_increase: bool = output_stake_info.is_increase() == 1;
            let output_inaugutation_epoch = output_stake_info.inauguration_epoch();

            if output_stake != 0 || !output_increase || output_inaugutation_epoch != 0 {
                return Err(Error::IllegalDefaultStakeInfo);
            }

            update_stake_info(
                addr,
                &stake_at_lock_hash,
                &stake_info_delta,
                &mut stake_infos_set,
            )?;
        }

        verify_staker_seletion(
            &stake_infos_set,
            &new_stake_smt_data,
            stake_smt_update_infos,
            epoch,
            metadata_type_id,
        )?;
    } else {
        // staker AT cell
        // only need to verify input and output both contain the Stake SMT cell of the Chain
        let input_smt_cell = get_cell_count(&stake_smt_type_id, Source::Input);
        if input_smt_cell != 1 {
            return Err(Error::BadInputStakeSmtCellCount);
        }
        let output_smt_cell = get_cell_count(&stake_smt_type_id, Source::Output);
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
