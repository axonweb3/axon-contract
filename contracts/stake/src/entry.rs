// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::{collections::BTreeSet, vec, vec::Vec};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        load_cell_data, load_cell_lock, load_cell_type_hash, load_script, load_witness_args,
        QueryIter,
    },
};

use crate::error::Error;
use protocol::{
    axon::{self, StakeInfo},
    read_at, Cursor,
};

enum FILTER {
    APPLIED,
    APPLYING,
    NOTAPPLY,
}

enum MODE {
    UPDATE,
    BURN,
    ADMIN,
    COMPANION,
}

fn get_stake_data_by_type_hash(
    cell_type_hash: &[u8; 32],
    source: Source,
) -> Result<axon::StakeLockCellData, Error> {
    let mut stake_data = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                assert!(stake_data.is_none());
                stake_data = {
                    let data = load_cell_data(i, source);
                    if let Err(_) = data {
                        return Err(Error::StakeDataError);
                    }
                    let stake_data: axon::StakeLockCellData = Cursor::from(data.unwrap()).into();
                    Some(stake_data)
                };
            }
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if stake_data.is_none() {
        return Err(Error::StakeDataEmpty);
    }
    Ok(stake_data.unwrap())
}

fn get_checkpoint_from_celldeps(
    checkpoint_type_hash: &Vec<u8>,
) -> Result<axon::CheckpointLockCellData, Error> {
    let mut checkpoint_data = None;
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .map(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32])[..] == checkpoint_type_hash[..] {
                assert!(checkpoint_data.is_none());
                checkpoint_data = {
                    let data = load_cell_data(i, Source::CellDep);
                    if let Err(_) = data {
                        return Err(Error::CheckpointDataError);
                    }
                    let checkpoint_data: axon::CheckpointLockCellData =
                        Cursor::from(data.unwrap()).into();
                    Some(checkpoint_data)
                };
            }
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if checkpoint_data.is_none() {
        return Err(Error::CheckpointDataEmpty);
    }
    Ok(checkpoint_data.unwrap())
}

fn bytes_to_u64(bytes: &Vec<u8>) -> u64 {
    let mut array: [u8; 8] = [0u8; 8];
    array.copy_from_slice(bytes.as_slice());
    u64::from_le_bytes(array)
}

fn bytes_to_u32(bytes: &Vec<u8>) -> u32 {
    let mut array: [u8; 4] = [0u8; 4];
    array.copy_from_slice(bytes.as_slice());
    u32::from_le_bytes(array)
}

fn filter_stakeinfos_by_era(
    era: u64,
    stake_infos: &axon::StakeInfoVec,
    filter_type: FILTER,
) -> Result<BTreeSet<Vec<u8>>, Error> {
    let mut filtered_stake_infos = BTreeSet::new();
    match filter_type {
        FILTER::APPLIED => {}
        FILTER::APPLYING => {}
        FILTER::NOTAPPLY => {
            for i in 0..stake_infos.len() {
                let stake_info = stake_infos.get(i);
                if bytes_to_u64(&stake_info.inauguration_era()) > era + 1 {
                    let mut bytes = vec![0u8; stake_info.cursor.size];
                    read_at(&stake_info.cursor, bytes.as_mut_slice());
                    if !filtered_stake_infos.insert(bytes.to_vec()) {
                        return Err(Error::StakeDataEmpty);
                    }
                }
            }
        }
    }
    Ok(filtered_stake_infos)
}

fn stakeinfos_into_set(stake_infos: &axon::StakeInfoVec) -> BTreeSet<Vec<u8>> {
    let mut btree_set = BTreeSet::new();
    for i in 0..stake_infos.len() {
        let stake_info = stake_infos.get(i);
        let mut bytes = vec![0u8; stake_info.cursor.size];
        read_at(&stake_info.cursor, bytes.as_mut_slice());
        btree_set.insert(bytes.to_vec());
    }
    btree_set
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract stake_args
    let stake_args: axon::StakeLockArgs = Cursor::from(args.to_vec()).into();
    let admin_identity = stake_args.admin_identity();
    let type_id_hash = stake_args.type_id_hash();
    let node_identity = stake_args.node_identity();

    // identify contract mode by witness
    let mode = match load_witness_args(0, Source::GroupInput) {
        Ok(witness) => {
            let value = witness.input_type().to_opt();
            if value.is_none() || value.as_ref().unwrap().len() != 1 {
                return Err(Error::BadWitnessInputType);
            }
            if value.unwrap().raw_data().to_vec().first().unwrap() == &0 {
                if node_identity.is_none() {
                    MODE::ADMIN
                } else {
                    MODE::BURN
                }
            } else {
                if node_identity.is_none() {
                    return Err(Error::UnknownMode);
                }
                MODE::COMPANION
            }
        }
        Err(_) => MODE::UPDATE,
    };

    // extract AT type_id from type_script
    let type_id_scripthash = {
        let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
        if type_hash.is_none() {
            return Err(Error::TypeScriptEmpty);
        }
        type_hash.unwrap()
    };

    match mode {
        MODE::ADMIN => {
            debug!("admin mode");
            // check admin signature
            if !secp256k1::verify_signature(&mut admin_identity.content()) {
                return Err(Error::SignatureMismatch);
            }
            let input_stake_data = get_stake_data_by_type_hash(&type_id_scripthash, Source::Input)?;
            let output_stake_data =
                get_stake_data_by_type_hash(&type_id_scripthash, Source::Output)?;
            if input_stake_data.version() != output_stake_data.version()
                || input_stake_data.checkpoint_type_hash()
                    != output_stake_data.checkpoint_type_hash()
                || input_stake_data.sudt_type_hash() != output_stake_data.sudt_type_hash()
                || output_stake_data.quorum_size() > 160
            {
                return Err(Error::AdminModeError);
            }
        }
        MODE::BURN => {
            debug!("burn mode");
            // check admin signature
            if !secp256k1::verify_signature(&mut admin_identity.content()) {
                return Err(Error::SignatureMismatch);
            }
            let mut at_cell_count = 0;
            QueryIter::new(load_cell_type_hash, Source::Output).for_each(|type_hash| {
                if type_hash.unwrap_or([0u8; 32]) == type_id_scripthash {
                    at_cell_count += 1;
                }
            });
            if at_cell_count != 0 {
                return Err(Error::ATCellShouldEmpty);
            }
        }
        MODE::COMPANION => {
            debug!("companion mode");
            // check normal signature
            if !secp256k1::verify_signature(&mut node_identity.unwrap().content()) {
                return Err(Error::SignatureMismatch);
            }
            let mut find_type_hash = false;
            QueryIter::new(load_cell_type_hash, Source::Input).for_each(|type_hash| {
                if type_hash.unwrap_or([0u8; 32])[..] == type_id_hash[..] {
                    find_type_hash = true;
                }
            });
            if !find_type_hash {
                return Err(Error::CompanionModeError);
            }
        }
        MODE::UPDATE => {
            debug!("update mode");
            // check stake_data between input and output
            // let input_stake_data = get_stake_data_by_type_hash(&type_id_scripthash, Source::Input)?;
            // let output_stake_data = get_stake_data_by_type_hash(&type_id_scripthash, Source::Output)?;
            // if input_stake_data.version() != output_stake_data.version()
            //     || input_stake_data.checkpoint_type_hash() != output_stake_data.checkpoint_type_hash()
            //     || input_stake_data.sudt_type_hash() != output_stake_data.sudt_type_hash()
            //     || input_stake_data.quorum_size() != output_stake_data.quorum_size() {
            //     return Err(Error::UpdateModeError);
            // }

            // // get checkpoint data from celldeps
            // let checkpoint = get_checkpoint_from_celldeps(&input_stake_data.checkpoint_type_hash())?;
            // let era = bytes_to_u64(&checkpoint.era());
            // let period = bytes_to_u64(&checkpoint.period());
            // let unlock_period = bytes_to_u32(&checkpoint.unlock_period());

            // // get different stake_info between input not_applied stake_infos and output not_applied stake_infos
            // let input_stake_infos = input_stake_data.stake_infos();
            // let output_stake_infos = output_stake_data.stake_infos();
            // let input_notapply_stake_infos = filter_stakeinfos_by_era(era, &input_stake_infos, FILTER::NOTAPPLY)?;
            // let output_notapply_stake_infos = filter_stakeinfos_by_era(era, &output_stake_infos, FILTER::NOTAPPLY)?;
            // let node_stake_info = {
            //     if output_notapply_stake_infos.len() != input_notapply_stake_infos.len() + 1 {
            //         return Err(Error::NotApplyStakeInfoError);
            //     }
            //     let diff_stake_infos = output_notapply_stake_infos
            // 		.symmetric_difference(&input_notapply_stake_infos)
            // 		.cloned()
            // 		.collect::<Vec<u8>>();
            // 	if diff_stake_infos.len() != 1 {
            //         return Err(Error::NotApplyStakeInfoError);
            // 	}
            //     diff_stake_infos.first().unwrap()
            // };

            // // check dumplicate stake_info in stake_infos from input
            // let mut stake_infos = stakeinfos_into_set(&input_stake_infos);
            // if !stake_infos.insert(node_stake_info) {
            // 	return Err(Error::DumplicateInputStakeInfo);
            // }
        }
    }

    Ok(())
}
