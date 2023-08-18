// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec::Vec;
use axon_types::metadata_reader;

use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_script, load_tx_hash, load_witness_args},
};

use axon_types::{stake_reader, Cursor};
use util::{error::Error, helper::*};

use crate::eth::Secp256k1Eth;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    // extract stake at cell lock hash
    let stake_at_lock_hash = calc_script_hash(&script);
    // debug!("stake_at_lock_hash:{:?}", stake_at_lock_hash);

    // extract stake_args
    let stake_args: stake_reader::StakeArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = stake_args.metadata_type_id();
    let staker_identity = stake_args.stake_addr();
    debug!(
        "metadata_type_id:{:?}, staker_identity: {:?}",
        metadata_type_id, staker_identity
    );
    check_l2_addr(&staker_identity, &stake_at_lock_hash)?;

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput);
    match witness_args {
        Ok(witness) => {
            let (mode, eth_sig) = {
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
                (value.mode(), value.eth_sig())
            };
            debug!(
                "stake at mode: {}, eth_sig: {:?}, len:{}",
                mode,
                eth_sig,
                eth_sig.len()
            );

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
                    let checkpoint_type_hash = get_script_hash(
                        &type_ids.checkpoint_code_hash(),
                        &type_ids.checkpoint_type_id(),
                    );
                    update_stake_at_cell(
                        &staker_identity,
                        &eth_sig,
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

fn check_l2_addr(l2_addr_args: &Vec<u8>, stake_at_lock_hash: &[u8; 32]) -> Result<(), Error> {
    let (_, output_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;

    let l2_addr_cell = output_stake_at_data.l2_address();
    debug!(
        "l2_addr:{:?}, staker_identity:{:?}",
        l2_addr_cell, l2_addr_args
    );
    if l2_addr_cell != l2_addr_args.as_slice() {
        return Err(Error::L1L2AddrMismatch);
    }

    Ok(())
}

fn check_stake_change(
    input_stake_delta: u128,
    output_stake_delta: u128,
    input_stake_at_amount: u128,
    output_stake_at_amount: u128,
) -> Result<(), Error> {
    if output_stake_delta >= input_stake_delta {
        if output_stake_delta - input_stake_delta != output_stake_at_amount - input_stake_at_amount
        {
            return Err(Error::BadStakeStakeChange);
        }
    } else {
        if input_stake_delta - output_stake_delta != input_stake_at_amount - output_stake_at_amount
        {
            return Err(Error::BadStakeStakeChange);
        }
    }
    Ok(())
}

pub fn update_stake_at_cell(
    staker_identity: &Vec<u8>,
    eth_sig: &Vec<u8>,
    stake_at_lock_hash: &[u8; 32],
    checkpoint_type_id: &Vec<u8>,
    xudt_type_hash: &Vec<u8>,
) -> Result<(), Error> {
    debug!("update stake info in stake at cell");
    let msg = load_tx_hash()?;
    let secp256_eth = Secp256k1Eth::default();
    let result = secp256_eth.verify_alone(
        staker_identity.as_slice().try_into().unwrap(),
        eth_sig.as_slice().try_into().unwrap(),
        msg,
    )?;
    debug!(
        "verify_signature eth_sig: {:?}, msg: {:?}, pubkey: {:?}, result: {}",
        eth_sig, msg, staker_identity, result
    );
    if !result {
        return Err(Error::SignatureMismatch);
    }

    check_xudt_type_hash(xudt_type_hash)?;

    let total_input_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Input)?;
    let total_output_at_amount = get_xudt_by_type_hash(xudt_type_hash, Source::Output)?;
    debug!(
        "input_at_amount:{}, output_at_amount:{}",
        total_input_at_amount, total_output_at_amount
    );
    if total_input_at_amount != total_output_at_amount {
        return Err(Error::InputOutputAtAmountNotEqual);
    }

    let (input_stake_at_amount, input_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Input)?;
    let (output_stake_at_amount, output_stake_at_data) =
        get_stake_at_data_by_lock_hash(&stake_at_lock_hash, Source::Output)?;
    debug!(
        "input_stake_at_amount:{}, output_stake_at_amount:{}",
        input_stake_at_amount, output_stake_at_amount
    );
    if input_stake_at_data.version() != output_stake_at_data.version()
        || input_stake_at_data.metadata_type_id() != output_stake_at_data.metadata_type_id()
    {
        return Err(Error::UpdateDataError);
    }

    let input_stake_info_delta = input_stake_at_data.delta();
    let input_stake_delta = bytes_to_u128(&input_stake_info_delta.amount());
    let input_increase = input_stake_info_delta.is_increase() == 1;

    let output_stake_info_delta = output_stake_at_data.delta();
    let output_stake_delta = bytes_to_u128(&output_stake_info_delta.amount());
    let output_increase = output_stake_info_delta.is_increase() == 1;
    let output_inaugutation_epoch = output_stake_info_delta.inauguration_epoch();

    let current_epoch = get_current_epoch(checkpoint_type_id)?;
    debug!(
        "input_stake_delta:{}, output_stake_delta:{}, output_inaugutation_epoch:{}, current_epoch:{}",
        input_stake_delta, output_stake_delta, output_inaugutation_epoch, current_epoch
    );
    if output_inaugutation_epoch != current_epoch + 2 {
        return Err(Error::BadInaugurationEpoch);
    }

    if input_increase {
        if output_increase {
            check_stake_change(
                input_stake_delta,
                output_stake_delta,
                input_stake_at_amount,
                output_stake_at_amount,
            )?;
        } else {
            if input_stake_delta != input_stake_at_amount - output_stake_at_amount {
                return Err(Error::BadStakeRedeemChange);
            }
        }
    } else {
        if output_increase {
            if output_stake_delta != output_stake_at_amount - input_stake_at_amount {
                return Err(Error::BadStakeChange);
            }
        }
    }

    if !output_increase {
        if output_stake_delta > output_stake_at_amount {
            return Err(Error::UnstakeTooMuch);
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
