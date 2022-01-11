extern crate alloc;
use alloc::{
    collections::{btree_map::BTreeMap, BTreeSet},
    vec,
    vec::Vec,
};
use blake2b_ref::Blake2bBuilder;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{
        core::ScriptHashType,
        packed::{Byte32, Script},
        prelude::*,
    },
    high_level::{
        load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type_hash, load_cell_capacity,
		QueryIter,
    },
};
use core::{cmp::Ordering, result::Result, convert::TryInto};
use protocol::{
    prelude::{Builder, Entity},
    reader, writer, Cursor,
};
use crate::error::Error;

//////////////////////////////////////////////////////////
/// used by common
//////////////////////////////////////////////////////////

pub fn bytes_to_u128(bytes: &Vec<u8>) -> u128 {
    let mut array: [u8; 16] = [0u8; 16];
    array.copy_from_slice(bytes.as_slice());
    u128::from_le_bytes(array)
}

pub fn bytes_to_u64(bytes: &Vec<u8>) -> u64 {
    let mut array: [u8; 8] = [0u8; 8];
    array.copy_from_slice(bytes.as_slice());
    u64::from_le_bytes(array)
}

pub fn bytes_to_u32(bytes: &Vec<u8>) -> u32 {
    let mut array: [u8; 4] = [0u8; 4];
    array.copy_from_slice(bytes.as_slice());
    u32::from_le_bytes(array)
}

//////////////////////////////////////////////////////////
/// used by withdrawal contract
//////////////////////////////////////////////////////////

pub fn get_total_sudt_by_script_hash(
    cell_lock_hash: &[u8; 32],
    cell_type_hash: &[u8; 32],
    source: Source,
) -> Result<u128, Error> {
    let total_amount = QueryIter::new(load_cell_lock_hash, source)
        .enumerate()
        .map(|(i, lock_hash)| {
            let mut amount = 0;
            if &lock_hash == cell_lock_hash {
                let type_hash = {
                    let type_hash = load_cell_type_hash(i, source);
                    if let Err(_) = type_hash {
                        return Err(Error::BadWithdrawalTypeHash);
                    }
                    match type_hash.unwrap() {
                        Some(value) => value,
                        None => return Err(Error::SomeWithdrawalTypeEmpty),
                    }
                };
                if &type_hash == cell_type_hash {
                    let data = load_cell_data(i, source);
                    if data.is_err() || data.as_ref().unwrap().len() != 24 {
                        return Err(Error::BadWithdrawalData);
                    }
                    amount = bytes_to_u128(&data.unwrap()[..16].to_vec());
                }
            }
            Ok(amount)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<u128>();
    Ok(total_amount)
}

//////////////////////////////////////////////////////////
/// used by stake contract
//////////////////////////////////////////////////////////

pub enum FILTER {
    APPLY,
    NOTAPPLY,
}

#[derive(Clone, PartialEq, Eq, PartialOrd)]
pub struct StakeInfoObject {
    pub identity: [u8; 21],
	pub l2_address: [u8; 20],
	pub bls_pub_key: [u8; 48],
    pub stake_amount: u128,
    pub inauguration_era: u64,
}

impl StakeInfoObject {
    pub fn new(stake_info: &reader::StakeInfo) -> Self {
        let mut identity = vec![stake_info.identity().flag()];
        identity.append(&mut stake_info.identity().content());
		Self {
			identity: identity.try_into().unwrap(),
			l2_address: stake_info.l2_address().try_into().unwrap(),
			bls_pub_key: stake_info.bls_pub_key().try_into().unwrap(),
			stake_amount: bytes_to_u128(&stake_info.stake_amount()),
			inauguration_era: bytes_to_u64(&stake_info.inauguration_era()),
		}
    }
}

impl Ord for StakeInfoObject {
    fn cmp(&self, other: &Self) -> Ordering {
		let mut order = other.stake_amount.cmp(&self.stake_amount);
		if let Ordering::Equal = order {
            order = other.identity.cmp(&self.identity);
			if let Ordering::Equal = order {
				order = other.inauguration_era.cmp(&self.inauguration_era);
			}
		}
		order
    }
}

pub fn get_stake_data_by_type_hash(
    cell_type_hash: &[u8; 32],
    source: Source,
) -> Result<reader::StakeLockCellData, Error> {
    let mut stake_data = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                assert!(stake_data.is_none());
                stake_data = {
                    let data = load_cell_data(i, source).unwrap();
                    let stake_data: reader::StakeLockCellData = Cursor::from(data).into();
                    Some(stake_data)
                };
            }
        });
    if stake_data.is_none() {
        return Err(Error::StakeDataEmpty);
    }
    Ok(stake_data.unwrap())
}

pub fn get_total_sudt_by_identity(
    stake_code_hash: &[u8; 32],
    sudt_type_hash: &Vec<u8>,
    identity: &[u8; 21],
    source: Source,
) -> Result<u128, Error> {
    let mut sudt = 0;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == sudt_type_hash.as_slice() {
                let lock = load_cell_lock(i, source).unwrap();
                if &lock.code_hash().unpack() == stake_code_hash {
                    // lock_args = admin_identity | checkpoint_type_hash | node_identity
                    let lock_args = lock.args().raw_data().to_vec();
                    if lock_args.len() < 74 {
                        return Err(Error::StakeATCellError);
                    }
                    if lock_args[53..] == identity[..] {
                        let data = load_cell_data(i, source).unwrap();
                        if data.len() < 16 {
                            return Err(Error::StakeATCellError);
                        }
                        sudt += bytes_to_u128(&data[..16].to_vec());
                    }
                }
            }
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sudt)
}

pub fn get_checkpoint_from_celldeps(
    checkpoint_type_hash: &Vec<u8>,
) -> Result<reader::CheckpointLockCellData, Error> {
    let mut checkpoint_data = None;
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == checkpoint_type_hash.as_slice() {
                assert!(checkpoint_data.is_none());
                checkpoint_data = {
                    let data = load_cell_data(i, Source::CellDep).unwrap();
                    let checkpoint_data: reader::CheckpointLockCellData = Cursor::from(data).into();
                    Some(checkpoint_data)
                };
            }
        });
    if checkpoint_data.is_none() {
        return Err(Error::CheckpointDataEmpty);
    }
    Ok(checkpoint_data.unwrap())
}

pub fn filter_stakeinfos(
    era: u64,
    quorum: u8,
    stake_infos: &BTreeSet<StakeInfoObject>,
    filter_type: FILTER,
) -> Result<BTreeSet<StakeInfoObject>, Error> {
    let mut filtered_stake_infos = BTreeSet::new();
    match filter_type {
        FILTER::APPLY => {
            let mut maximum_eras = BTreeMap::new();
            for stake_info in stake_infos {
                if stake_info.inauguration_era <= era {
                    let personal_max_era = maximum_eras.entry(&stake_info.identity).or_insert(0u64);
                    if stake_info.inauguration_era > *personal_max_era {
                        (*personal_max_era) = stake_info.inauguration_era;
                    }
                    if !filtered_stake_infos.insert(stake_info.clone()) {
                        return Err(Error::StakeInfoDumplicateError);
                    }
                }
            }
            filtered_stake_infos = filtered_stake_infos
                .into_iter()
                .filter(|info| {
                    if let Some(max_era) = maximum_eras.get(&info.identity) {
                        &info.inauguration_era == max_era
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>()[..quorum as usize]
                .to_vec()
                .into_iter()
                .collect();
        }
        FILTER::NOTAPPLY => {
            for stake_info in stake_infos {
                if stake_info.inauguration_era > era {
                    if !filtered_stake_infos.insert(stake_info.clone()) {
                        return Err(Error::StakeInfoDumplicateError);
                    }
                }
            }
            if filtered_stake_infos.len() as u8 >= quorum {
                return Err(Error::StakeInfoQuorumError);
            }
        }
    }
    Ok(filtered_stake_infos)
}

pub fn stakeinfos_into_set(
    stake_infos: &reader::StakeInfoVec,
) -> Result<BTreeSet<StakeInfoObject>, Error> {
    let mut btree_set = BTreeSet::new();
    for i in 0..stake_infos.len() {
        if btree_set.insert(StakeInfoObject::new(&stake_infos.get(i))) {
            return Err(Error::StakeInfoDumplicateError);
        }
    }
    Ok(btree_set)
}

pub fn calc_withdrawal_lock_hash(
    withdrawal_code_hash: &Vec<u8>,
    admin_identity: reader::Identity,
    checkpoint_type_hash: &Vec<u8>,
    node_identity: &[u8; 21],
) -> [u8; 32] {
    let node_identity = {
        let identity = writer::Identity::new_builder()
            .flag(node_identity[0].into())
            .content(writer::Byte20::new_unchecked(node_identity[1..20].into()))
            .build();
        writer::IdentityOpt::new_builder()
            .set(Some(identity))
            .build()
    };
    let admin_identity = writer::Identity::new_builder()
        .flag(admin_identity.flag().into())
        .content(writer::Byte20::new_unchecked(
            admin_identity.content().into(),
        ))
        .build();
    let withdrawal_lock_args = writer::WithdrawalLockArgs::new_builder()
        .admin_identity(admin_identity)
        .checkpoint_cell_type_hash(writer::Byte32::new_unchecked(
            checkpoint_type_hash.as_slice().into(),
        ))
        .node_identity(node_identity)
        .build();
    let withdrawal_lock = Script::new_builder()
        .code_hash(Byte32::new_unchecked(
            withdrawal_code_hash.as_slice().into(),
        ))
        .hash_type(ScriptHashType::Type.into())
        .args(withdrawal_lock_args.as_slice().pack())
        .build();
    let mut lock_hash = [0u8; 32];
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(withdrawal_lock.as_slice());
    blake2b.finalize(&mut lock_hash);
    lock_hash
}

pub fn get_withdrawal_total_sudt_amount(
    withdrawal_lock_hash: &[u8; 32],
    sudt_type_hash: &Vec<u8>,
    period: u64,
    source: Source,
) -> Result<u128, Error> {
    let mut total_sudt = 0;
    QueryIter::new(load_cell_lock_hash, source)
        .enumerate()
        .map(|(i, lock_hash)| {
            if &lock_hash == withdrawal_lock_hash {
                let type_hash = load_cell_type_hash(i, source).unwrap();
                if type_hash.unwrap_or([0u8; 32]) == sudt_type_hash.as_slice() {
                    let data = load_cell_data(i, source).unwrap();
                    if data.len() < 24 {
                        return Err(Error::WithdrawCellError);
                    }
                    if period > 0 && period != bytes_to_u64(&data[16..24].to_vec()) {
                        return Err(Error::WithdrawCellPeriodMismatch);
                    }
                    total_sudt += bytes_to_u128(&data[..16].to_vec());
                }
            }
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(total_sudt)
}

//////////////////////////////////////////////////////////
/// used by checkpoint contract
//////////////////////////////////////////////////////////

pub fn get_info_by_type_hash(
    type_hash: &Vec<u8>,
    source: Source,
) -> Result<(u64, reader::CheckpointLockCellData), Error> {
    let mut capacity = 0u64;
    let mut celldata = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, cell_type_hash)| {
            if cell_type_hash.unwrap_or([0u8; 32]) != type_hash[..] {
                return Ok(());
            }
            if celldata.is_some() {
                return Err(Error::CheckpointCellError);
            }
            capacity = load_cell_capacity(i, source).unwrap();
			celldata = {
				let data = load_cell_data(i, source).unwrap();
				Some(reader::CheckpointLockCellData::from(Cursor::from(data)))
			};
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if celldata.is_none() {
        return Err(Error::CheckpointCellError);
    }
    Ok((capacity, celldata.unwrap()))
}

pub fn get_sudt_by_type_hash(type_hash: &Vec<u8>, source: Source) -> Result<u128, Error> {
    let mut sudt = 0u128;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, cell_type_hash)| {
            if cell_type_hash.unwrap_or([0u8; 32]) == type_hash[..] {
                let data = load_cell_data(i, source).unwrap();
				if data.len() < 16 {
					return Err(Error::BadSudtDataFormat);
				}
				sudt += bytes_to_u128(&data[..16].to_vec());
            }
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sudt)
}

pub fn get_valid_stakeinfos_from_celldeps(era: u64, stake_type_hash: &Vec<u8>) -> Result<Vec<StakeInfoObject>, Error> {
	let mut stake_data = None;
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == stake_type_hash[..] {
				stake_data = {
					let data = load_cell_data(i, Source::CellDep).unwrap();
				 	Some(reader::StakeLockCellData::from(Cursor::from(data)))
				};
            }
        });
	if stake_data.is_none() {
		return Err(Error::StakeCellDepEmpty);
	}
	let stake_data = stake_data.unwrap();
	let mut valid_stakeinfos = {
		let quorum: u8 = stake_data.quorum_size().into();
		let stakeinfos_set = stakeinfos_into_set(&stake_data.stake_infos())?;
		filter_stakeinfos(era, quorum, &stakeinfos_set, FILTER::APPLY)?
			.into_iter()
			.collect::<Vec<_>>()
	};
	valid_stakeinfos.sort_by(|a, b| a.l2_address.cmp(&b.l2_address));
	Ok(valid_stakeinfos)
}
