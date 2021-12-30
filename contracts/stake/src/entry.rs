// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::{
    vec::Vec, vec, collections::BTreeSet
};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source, high_level::{
        load_script, load_cell_type_hash, load_cell_data, load_cell_lock, load_witness_args, QueryIter
    }, ckb_types::{
        bytes::Bytes, prelude::*
    },
};

use crate::error::Error;
use protocol::{
	Cursor, axon::{
		self, StakeInfo
	}
};

enum FILTER {
    APPLIED,
    APPLYING,
    NOTAPPLY
}

fn get_stake_data_by_type_hash(cell_type_hash: &[u8; 32], source: Source) -> Result<axon::StakeLockCellData, Error> {
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

fn get_checkpoint_from_celldeps(checkpoint_type_hash: &Vec<u8>) -> Result<axon::CheckpointLockCellData, Error> {
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
                    let checkpoint_data: axon::CheckpointLockCellData = Cursor::from(data.unwrap()).into();
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

// fn filter_stakeinfos_by_era(era: u64, stake_infos: &axon::StakeInfoVec, filter_type: FILTER) -> Vec<axon::StakeInfo> {
//     let mut filtered_stake_infos = vec![];
//     match filter_type {
//         FILTER::APPLIED => {
            
//         },
//         FILTER::APPLYING => {

//         },
//         FILTER::NOTAPPLY => {
//             for i in 0..stake_infos.len() {
//                 let stake_info = &stake_infos.get(i);
//                 if bytes_to_u64(&stake_info.inauguration_era()) > era + 1 {
//                     filtered_stake_infos.push(stake_info.clone());
//                 }
//             }
//         }
//     }
//     filtered_stake_infos
// }

// fn stakeinfos_diff(stake_infos_1: &Vec<axon::StakeInfo>, stake_infos_2: &Vec<axon::StakeInfo>) -> Vec<axon::StakeInfo> {
    
// }

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // extract stake_args 
    let stake_args: axon::StakeLockArgs = Cursor::from(args.to_vec()).into();
    let admin_identity = stake_args.admin_identity();
    let type_id_hash = stake_args.type_id_hash();
    let node_identity = stake_args.node_identity();

	// check this is wether admin signature or normal signature
	let witness_args = load_witness_args(0, Source::GroupInput)?;
	let is_admin = {
		let input_type = witness_args.input_type().to_opt();
		if input_type.is_none() {
			return Err(Error::BadWitnessInputType);
		}
		match input_type.unwrap().raw_data().to_vec().first() {
			Some(value) => value == &0,
			None        => return Err(Error::BadWitnessInputType)
		}
	};

	// extract AT script_hash from type_script
	let at_type_hash = {
		let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
		if type_hash.is_none() {
			return Err(Error::TypeScriptEmpty);
		}
		type_hash.unwrap()
	};

    // mode select
    if is_admin {
		// check admin signature
		if !secp256k1::verify_signature(&mut admin_identity.content()) {
			return Err(Error::SignatureMismatch);
		}
        // burn mode
        if node_identity.is_some() {
            let mut at_cell_count = 0;
            QueryIter::new(load_cell_type_hash, Source::Output)
                .for_each(|type_hash| {
                    if type_hash.unwrap_or([0u8; 32]) == at_type_hash {
                        at_cell_count += 1;
                    }
                });
            if at_cell_count != 0 {
                return Err(Error::ATCellShouldEmpty);
            }
        // admin mode
        } else {
            let input_stake_data = get_stake_data_by_type_hash(&at_type_hash, Source::Input)?;
            let output_stake_data = get_stake_data_by_type_hash(&at_type_hash, Source::Output)?;
            if input_stake_data.version() != output_stake_data.version()
                || input_stake_data.checkpoint_type_hash() != output_stake_data.checkpoint_type_hash()
                || input_stake_data.sudt_type_hash() != output_stake_data.sudt_type_hash()
                || output_stake_data.quorum_size() > 160 {
                return Err(Error::AdminModeError);
            }
        }
    } else {
        // update mode
        if node_identity.is_some() {
            // check normal signature
            if !secp256k1::verify_signature(&mut node_identity.unwrap().content()) {
                return Err(Error::SignatureMismatch);
            }
            // check stake_data between input and output
            let input_stake_data = get_stake_data_by_type_hash(&at_type_hash, Source::Input)?;
            let output_stake_data = get_stake_data_by_type_hash(&at_type_hash, Source::Output)?;
            if input_stake_data.version() != output_stake_data.version()
                || input_stake_data.checkpoint_type_hash() != output_stake_data.checkpoint_type_hash()
                || input_stake_data.sudt_type_hash() != output_stake_data.sudt_type_hash()
                || input_stake_data.quorum_size() != output_stake_data.quorum_size() {
                return Err(Error::UpdateModeError);
            }
            // get checkpoint data from celldeps
            let checkpoint = get_checkpoint_from_celldeps(&input_stake_data.checkpoint_type_hash())?;
            let era = bytes_to_u64(&checkpoint.era());
            // get different stake_info between input not applied stake_infos and output not
            // applied stake_infos
            //
            // let input_notapply_stake_infos = filter_stakeinfos_by_era(era, &input_stake_data.stake_infos(), FILTER::NOTAPPLY);
            // let output_notapply_stake_infos = filter_stakeinfos_by_era(era, &output_stake_data.stake_infos(), FILTER::NOTAPPLY);
            // let dumplicate_value_check = {
            //     if output_notapply_stake_infos.len() != input_notapply_stake_infos.len() + 1 {
            //         return Err(Error::NotApplyStakeInfoError);
            //     }
            //     let diff_infos = stakeinfos_diff(&output_notapply_stake_infos, &input_notapply_stake_infos);
            //     assert!(diff_infos.len() == 1);
            //     let mut btree_set = BTreeSet::new();
            //     diff_infos.first().unwrap().clone()
            // };
            // // check dumplicate stake_info in stake_infos from input
            // let mut input_stake_infos: Vec<axon::StakeInfo> = input_stake_data.stake_infos().into();
            // input_stake_infos.push(node_stake_info);
            
        // companion mode
        } else {
            let mut find_node_identity = false;
            QueryIter::new(load_cell_lock, Source::Input)
                .for_each(|lock| {
                    if lock.code_hash().as_slice() == script.code_hash().as_slice() {
                        let lock_args: axon::StakeLockArgs = {
                            let bytes: Bytes = lock.args().unpack();
                            Cursor::from(bytes.to_vec()).into()
                        };
                        if lock_args.node_identity().is_some() {
                            find_node_identity = true;
                        }
                    }
                });
            if !find_node_identity {
                return Err(Error::CompanionModeError);
            }
        }
    }

    Ok(())
}
