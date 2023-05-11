// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec::Vec;
use alloc::{collections::BTreeSet, vec};
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_smt::smt::{Pair, Tree};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_cell_type_hash, load_script, load_witness_args},
};

use axon_types::{reward_reader, Cursor};
use util::{error::Error, helper::*};

#[derive(Clone, Copy, Default)]
struct RewardStakeInfoObject {
    staker: [u8; 20],
    propose_count: u64,
    stake_amount: u128,
}

#[derive(Clone, Default)]
struct EpochRewardStakeInfoObject {
    stake_infos: Vec<RewardStakeInfoObject>,
    count_proof: Vec<u8>,       // bottom propose count smt proof
    count_root: [u8; 32],       // smt root of bottom propose count smt, optimize
    count_epoch_proof: Vec<u8>, // smt proof of top propose count smt
    amount_proof: Vec<u8>,      // bottom stake amount smt proof
    amount_root: [u8; 32],
    amount_epoch_proof: Vec<u8>,
}

#[derive(Clone, Default)]
struct RewardObject {
    staker: [u8; 20],
    stake_amount: u128,
    delegate_amount: Option<u128>,
    total_delegate_amount: u128,
    propose_count: u64,
}

#[derive(Clone, Default)]
struct EpochRewardObject {
    miner: [u8; 20],
    reward_objs: Vec<RewardObject>,
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    let reward_args: reward_reader::RewardArgs = Cursor::from(args.to_vec()).into();
    let metadata_type_id = reward_args.metadata_type_id();

    let metadata_type_ids = get_type_ids(
        &metadata_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    if metadata_type_id != metadata_type_ids.metadata_type_id() {
        return Err(Error::MisMatchMetadataTypeId);
    }

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    let reward_witness = {
        let witness_lock = witness_args.lock().to_opt();
        if witness_lock.is_none() {
            return Err(Error::WitnessLockError);
        }
        let value: reward_reader::RewardWitness =
            Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
        value
    };

    let miner = reward_witness.miner();
    let no_claim_proof = reward_witness.old_not_claim_info();
    let epoch = no_claim_proof.epoch();
    let old_claim_root = [0u8; 32];
    verify_claim(
        &old_claim_root,
        &miner.as_slice().try_into().unwrap(),
        epoch,
        &no_claim_proof.proof(),
    )?;

    let mut reward_amount = 0u128;
    let reward_infos = reward_witness.reward_infos();
    let delegate_smt_data = get_delegate_smt_data(
        metadata_type_ids
            .delegate_smt_type_id()
            .as_slice()
            .try_into()
            .unwrap(),
        Source::CellDep,
    )?;
    for i in 0..reward_infos.len() {
        let mut epoch_reward_obj = EpochRewardObject::default(); // used to calculate reward
        epoch_reward_obj.miner = miner.as_slice().try_into().unwrap();

        let mut epoch_reward_stake_info_obj = EpochRewardStakeInfoObject::default();
        let mut stake_info_objs = Vec::new();

        let epoch_reward_info = reward_infos.get(i);
        let staker_infos = epoch_reward_info.reward_stake_infos();
        // get one staker's propose count, stake amount, verify its delegate info
        for i in 0..staker_infos.len() {
            let mut reward_obj = RewardObject::default();
            let stake_info = staker_infos.get(i);
            let staker = stake_info.validator();
            let delegate_infos = stake_info.delegate_infos();
            let mut delegate_infos_set = BTreeSet::new();
            let mut total_delegate_amount = 0u128;
            for i in 0..delegate_infos.len() {
                let delegate_info = delegate_infos.get(i);
                let delegate_info_obj = DelegateInfoObject {
                    addr: delegate_info
                        .delegator_addr()
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    amount: bytes_to_u128(&delegate_info.amount()),
                };
                delegate_infos_set.insert(delegate_info_obj);
                total_delegate_amount += bytes_to_u128(&delegate_info.amount());
                if delegate_info.delegator_addr() == miner {
                    reward_obj.delegate_amount = Some(bytes_to_u128(&delegate_info.amount()));
                }
            }
            let delegate_epoch_proof = stake_info.delegate_epoch_proof();
            let delegate_epoch_root = [0u8; 32]; // shoule get from delegate smt cell
            verify_2layer_smt_delegate(
                &delegate_infos_set,
                epoch,
                &delegate_epoch_proof,
                &delegate_epoch_root,
            )?;

            let propose_count = stake_info.propose_count();
            let stake_amount = bytes_to_u128(&stake_info.staker_amount());

            let stake_info_obj = RewardStakeInfoObject {
                staker: staker.as_slice().try_into().unwrap(),
                propose_count: propose_count,
                stake_amount: stake_amount,
            };
            stake_info_objs.push(stake_info_obj);

            reward_obj.staker = staker.as_slice().try_into().unwrap();
            reward_obj.stake_amount = stake_amount;
            reward_obj.propose_count = propose_count;
            reward_obj.total_delegate_amount = total_delegate_amount;
            epoch_reward_obj.reward_objs.push(reward_obj);
        }

        epoch_reward_stake_info_obj.stake_infos = stake_info_objs;
        epoch_reward_stake_info_obj.amount_root = epoch_reward_info
            .amount_root()
            .as_slice()
            .try_into()
            .unwrap();
        epoch_reward_stake_info_obj.amount_proof = epoch_reward_info.amount_proof();
        epoch_reward_stake_info_obj.amount_epoch_proof = epoch_reward_info.amount_epoch_proof();

        epoch_reward_stake_info_obj.count_root = epoch_reward_info
            .count_root()
            .as_slice()
            .try_into()
            .unwrap();
        epoch_reward_stake_info_obj.count_proof = epoch_reward_info.count_proof();
        epoch_reward_stake_info_obj.count_epoch_proof = epoch_reward_info.count_epoch_proof();
        verify_stake_propse(epoch + i as u64 + 1, &epoch_reward_stake_info_obj)?;

        let epoch_reward = calculate_reward(&miner, &epoch_reward_obj);
        reward_amount += epoch_reward;
    }

    // just to pass compile
    let staker_identity = vec![0u8; 20];
    if !secp256k1::verify_signature(&staker_identity) {
        return Err(Error::SignatureMismatch);
    }
    // get at amount of normal at cell from output
    Ok(())
}

fn calculate_reward(miner: &Vec<u8>, epoch_reward_obj: &EpochRewardObject) -> u128 {
    let mut epoch_reward = 0u128;
    let commission_rate = 20; // assume commission rate is 20%
    for obj in &epoch_reward_obj.reward_objs {
        let propose_count = obj.propose_count;
        let base_reward = 1000u128;
        let mut reward = base_reward;
        if propose_count < 100 {
            reward = base_reward * 95 / 100;
        }

        let total_lock_amount = obj.stake_amount + obj.total_delegate_amount;
        let staker_reward = reward * obj.stake_amount / total_lock_amount;
        let delegate_reward = reward - staker_reward;
        if *miner == obj.staker.to_vec() {
            epoch_reward = staker_reward;
            let commission_fee = delegate_reward * commission_rate / 100;
            epoch_reward += commission_fee;
        } else {
            epoch_reward = obj.delegate_amount.unwrap() * delegate_reward * (100 - commission_rate)
                / obj.total_delegate_amount;
        }
    }
    return epoch_reward;
}

fn verify_claim(
    epoch_root: &[u8; 32],
    miner: &[u8; 20], // key
    epoch: u64,       // value
    epoch_proof: &Vec<u8>,
) -> Result<(), Error> {
    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(
            &bytes_to_h256(&miner.to_vec()),
            &bytes_to_h256(&epoch.to_le_bytes().to_vec()),
        )
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;
    epoch_tree
        .verify(&epoch_root, &epoch_proof)
        .map_err(|err| {
            debug!("verify claim smt error: {}", err);
            Error::OldStakeInfosErr
        })?;
    Ok(())
}

fn verify_stake(
    epoch: u64,
    epoch_reward_stake_info_obj: &EpochRewardStakeInfoObject,
) -> Result<(), Error> {
    let mut tree_buf = [Pair::default(); 100];
    let mut amount_tree = Tree::new(&mut tree_buf[..]);
    for stake_info_obj in &epoch_reward_stake_info_obj.stake_infos {
        let staker = stake_info_obj.staker;
        let amount = stake_info_obj.stake_amount;
        amount_tree
            .update(
                &bytes_to_h256(&staker.to_vec()),
                &bytes_to_h256(&amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            })?;
    }

    amount_tree
        .verify(
            &epoch_reward_stake_info_obj.amount_root,
            &epoch_reward_stake_info_obj.amount_proof,
        )
        .map_err(|err| {
            debug!("verify claim smt error: {}", err);
            Error::OldStakeInfosErr
        })?;

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(
            &bytes_to_h256(&epoch.to_le_bytes().to_vec()),
            &epoch_reward_stake_info_obj.amount_root,
        )
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;

    let stake_smt_root = [0u8; 32];
    epoch_tree
        .verify(
            &stake_smt_root,
            &epoch_reward_stake_info_obj.count_epoch_proof,
        )
        .map_err(|err| {
            debug!("verify claim smt error: {}", err);
            Error::OldStakeInfosErr
        })?;

    Ok(())
}

fn verify_stake_propse(
    epoch: u64,
    epoch_reward_stake_info_obj: &EpochRewardStakeInfoObject,
) -> Result<(), Error> {
    verify_stake(epoch, epoch_reward_stake_info_obj)?;
    // verify propose count
    let mut tree_buf = [Pair::default(); 100];
    let mut count_tree = Tree::new(&mut tree_buf[..]);
    for stake_info_obj in &epoch_reward_stake_info_obj.stake_infos {
        let staker = stake_info_obj.staker;
        let propose_count = stake_info_obj.propose_count;
        count_tree
            .update(
                &bytes_to_h256(&staker.to_vec()),
                &bytes_to_h256(&propose_count.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            })?;
    }

    count_tree
        .verify(
            &epoch_reward_stake_info_obj.count_root,
            &epoch_reward_stake_info_obj.count_proof,
        )
        .map_err(|err| {
            debug!("verify claim smt error: {}", err);
            Error::OldStakeInfosErr
        })?;

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(
            &bytes_to_h256(&epoch.to_le_bytes().to_vec()),
            &epoch_reward_stake_info_obj.count_root,
        )
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;

    let propose_count_smt_root = [0u8; 32];
    epoch_tree
        .verify(
            &propose_count_smt_root,
            &epoch_reward_stake_info_obj.count_epoch_proof,
        )
        .map_err(|err| {
            debug!("verify claim smt error: {}", err);
            Error::OldStakeInfosErr
        })?;

    Ok(())
}
