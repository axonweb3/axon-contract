// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec::Vec;
use alloc::{collections::BTreeSet, vec};
use axon_types::reward_reader::NotClaimInfo;
use axon_types::reward_reader::RewardSmtCellData;
use ckb_type_id::{load_type_id_from_script_args, validate_type_id};
use core::result::Result;
use sparse_merkle_tree::{blake2b::Blake2bHasher, CompiledMerkleProof, H256};
use util::smt::{
    addr_to_h256, smt_verify_leaves, u128_to_h256, u64_to_h256, verify_2layer_smt, verify_top_smt,
    LockInfo,
};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
// use ckb_smt::smt::{Pair, Tree};
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{load_script, load_witness_args},
};

use axon_types::{reward_reader, Cursor};
use util::{error::Error, helper::*};

#[derive(Clone, Copy, Debug, Default)]
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

fn verify_claim_smt(
    miner: &Vec<u8>,
    claim_epoch: &u64,
    not_claim_info: &NotClaimInfo,
    reward_smt_data: &RewardSmtCellData,
) -> Result<(), Error> {
    let miner_h256 = addr_to_h256(&miner.as_slice().try_into().unwrap());
    let proof = CompiledMerkleProof(not_claim_info.proof());
    let mut claim_epoch_h256 = u64_to_h256(*claim_epoch);
    if *claim_epoch == 0 {
        claim_epoch_h256 = H256::default();
    }
    let claim_root: [u8; 32] = reward_smt_data
        .claim_smt_root()
        .as_slice()
        .try_into()
        .unwrap();
    let claim_root: H256 = claim_root.into();
    let result = verify_top_smt(miner_h256, claim_epoch_h256, claim_root, proof)?;
    debug!("verify claim smt result: {}", result);
    Ok(())
}

fn verify_old_new_claim_smt(
    reward_smt_type_id: &Vec<u8>,
    miner: &Vec<u8>,
    old_not_claim_info: &NotClaimInfo,
    new_not_claim_info: &NotClaimInfo,
) -> Result<(u64, u64, Vec<u8>), Error> {
    let old_claim_epoch = old_not_claim_info.epoch();
    let new_claim_epoch = new_not_claim_info.epoch();

    let old_reward_smt_data = get_reward_smt_data(
        reward_smt_type_id.as_slice().try_into().unwrap(),
        Source::GroupInput,
    )?;

    verify_claim_smt(
        &miner,
        &old_claim_epoch,
        &old_not_claim_info,
        &old_reward_smt_data,
    )?;

    let new_reward_smt_data = get_reward_smt_data(
        reward_smt_type_id.as_slice().try_into().unwrap(),
        Source::GroupOutput,
    )?;
    verify_claim_smt(
        &miner,
        &new_claim_epoch,
        &new_not_claim_info,
        &new_reward_smt_data,
    )?;

    if old_reward_smt_data.metadata_type_id() != new_reward_smt_data.metadata_type_id() {
        return Err(Error::MisMatchMetadataTypeId);
    }

    Ok((
        old_claim_epoch,
        new_claim_epoch,
        old_reward_smt_data.metadata_type_id(),
    ))
}

pub fn main() -> Result<(), Error> {
    let type_id = load_type_id_from_script_args(0)?;
    // debug!("type_id: {:?}", type_id);
    validate_type_id(type_id)?;

    let script = load_script()?;
    let reward_smt_type_id = calc_script_hash(&script).to_vec();
    // debug!("reward_smt_type_id = {:?}", reward_smt_type_id);
    let input_reward_smt_count = get_cell_count_by_type_hash(&reward_smt_type_id, Source::Input);
    if input_reward_smt_count == 0 {
        debug!("reward smt cell creation");
        return Ok(());
    }

    // identify contract mode by witness
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    let reward_witness = {
        let witness_input_type = witness_args.input_type().to_opt();
        if witness_input_type.is_none() {
            return Err(Error::WitnessLockError);
        }
        let value: reward_reader::RewardWitness =
            Cursor::from(witness_input_type.unwrap().raw_data().to_vec()).into();
        value
    };

    debug!("verify reward claim info");
    let miner = reward_witness.miner();
    let old_not_claim_info = reward_witness.old_not_claim_info();
    let new_not_claim_info = reward_witness.new_not_claim_info();
    let (old_claim_epoch, new_claim_epoch, meta_type_id) = verify_old_new_claim_smt(
        &reward_smt_type_id,
        &miner,
        &old_not_claim_info,
        &new_not_claim_info,
    )?;

    // debug!("get type ids, {:?}", meta_type_id);
    let metadata_type_id = meta_type_id.as_slice().try_into().unwrap();
    let type_ids = get_type_ids(&metadata_type_id, Source::CellDep)?;

    let stake_smt_type_id = get_script_hash(
        &type_ids.stake_smt_code_hash(),
        &type_ids.stake_smt_type_id(),
    );
    // debug!("stake_smt_type_id = {:?}", stake_smt_type_id);
    let stake_smt_root = get_stake_smt_root(
        stake_smt_type_id.as_slice().try_into().unwrap(),
        Source::CellDep,
    )?;
    let delegate_smt_type_id = get_script_hash(
        &type_ids.delegate_smt_code_hash(),
        &type_ids.delegate_smt_type_id(),
    );
    let delegate_smt_data = get_delegate_smt_data(&delegate_smt_type_id, Source::CellDep)?;
    let metadata = get_metada_data_by_type_id(&metadata_type_id, Source::CellDep)?;
    let propose_count_smt_root: [u8; 32] = metadata
        .propose_count_smt_root()
        .as_slice()
        .try_into()
        .unwrap();

    let mut reward_amount: u128 = 0;
    let reward_infos = reward_witness.reward_infos();
    for current_epoch in old_claim_epoch..new_claim_epoch {
        // many epoch, 1st layer
        let mut epoch_reward_obj = EpochRewardObject::default(); // used to calculate reward
        epoch_reward_obj.miner = miner.as_slice().try_into().unwrap();

        let mut epoch_reward_stake_info_obj = EpochRewardStakeInfoObject::default();
        let mut stake_info_objs = Vec::new();

        let epoch_reward_info =
            reward_infos.get((current_epoch - old_claim_epoch).try_into().unwrap());
        let staker_infos = epoch_reward_info.reward_stake_infos();
        // get one staker's propose count, stake amount, verify its delegate info
        for j in 0..staker_infos.len() {
            // many staker, 2nd layer
            let mut reward_obj = RewardObject::default();
            let stake_info = staker_infos.get(j);
            let staker = stake_info.validator();
            let delegate_infos = stake_info.delegate_infos();
            let mut delegate_infos_set = BTreeSet::new();
            let mut total_delegate_amount = 0u128;
            for k in 0..delegate_infos.len() {
                let delegate_info = delegate_infos.get(k);
                let delegate_info_obj = LockInfo {
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
            let delegate_epoch_proof = CompiledMerkleProof(delegate_epoch_proof);
            let delegate_epoch_root = get_delegate_smt_root_from_cell_data(
                staker.as_slice().try_into().unwrap(),
                &delegate_smt_data,
            )?;
            let delegate_epoch_root: H256 = delegate_epoch_root.into();
            verify_2layer_smt(
                &delegate_infos_set,
                u64_to_h256(current_epoch),
                delegate_epoch_root,
                delegate_epoch_proof,
            )?;

            let propose_count = stake_info.propose_count();
            let stake_amount = bytes_to_u128(&stake_info.staker_amount());

            let stake_info_obj = RewardStakeInfoObject {
                staker: staker.as_slice().try_into().unwrap(),
                propose_count: propose_count,
                stake_amount: stake_amount,
            };
            debug!(
                "stake_info_obj: {:?}, total_delegate_amount: {}",
                stake_info_obj, total_delegate_amount
            );
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
        verify_stake_propse(
            current_epoch,
            &epoch_reward_stake_info_obj,
            &stake_smt_root,
            &propose_count_smt_root,
        )?;

        let epoch_reward = calculate_reward(&miner, &epoch_reward_obj);
        reward_amount += epoch_reward;
    }

    // get at amount of normal at cell from output
    let xudt_type_hash = type_ids.xudt_type_hash();
    let input_total_amount = get_xudt_by_type_hash(&xudt_type_hash, Source::Input)?;
    let output_total_amount = get_xudt_by_type_hash(&xudt_type_hash, Source::Output)?;
    debug!(
        "reward_amount: {}, input_total_amount: {}, output_total_amount: {}",
        reward_amount, input_total_amount, output_total_amount
    );
    if input_total_amount + reward_amount != output_total_amount {
        return Err(Error::RewardWrongAmount);
    }
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
        debug!("miner: {:?},staker: {:?}", miner, obj.staker);
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

/*
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
*/

fn verify_stake(
    epoch: u64,
    epoch_reward_stake_info_obj: &EpochRewardStakeInfoObject,
    stake_smt_root: &[u8; 32],
) -> Result<(), Error> {
    let mut leaves = Vec::new();
    for stake_info_obj in &epoch_reward_stake_info_obj.stake_infos {
        leaves.push((
            addr_to_h256(&stake_info_obj.staker),
            u128_to_h256(stake_info_obj.stake_amount),
        ));
    }

    let result = smt_verify_leaves(
        leaves,
        epoch_reward_stake_info_obj.amount_root.into(),
        CompiledMerkleProof(epoch_reward_stake_info_obj.amount_proof.clone()),
    )?;
    debug!("verify stake smt bottom result: {}", result);
    if result == false {
        return Err(Error::RewardStakeAmountBottomFail);
    }

    let amount_epoch_proof =
        CompiledMerkleProof(epoch_reward_stake_info_obj.amount_epoch_proof.clone());
    // debug!(
    //     "verify stake smt top proof: {:?}, top root: {:?}, amount_root: {:?}, epoch: {}",
    //     epoch_reward_stake_info_obj.amount_epoch_proof,
    //     stake_smt_root,
    //     epoch_reward_stake_info_obj.amount_root,
    //     epoch
    // );
    let stake_smt_root: H256 = (*stake_smt_root).into();
    let result = amount_epoch_proof
        .verify::<Blake2bHasher>(
            &stake_smt_root,
            vec![(
                u64_to_h256(epoch),
                epoch_reward_stake_info_obj.amount_root.into(),
            )],
        )
        .unwrap();
    debug!("verify stake smt top result: {}", result);
    if result == false {
        return Err(Error::RewardStakeAmountTopFail);
    }

    Ok(())
}

fn verify_stake_propse(
    epoch: u64,
    epoch_reward_stake_info_obj: &EpochRewardStakeInfoObject,
    stake_smt_root: &[u8; 32],
    propose_count_smt_root: &[u8; 32],
) -> Result<(), Error> {
    verify_stake(epoch, epoch_reward_stake_info_obj, stake_smt_root)?;

    // verify propose count
    let mut leaves = Vec::new();
    for stake_info_obj in &epoch_reward_stake_info_obj.stake_infos {
        leaves.push((
            addr_to_h256(&stake_info_obj.staker),
            u64_to_h256(stake_info_obj.propose_count),
        ));
        // debug!(
        //     "verify propose count smt bottom proof: {:?}, bottom root: {:?}, count: {:?}, epoch: {}",
        //     epoch_reward_stake_info_obj.count_proof,
        //     epoch_reward_stake_info_obj.count_root,
        //     stake_info_obj.propose_count,
        //     epoch
        // );
    }

    let result = smt_verify_leaves(
        leaves,
        epoch_reward_stake_info_obj.count_root.into(),
        CompiledMerkleProof(epoch_reward_stake_info_obj.count_proof.clone()),
    )?;
    debug!("verify propose count smt bottom result: {}", result);
    if result == false {
        return Err(Error::RewardProposeCountBottomFail);
    }

    let propose_count_smt_root: H256 = (*propose_count_smt_root).into();
    let propose_count_epoch_proof =
        CompiledMerkleProof(epoch_reward_stake_info_obj.count_epoch_proof.clone());
    let leaves = vec![(
        u64_to_h256(epoch),
        epoch_reward_stake_info_obj.count_root.into(),
    )];
    let result = propose_count_epoch_proof
        .verify::<Blake2bHasher>(&propose_count_smt_root, leaves)
        .unwrap();
    debug!("verify propose count smt top result: {}", result);
    if result == false {
        return Err(Error::RewardProposeCountTopFail);
    }

    Ok(())
}
