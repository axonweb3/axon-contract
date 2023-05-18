extern crate alloc;

use crate::error::Error;
use alloc::{collections::BTreeSet, vec::Vec};
use axon_types::{
    checkpoint_reader,
    delegate_reader::{self, DelegateInfoDelta, DelegateSmtCellData},
    metadata_reader::{self, TypeIds, MetadataCellData},
    stake_reader::{self, StakeInfoDelta, StakeInfos, StakeSmtCellData},
    withdraw_reader, Cursor,
};
use ckb_smt::smt::{Pair, Tree};
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock_hash, load_cell_type_hash, QueryIter,
    },
};
use core::{cmp::Ordering, result::Result};

#[derive(Clone, Default, Eq, PartialOrd, Debug)]
pub struct StakeInfoObject {
    pub identity: [u8; 20],
    pub stake_amount: u128,
}

impl StakeInfoObject {
    pub fn new(stake_info: &stake_reader::StakeInfo) -> Self {
        let mut identity = [0u8; 20];
        identity.copy_from_slice(&stake_info.addr());
        Self {
            identity: identity,
            stake_amount: bytes_to_u128(&stake_info.amount()),
        }
    }
}

impl Ord for StakeInfoObject {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = other.stake_amount.cmp(&self.stake_amount);
        order
    }
}

impl PartialEq for StakeInfoObject {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

#[derive(Clone, Copy, Default, Eq, PartialOrd, Debug)]
pub struct DelegateInfoObject {
    pub addr: [u8; 20],
    pub amount: u128,
}

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

impl Ord for DelegateInfoObject {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = other.amount.cmp(&self.amount);
        order
    }
}

impl PartialEq for DelegateInfoObject {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

#[derive(Clone, Default, Eq, PartialOrd, Debug)]
pub struct MinerGroupInfoObject {
    pub staker: [u8; 20],
    pub stake_amount: Option<u128>,
    pub delegators: Vec<DelegateInfoObject>,
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
            let delegate_info_obj = DelegateInfoObject {
                addr: addr,
                amount: bytes_to_u128(&delegator_info.amount()),
            };
            delegators.push(delegate_info_obj);
        }

        let mut staker = [0u8; 20];
        staker.copy_from_slice(&miner_group_info.staker());
        Self {
            staker: staker,
            stake_amount: Some(bytes_to_u128(&miner_group_info.amount().unwrap())),
            delegators: delegators,
            delegator_epoch_proof: miner_group_info.delegate_epoch_proof(),
        }
    }

    pub fn get_total_amount(&self) -> u128 {
        let mut total_amount = self.stake_amount.unwrap();
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
    pub identity: [u8; 20],
    pub count: u32,
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

pub fn get_checkpoint_from_celldeps(
    checkpoint_type_hash: &Vec<u8>,
) -> Result<checkpoint_reader::CheckpointCellData, Error> {
    let mut checkpoint_data = None;
    QueryIter::new(load_cell_type_hash, Source::CellDep)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if type_hash.unwrap_or([0u8; 32]) == checkpoint_type_hash.as_slice() {
                debug!("checkpoint type hash: {:?}", checkpoint_type_hash);
                assert!(checkpoint_data.is_none());
                checkpoint_data = {
                    debug!("checkpoint data index: {}", i);
                    let data = load_cell_data(i, Source::CellDep);
                    match data {
                        Ok(data) => {
                            debug!("checkpoint data len: {}", data.len());
                            let checkpoint_data: checkpoint_reader::CheckpointCellData =
                                Cursor::from(data).into();
                            Some(checkpoint_data)
                        },
                        Err(err) => {
                            debug!("checkpoint data error: {:?}", err);
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
    debug!("get_current_epoch checkpoint_type_id: {:?}", checkpoint_type_id);
    let checkpoint_data = get_checkpoint_from_celldeps(checkpoint_type_id)?;
    Ok(checkpoint_data.epoch())
}

pub fn get_xudt_by_type_hash(type_hash: &Vec<u8>, source: Source) -> Result<u128, Error> {
    let mut sudt = 0u128;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .map(|(i, cell_type_hash)| {
            if cell_type_hash.unwrap_or([0u8; 32]) == type_hash[..] {
                let data = load_cell_data(i, source).unwrap();
                debug!("sudt cell data len: {}", data.len());
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
) -> Result<(u128, stake_reader::StakeAtCellData), Error> {
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
                    assert!(stake_at_data.is_none());
                    stake_at_data = {
                        let stake_data: stake_reader::StakeAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        Some(stake_data)
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
) -> Result<(u128, delegate_reader::DelegateAtCellData), Error> {
    let mut sudt = None;
    let mut delegate_at_data = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == cell_lock_hash {
                let data = load_cell_data(i, source).unwrap();
                if data.len() >= 16 {
                    sudt = Some(bytes_to_u128(&data[..16].to_vec()));
                    assert!(delegate_at_data.is_none());
                    delegate_at_data = {
                        let delegate_data: delegate_reader::DelegateAtCellData =
                            Cursor::from(data[16..].to_vec()).into();
                        Some(delegate_data)
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
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == cell_lock_hash {
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
        return Err(Error::BadSudtDataFormat);
    }
    if withdraw_at_data.is_none() {
        return Err(Error::StakeDataEmpty);
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
    source: Source,
) -> Result<Vec<([u8; 20], [u8; 32], StakeInfoDelta)>, Error> {
    let mut stake_update_infos = Vec::<([u8; 20], [u8; 32], StakeInfoDelta)>::default();
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                let lock_hash = load_cell_lock_hash(i, source).unwrap();
                let data = load_cell_data(i, source).unwrap();
                let stake_at_data = {
                    let stake_data: stake_reader::StakeAtCellData =
                        Cursor::from(data[16..].to_vec()).into();
                    stake_data
                };
                let stake_info_delta = stake_at_data.delta();
                // get address from lock script args
                let address: [u8; 20] = stake_at_data.l1_address().as_slice().try_into().unwrap();
                stake_update_infos.push((address, lock_hash, stake_info_delta));
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
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, type_hash)| {
            if &type_hash.unwrap_or([0u8; 32]) == cell_type_hash {
                let lock_hash = load_cell_lock_hash(i, source).unwrap();
                let data = load_cell_data(i, source).unwrap();
                let delegate_at_data = {
                    let delegate_data: delegate_reader::DelegateAtCellData =
                        Cursor::from(data[16..].to_vec()).into();
                    delegate_data
                };
                let delegate_infos = delegate_at_data.delegator_infos();
                for i in 0..delegate_infos.len() {
                    let delegate_info = delegate_infos.get(i);
                    if delegate_info.staker() == *staker {
                        let address: [u8; 20] =
                            delegate_at_data.l1_address().as_slice().try_into().unwrap();
                        delegate_update_infos.push((address, lock_hash, delegate_info));
                        break;
                    }
                }
            }
        });

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
    if celldata.is_none() {
        return Err(Error::CheckpointCellError);
    }
    Ok((capacity, celldata.unwrap()))
}

pub fn get_valid_stakeinfos_from_celldeps(
    _epoch: u64,
    metadata_type_id: &Vec<u8>,
) -> Result<Vec<StakeInfoObject>, Error> {
    let stakers: Vec<StakeInfoObject> = Vec::new();
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
                let data = load_cell_data(i, source).unwrap();
                metadata = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

    if metadata.is_none() {
        Err(Error::MetadataNotFound)
    } else {
        Ok(metadata.unwrap())        
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
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == cell_type_id {
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

pub fn get_stake_smt_data(typd_id: &[u8; 32], source: Source) -> Result<StakeSmtCellData, Error> {
    let mut stake_smt_data: Option<StakeSmtCellData> = None;
    QueryIter::new(load_cell_type_hash, source)
        .enumerate()
        .for_each(|(i, lock_hash)| {
            if &lock_hash.unwrap_or([0u8; 32]) == typd_id {
                let data = load_cell_data(i, source).unwrap();
                stake_smt_data = Some(Cursor::from(data[..].to_vec()).into());
            }
        });

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

pub fn transform_to_set(stake_infos: &StakeInfos) -> BTreeSet<StakeInfoObject> {
    let mut stake_infos_set = BTreeSet::new();
    for i in 0..stake_infos.len() {
        let stake_info = &stake_infos.get(i);
        let stake_info_obj = StakeInfoObject::new(stake_info);
        stake_infos_set.insert(stake_info_obj);
    }
    stake_infos_set
}

pub fn verify_2layer_smt_stake(
    stake_infos: &BTreeSet<StakeInfoObject>,
    epoch: u64,
    epoch_proof: &Vec<u8>,
    epoch_root: &[u8; 32],
) -> Result<(), Error> {
    // construct old stake smt root & verify
    let mut tree_buf = [Pair::default(); 100];
    let mut tree = Tree::new(&mut tree_buf);
    stake_infos.iter().for_each(|stake_info| {
        let _ = tree
            .update(
                &bytes_to_h256(&stake_info.identity.to_vec()),
                &bytes_to_h256(&stake_info.stake_amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    let proof = [0u8; 32];
    let stake_root = tree.calculate_root(&proof)?; // epoch smt value

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(&bytes_to_h256(&epoch.to_le_bytes().to_vec()), &stake_root)
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;
    epoch_tree
        .verify(&epoch_root, &epoch_proof)
        .map_err(|err| {
            debug!("verify top smt error: {}", err);
            Error::OldStakeInfosErr
        })?;
    Ok(())
}

pub fn verify_2layer_smt_delegate(
    delegate_infos: &BTreeSet<DelegateInfoObject>,
    epoch: u64,
    epoch_proof: &Vec<u8>,
    epoch_root: &[u8; 32],
) -> Result<(), Error> {
    // construct old stake smt root & verify
    let mut tree_buf = [Pair::default(); 100];
    let mut tree = Tree::new(&mut tree_buf);
    delegate_infos.iter().for_each(|stake_info| {
        let _ = tree
            .update(
                &bytes_to_h256(&stake_info.addr.to_vec()),
                &bytes_to_h256(&stake_info.amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    let proof = [0u8; 32];
    let stake_root = tree.calculate_root(&proof)?; // epoch smt value

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(&bytes_to_h256(&epoch.to_le_bytes().to_vec()), &stake_root)
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;
    epoch_tree
        .verify(&epoch_root, &epoch_proof)
        .map_err(|err| {
            debug!("verify top smt error: {}", err);
            Error::OldStakeInfosErr
        })?;
    Ok(())
}

// pub fn verify_2layer_smt(stake_proof: MerkleProof, stake_root: H256, staker_identity: Vec<u8>, old_stake: u128,
//                          epoch_proof: MerkleProof, epoch_root: H256, epoch: u64) -> Result<(), Error> {
//     if verify_smt(stake_proof, &stake_root, staker_identity.to_h256(), old_stake.to_h256()) {
//         return Err(Error::IllegalInputStakeInfo);
//     }

//     if verify_smt(epoch_proof, &epoch_root, epoch.to_h256(), stake_root) {
//         Err(Error::IllegalInputStakeInfo)
//     } else {
//         Ok(())
//     }
// }

// pub fn verify_smt(proof: MerkleProof, root: &H256, key: H256, value: H256) -> bool {
//     let leaves = vec![(key, value)];
//     proof.verify::<Blake2bHasher>(root, leaves).unwrap()
// }
