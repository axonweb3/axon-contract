// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec::Vec;
use axon_types::metadata_reader;
// use ckb_std::ckb_types::packed::WitnessArgs;

use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_script, load_witness_args},
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
    debug!("metadata_type_id:{:?}", metadata_type_id);

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let mode = {
                let witness_lock = witness.lock().to_opt();
                if witness_lock.is_none() {
                    return Err(Error::WitnessLockError);
                }
                debug!(
                    "witness_lock:{:?}",
                    witness_lock.clone().unwrap().raw_data().len()
                );
                let value: stake_reader::StakeAtWitness =
                    Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
                debug!("witness mode: {}", value.mode());
                value.mode()
            };
            debug!("stake at mode: {}", mode);

            let type_ids = get_type_ids(
                &metadata_type_id.as_slice().try_into().unwrap(),
                Source::CellDep,
            )?;
            if metadata_type_id
                != get_script_hash(&type_ids.metadata_code_hash(), &type_ids.metadata_type_id())
            {
                return Err(Error::MisMatchMetadataTypeId);
            }

            match mode {
                0 => {
                    // update stake at cell
                    // extract stake at cell lock hash
                    let stake_at_lock_hash = { load_cell_lock_hash(0, Source::Input)? };
                    // debug!("stake_at_lock_hash:{:?}", stake_at_lock_hash);
                    let checkpoint_type_hash = get_script_hash(
                        &type_ids.checkpoint_code_hash(),
                        &type_ids.checkpoint_type_id(),
                    );
                    update_stake_at_cell(
                        &staker_identity,
                        &stake_at_lock_hash,
                        &checkpoint_type_hash.to_vec(),
                        &type_ids.xudt_type_hash(),
                    )?;
                }
                1 => {
                    // kicker update stake smt cell
                    update_stake_smt(&type_ids)?;
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

    debug!("stake at cell lock script ok");
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

fn update_stake_smt(type_ids: &metadata_reader::TypeIds) -> Result<(), Error> {
    debug!("at cell update stake smt root mode");
    let xudt_type_hash = type_ids.xudt_type_hash();
    let stake_smt_type_id = get_script_hash(
        &type_ids.stake_smt_code_hash(),
        &type_ids.stake_smt_type_id(),
    )
    .to_vec();

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
    debug!("check_xudt_type_hash");
    check_xudt_type_hash(&xudt_type_hash)?;
    Ok(())
}
