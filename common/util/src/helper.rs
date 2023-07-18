extern crate alloc;

use crate::{error::Error, smt::LockInfo};
use alloc::vec::Vec;
use axon_types::{
    basic, checkpoint_reader,
    delegate_reader::{self, DelegateInfoDelta, DelegateSmtCellData},
    metadata_reader::{self, MetadataCellData, TypeIds},
    reward_reader::RewardSmtCellData,
    stake_reader::{self, StakeInfoDelta, StakeSmtCellData},
    withdraw, withdraw_reader, Cursor,
};
use blake2b_ref::Blake2bBuilder;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{
        core::ScriptHashType,
        packed::{Byte, Script},
        prelude::{Builder, Entity, Pack},
    },
    debug,
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock, load_cell_lock_hash,
        load_cell_type_hash, QueryIter,
    },
};
use core::{cmp::Ordering, result::Result};
use tiny_keccak::{Hasher, Keccak};

// #[derive(Clone, Copy, Default, Eq, PartialOrd, Debug)]
// pub struct DelegateInfoObject {
//     pub addr: [u8; 20],
//     pub amount: u128,
// }

// impl DelegateInfoObject {
//     pub fn new(delegate_info: &delegate_reader::DelegateInfo) -> Self {

//         let mut identity = [0u8; 20];
//         identity.copy_from_slice(delegate_info.addr());
//         Self {
//             addr: identity,
//             amount: bytes_to_u128(delegate_info.amount()),
//         }
//     }
// }

// impl Ord for DelegateInfoObject {
//     fn cmp(&self, other: &Self) -> Ordering {
//         let order = other.amount.cmp(&self.amount);
//         order
//     }
// }

// impl PartialEq for DelegateInfoObject {
//     fn eq(&self, other: &Self) -> bool {
//         self.addr == other.addr
//     }
// }

#[derive(Clone, Default, Eq, PartialOrd, Debug)]
pub struct MinerGroupInfoObject {
    pub staker: [u8; 20],
    pub stake_amount: u128,
    pub delegators: Vec<LockInfo>,
    pub delegator_epoch_proof: Vec<u8>,
}

impl MinerGroupInfoObject {
    pub fn new(miner_group_info: &metadata_reader::MinerGroupInfo) -> Self {
        let mut delegators = Vec::new();
        let delegator_infos = miner_group_info.delegate_infos();
        for i in 0..delegator_infos.len() {
            let delegator_info = &delegator_infos.get(i);
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&delegator_info.addr());
            let delegate_info_obj = LockInfo {
                addr: addr,
                amount: bytes_to_u128(&delegator_info.amount()),
            };
            delegators.push(delegate_info_obj);
        }

        let mut staker = [0u8; 20];
        staker.copy_from_slice(&miner_group_info.staker());
        Self {
            staker: staker,
            stake_amount: bytes_to_u128(&miner_group_info.amount()),
            delegators: delegators,
            delegator_epoch_proof: miner_group_info.delegate_epoch_proof(),
        }
    }

    pub fn get_total_amount(&self) -> u128 {
        let mut total_amount = self.stake_amount;
        for delegate_info in self.delegators.iter() {
            total_amount += delegate_info.amount;
        }
        total_amount
    }
}

impl Ord for MinerGroupInfoObject {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = other.get_total_amount().cmp(&self.get_total_amount());
        order
    }
}

impl PartialEq for MinerGroupInfoObject {
    fn eq(&self, other: &Self) -> bool {
        self.staker == other.staker
    }
}

#[derive(Clone, Default, Debug)]
pub struct ProposeCountObject {
    pub addr: [u8; 20],
    pub count: u64,
}

pub fn find_script_input(script: &Script) -> bool {
    let script_hash = calc_script_hash(&script).to_vec();
    debug!("script_hash = {:?}", script_hash);
    let input_count = get_cell_count_by_type_hash(&script_hash, Source::Input);
    input_count > 0
}

pub fn calc_script_hash(script: &Script) -> [u8; 32] {
    let mut hash = [0; 32];
    let mut blake2b = blake2b_ref::Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(script.as_slice());
    blake2b.finalize(&mut hash);
    hash
}

pub fn check_xudt_type_hash(xudt_type_hash: &Vec<u8>) -> Result<(), Error> {
    // extract AT type_hash from type_script
    let type_hash = {
        let type_hash = load_cell_type_hash(0, Source::GroupInput)?;
        if type_hash.is_none() {
            return Err(Error::TypeScriptEmpty);
        }
        type_hash.unwrap()
    };

    debug!(
        "type_hash: {:?}, xudt_type_hash: {:?}",
        type_hash, xudt_type_hash
    );
    if type_hash.to_vec() != *xudt_type_hash {
        return Err(Error::MismatchXudtTypeId);
    }

    Ok(())
}

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

// needs refinement
pub fn bytes_to_h256(bytes: &Vec<u8>) -> [u8; 32] {
    let mut h256 = [0u8; 32];
    h256.copy_from_slice(bytes);
    h256
}

pub fn get_script_hash(code_hash: &Vec<u8>, args: &Vec<u8>) -> [u8; 32] {
    let code_hash: [u8; 32] = code_hash.as_slice().try_into().unwrap();
    let script = Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(args.pack())
        .build();
    calc_script_hash(&script)
}

pub fn get_script_hash_with_type(
    code_hash: &Vec<u8>,
    hash_type: ScriptHashType,
    args: &Vec<u8>,
) -> [u8; 32] {
    let code_hash: [u8; 32] = code_hash.as_slice().try_into().unwrap();
    let script = Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(hash_type.into())
        .args(args.pack())
        .build();
    calc_script_hash(&script)
}

pub fn get_checkpoint_from_celldeps(
    checkpoint_type_hash: &Vec<u8>,
) -> Result<checkpoint_reader::CheckpointCellData, Error> {
    let mut checkpoint_data = None;
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == checkpoint_type_hash.as_slice() {
                // debug!("checkpoint type hash: {:?}", checkpoint_type_hash);
                assert!(checkpoint_data.is_none());
                checkpoint_data = {
                    // debug!("checkpoint data index: {}", i);
                    let data = load_cell_data(i, Source::CellDep);
                    match data {
                        Ok(data) => {
                            debug!("checkpoint data len: {}", data.len());
                            let checkpoint_data: checkpoint_reader::CheckpointCellData =
                                Cursor::from(data).into();
                            Some(checkpoint_data)
                        }
                        Err(_err) => {
                            debug!("checkpoint data error: {:?}", _err);
                            None
                        }
                    }
                };
            }
        });

    match checkpoint_data {
        Some(checkpoint_data) => Ok(checkpoint_data),
        None => Err(Error::CheckpointDataEmpty),
    }
}

pub fn get_current_epoch(checkpoint_type_id: &Vec<u8>) -> Result<u64, Error> {
    debug!(
        "get_current_epoch checkpoint_type_id: {:?}",
        checkpoint_type_id
    );
    let checkpoint_data = get_checkpoint_from_celldeps(checkpoint_type_id)?;
    // debug!(
    //     "checkpoint_data: period len{},, epoch:{}",
    //     checkpoint_data.period(),
    //     checkpoint_data.epoch()
    // );
    Ok(checkpoint_data.epoch())
}

pub fn get_xudt_by_type_hash(type_hash: &Vec<u8>, source: Source) -> Result<u128, Error> {
    let mut sudt = 0u128;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, cell_type_hash)| {
            if cell_type_hash.unwrap_or([0u8; 32]) == type_hash[..] {
                let data = load_cell_data(i, source).unwrap();
                // debug!("sudt cell data len: {}", data.len());
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

pub fn get_stake_at_data_by_lock_hash(
    cell_lock_hash: &[u8; 32],
    source: Source,
) -> Result<(u128, stake_reader::StakeAtCellLockData), Error> {
    let mut sudt = None;
    let mut stake_at_data = None;
    QueryIter::new(load_cell_lock_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            // debug!("get_stake_at_data_by_lock_hash lock_hash: {:?}", lock_hash);
            if lock_hash == cell_lock_hash[..] {
                let data = load_cell_data(i, source).unwrap();
                debug!("get_stake_at_data_by_lock_hash data len:{}", data.len());
                if data.len() >= 16 {
                    sudt = Some(bytes_to_u128(&data[..16].to_vec()));
                    // debug!("get_stake_at_data_by_lock_hash data sudt:{:?}", sudt);
                    assert!(stake_at_data.is_none());
                    stake_at_data = {
                        let stake_data: stake_reader::StakeAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        Some(stake_data.lock())
                    };
                }
            }
        });
    if sudt.is_none() {
        return Err(Error::BadSudtDataFormat);
    }
    if stake_at_data.is_none() {
        return Err(Error::StakeDataEmpty);
    }
    Ok((sudt.unwrap(), stake_at_data.unwrap()))
}

pub fn get_delegate_at_data_by_lock_hash(
    cell_lock_hash: &[u8; 32],
    source: Source,
) -> Result<(u128, delegate_reader::DelegateAtCellLockData), Error> {
    let mut sudt = None;
    let mut delegate_at_data = None;
    QueryIter::new(load_cell_lock_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if lock_hash == cell_lock_hash[..] {
                let data = load_cell_data(i, source).unwrap();
                if data.len() >= 16 {
                    sudt = Some(bytes_to_u128(&data[..16].to_vec()));
                    assert!(delegate_at_data.is_none());
                    delegate_at_data = {
                        let delegate_data: delegate_reader::DelegateAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        Some(delegate_data.lock())
                    };
                }
            }
        });
    if sudt.is_none() {
        return Err(Error::BadSudtDataFormat);
    }
    if delegate_at_data.is_none() {
        return Err(Error::StakeDataEmpty);
    }
    Ok((sudt.unwrap(), delegate_at_data.unwrap()))
}

pub fn get_withdraw_at_data_by_lock_hash(
    cell_lock_hash: &[u8; 32],
    source: Source,
) -> Result<(u128, withdraw_reader::WithdrawAtCellData), Error> {
    let mut sudt = None;
    let mut withdraw_at_data = None;
    QueryIter::new(load_cell_lock_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if lock_hash == cell_lock_hash[..] {
                let data = load_cell_data(i, source).unwrap();
                if data.len() >= 16 {
                    sudt = Some(bytes_to_u128(&data[..16].to_vec()));
                    assert!(withdraw_at_data.is_none());
                    withdraw_at_data = {
                        let withdraw_data: withdraw_reader::WithdrawAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        Some(withdraw_data)
                    };
                }
            }
        });
    if sudt.is_none() {
        return Err(Error::WithdrawBadSudtDataFormat);
    }
    if withdraw_at_data.is_none() {
        return Err(Error::WithdrawDataEmpty);
    }
    Ok((sudt.unwrap(), withdraw_at_data.unwrap()))
}

pub fn get_delegate_delta(
    staker: &Vec<u8>,
    cell_lock_hash: &[u8; 32],
    source: Source,
) -> Result<delegate_reader::DelegateInfoDelta, Error> {
    let (_, delegate_at_data) = get_delegate_at_data_by_lock_hash(cell_lock_hash, source)?;
    let delegate_info_deltas = delegate_at_data.delegator_infos();
    for i in 0..delegate_info_deltas.len() {
        let delegate_info_delta = delegate_info_deltas.get(i);
        if delegate_info_delta.staker() == *staker {
            return Ok(delegate_info_delta);
        }
    }

    return Err(Error::StakeDataEmpty);
}

pub fn get_stake_update_infos(
    cell_type_hash: &[u8; 32],
    stake_at_code_hash: &Vec<u8>,
    source: Source,
) -> Result<Vec<([u8; 20], [u8; 32], StakeInfoDelta)>, Error> {
    let mut stake_update_infos = Vec::<([u8; 20], [u8; 32], StakeInfoDelta)>::default();
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                let lock_script = load_cell_lock(i, source).unwrap();
                let lock_script_code_hash = lock_script.code_hash();
                // debug!(
                //     "i:{}, lock_script_code_hash:{:?}, stake_at_code_hash:{:?}",
                //     i,
                //     lock_script_code_hash.as_slice(),
                //     stake_at_code_hash
                // );
                if lock_script_code_hash.as_slice() == stake_at_code_hash {
                    let lock_hash = load_cell_lock_hash(i, source).unwrap();
                    let data = load_cell_data(i, source).unwrap();
                    let stake_xudt_lock = {
                        let stake_data: stake_reader::StakeAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        stake_data.lock()
                    };
                    let stake_info_delta = stake_xudt_lock.delta();
                    let address: [u8; 20] =
                        stake_xudt_lock.l2_address().as_slice().try_into().unwrap();
                    stake_update_infos.push((address, lock_hash, stake_info_delta));
                }
            }
        });

    Ok(stake_update_infos)
}

pub fn get_delegate_update_infos(
    staker: &Vec<u8>,
    cell_type_hash: &[u8; 32],
    source: Source,
) -> Result<Vec<([u8; 20], [u8; 32], DelegateInfoDelta)>, Error> {
    let mut delegate_update_infos = Vec::<([u8; 20], [u8; 32], DelegateInfoDelta)>::default();
    debug!(
        "get_delegate_update_infos staker: {:?}, cell_type_hash: {:?}",
        staker, cell_type_hash
    );
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                let lock_hash = load_cell_lock_hash(i, source).unwrap();
                let data = load_cell_data(i, source).unwrap();
                let delegate_at_data = {
                    let delegate_data: delegate_reader::DelegateAtCellData =
                        Cursor::from(data[16..].to_vec()).into();
                    delegate_data.lock()
                };
                let delegate_infos = delegate_at_data.delegator_infos();
                for i in 0..delegate_infos.len() {
                    let delegate_info = delegate_infos.get(i);
                    if delegate_info.staker() == *staker {
                        let address: [u8; 20] =
                            delegate_at_data.l2_address().as_slice().try_into().unwrap();
                        // debug!("delegate_info.staker: {:?}, amount: {}", delegate_info.staker(), delegate_info.amount());
                        delegate_update_infos.push((address, lock_hash, delegate_info));
                        break;
                    }
                }
            }
        });

    debug!("delegate_update_infos len: {}", delegate_update_infos.len());
    Ok(delegate_update_infos)
}

pub fn get_cell_count(type_id: &Vec<u8>, source: Source) -> u8 {
    let mut cells_count = 0u8;
    QueryIter::new(load_cell_lock_hash, source).for_each(|lock_hash| {
        if &lock_hash == type_id.as_slice() {
            cells_count += 1;
        }
    });
    cells_count
}

pub fn get_cell_count_by_type_hash(cell_type_hash: &Vec<u8>, source: Source) -> u8 {
    let mut cells_count = 0u8;
    QueryIter::new(load_cell_type_hash, source).for_each(|type_hash| match type_hash {
        Some(type_hash) => {
            if &type_hash == cell_type_hash.as_slice() {
                cells_count += 1;
            }
        }
        None => {}
    });
    cells_count
}
//////////////////////////////////////////////////////////
/// used by checkpoint contract
//////////////////////////////////////////////////////////

pub fn get_checkpoint_by_type_id(
    type_hash: &Vec<u8>,
    source: Source,
) -> Result<(u64, checkpoint_reader::CheckpointCellData), Error> {
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
                Some(checkpoint_reader::CheckpointCellData::from(Cursor::from(
                    data,
                )))
            };
            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;

    match celldata {
        Some(celldata) => Ok((capacity, celldata)),
        None => Err(Error::CheckpointCellError),
    }
}

pub fn get_valid_stakeinfos_from_celldeps(
    _epoch: u64,
    metadata_type_id: &Vec<u8>,
) -> Result<Vec<LockInfo>, Error> {
    let stakers: Vec<LockInfo> = Vec::new();
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .for_each(|(_i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == metadata_type_id[..] {
                // get metadata data, and parse validators
            }
        });

    Ok(stakers)
}

pub fn get_metada_data_by_type_id(
    cell_type_id: &[u8; 32],
    source: Source,
) -> Result<MetadataCellData, Error> {
    let mut metadata: Option<MetadataCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == cell_type_id {
                // debug!("get_metada_data_by_type_id index: {}", i);
                let data = load_cell_data(i, source).unwrap();
                // debug!("get_metada_data_by_type_id index: {}", i);
                metadata = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

    match metadata {
        Some(metadata) => Ok(metadata),
        None => Err(Error::MetadataNotFound),
    }
}

pub fn get_type_ids(metadata_type_id: &[u8; 32], source: Source) -> Result<TypeIds, Error> {
    let metadata = get_metada_data_by_type_id(metadata_type_id, source)?;
    Ok(metadata.type_ids())
}

pub fn get_current_validators(
    cell_type_id: &[u8; 32],
    source: Source,
) -> Result<Vec<[u8; 48]>, Error> {
    let mut metadata: Option<MetadataCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_id {
                let data = load_cell_data(i, source).unwrap();
                metadata = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

    let mut bls_pub_keys: Vec<[u8; 48]> = Vec::new();
    let validator_list = metadata.unwrap().metadata().get(0).validators(); // validator of current is in index 0
    for i in 0..validator_list.len() {
        let validator = validator_list.get(i);
        let bls_pub_key: [u8; 48] = validator.bls_pub_key().as_slice().try_into().unwrap();
        bls_pub_keys.push(bls_pub_key);
    }
    Ok(bls_pub_keys)
}

pub fn get_epoch_len(metadata_type_id: &[u8; 32], source: Source) -> Result<u32, Error> {
    let metadata = get_metada_data_by_type_id(metadata_type_id, source)?;
    let metadata_list = metadata.metadata();
    let metadata0 = metadata_list.get(0);
    Ok(metadata0.epoch_len())
}

pub fn get_quorum_size(metadata_type_id: &[u8; 32], source: Source) -> Result<u16, Error> {
    let metadata = get_metada_data_by_type_id(metadata_type_id, source)?;
    let metadata_list = metadata.metadata();
    let metadata = metadata_list.get(0); // index 0 is metadata of current epoch
    let quorum_size = metadata.quorum();
    Ok(quorum_size)
}

pub fn get_stake_smt_data(type_id: &[u8; 32], source: Source) -> Result<StakeSmtCellData, Error> {
    let mut stake_smt_data: Option<StakeSmtCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == type_id {
                debug!(
                    "get_stake_smt_data index: {}, type_hash: {:?}",
                    i, type_hash
                );
                let data = load_cell_data(i, source).unwrap();
                debug!("get_stake_smt_data data len: {}", data.len());
                stake_smt_data = Some(Cursor::from(data[..].to_vec()).into());
            }
        });
    debug!("get_stake_smt_data ok");
    Ok(stake_smt_data.unwrap())
}

pub fn get_stake_smt_root(typd_id: &[u8; 32], source: Source) -> Result<[u8; 32], Error> {
    let stake_smt_data = get_stake_smt_data(typd_id, source)?;
    Ok(stake_smt_data.smt_root().as_slice().try_into().unwrap())
}

pub fn get_delegate_smt_root(
    typd_id: &[u8; 32],
    addr: &[u8; 20],
    source: Source,
) -> Result<[u8; 32], Error> {
    let mut delegate_smt_data: Option<DelegateSmtCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == typd_id {
                let data = load_cell_data(i, source).unwrap();
                delegate_smt_data = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

    let smt_roots = delegate_smt_data.unwrap().smt_roots();
    for i in 0..smt_roots.len() {
        let smt_root = smt_roots.get(i);
        if smt_root.staker() == addr {
            return Ok(smt_root.root().as_slice().try_into().unwrap());
        }
    }

    Err(Error::StakerNotFound)
}

pub fn get_delegate_smt_root_from_cell_data(
    addr: &[u8; 20],
    smt_data: &DelegateSmtCellData,
) -> Result<[u8; 32], Error> {
    let smt_roots = smt_data.smt_roots();
    for i in 0..smt_roots.len() {
        let smt_root = smt_roots.get(i);
        if smt_root.staker() == addr {
            return Ok(smt_root.root().as_slice().try_into().unwrap());
        }
    }

    Err(Error::StakerNotFound)
}

pub fn get_delegate_smt_data(
    typd_id: &[u8; 32],
    source: Source,
) -> Result<DelegateSmtCellData, Error> {
    let mut delegate_smt_data: Option<DelegateSmtCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == typd_id {
                let data = load_cell_data(i, source).unwrap();
                delegate_smt_data = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

    Ok(delegate_smt_data.unwrap())
}

pub fn get_reward_smt_data(type_id: &[u8; 32], source: Source) -> Result<RewardSmtCellData, Error> {
    let mut reward_smt_data: Option<RewardSmtCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == type_id {
                // debug!(
                //     "get_reward_smt_data index: {}, type_hash: {:?}",
                //     i, type_hash
                // );
                let data = load_cell_data(i, source).unwrap();
                // debug!("get_reward_smt_data data len: {}", data.len());
                reward_smt_data = Some(Cursor::from(data[..].to_vec()).into());
            }
        });
    // debug!("get_reward_smt_data ok");
    Ok(reward_smt_data.unwrap())
}

pub fn axon_byte32(bytes: &[u8]) -> basic::Byte32 {
    basic::Byte32::new_unchecked(bytes.to_vec().into())
}
pub fn axon_identity(addr: &[u8; 20]) -> basic::Identity {
    // convert [u8; 20] to [Byte; 20]
    let mut new_addr = [Byte::new(0); 20];
    for i in 0..20 {
        new_addr[i] = Byte::new(addr[i].into());
    }

    basic::Identity::new_builder().set(new_addr).build()
}

pub fn calc_withdrawal_lock_hash(
    withdraw_code_hash: &Vec<u8>,
    addr: &[u8; 20],
    metadata_type_id: &[u8; 32],
) -> [u8; 32] {
    let withdraw_lock_args = {
        withdraw::WithdrawArgs::new_builder()
            .metadata_type_id(axon_byte32(metadata_type_id))
            .addr(axon_identity(addr))
            .build()
    };
    let withdraw_lock = {
        let code_hash: [u8; 32] = withdraw_code_hash.clone().try_into().unwrap();
        Script::new_builder()
            .code_hash(code_hash.pack())
            .hash_type(ScriptHashType::Data1.into())
            .args(withdraw_lock_args.as_slice().pack())
            .build()
    };
    let mut lock_hash = [0u8; 32];
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(withdraw_lock.as_slice());
    blake2b.finalize(&mut lock_hash);
    lock_hash
}

pub fn pubkey_to_eth_addr(pubkey: &Vec<u8>) -> [u8; 20] {
    let mut keccak = Keccak::v256();
    let input = pubkey.as_slice();
    keccak.update(input);
    let mut output = [0; 32];
    keccak.finalize(&mut output);
    let pubkey_hash = output[12..].to_vec();
    pubkey_hash.try_into().unwrap()
}

pub fn keccak256(data: &Vec<u8>) -> [u8; 32] {
    let mut keccak = Keccak::v256();
    let input = data.as_slice();
    keccak.update(input);
    let mut output = [0; 32];
    keccak.finalize(&mut output);
    output
}
