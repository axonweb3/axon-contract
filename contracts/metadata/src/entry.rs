extern crate alloc;
// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

use alloc::vec::Vec;
use alloc::{collections::BTreeSet, vec};

// use axon_types::metadata;
use axon_types::metadata_reader::{self, ElectionSmtProof, MetadataWitness, StakeSmtElectionInfo};
// use axon_types::reward_reader::EpochRewardStakeInfo;
use axon_types::{
    checkpoint_reader::CheckpointCellData, metadata_reader::MetadataCellData,
    metadata_reader::TypeIds, metadata_reader::ValidatorList,
};
// use ckb_std::high_level::load_cell_type_hash;
// use ckb_std::ckb_types::packed::Script;
// use ckb_smt::smt::{Pair, Tree};
// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_script, load_witness_args},
};

use axon_types::{metadata_reader::Metadata, Cursor};
use sparse_merkle_tree::{CompiledMerkleProof, H256};
use util::helper::{
    calc_script_hash, get_current_epoch, get_delegate_smt_root, get_quorum_size, get_script_hash,
    get_stake_smt_root, MinerGroupInfoObject,
};
use util::smt::{u64_to_h256, verify_2layer_smt_propose, LockInfo};
use util::{
    error::Error,
    helper::{
        get_checkpoint_by_type_id, get_epoch_len, get_metada_data_by_type_id, get_type_ids,
        ProposeCountObject,
    },
};

pub fn main() -> Result<(), Error> {
    // debug!("begin metadata verification");
    let script = load_script()?;
    // debug!("script: {:?}", script);
    let args: Bytes = script.args().unpack();

    let metadata_args: metadata_reader::MetadataArgs = Cursor::from(args.to_vec()).into();
    debug!(
        "metadata_args script args {:?}",
        metadata_args.metadata_type_id()
    );
    // let metadata_type_id = metadata_args.metadata_type_id();
    let metadata_type_id = calc_script_hash(&script);
    debug!("metadata_type_id: {:?}", metadata_type_id);
    let type_ids = get_type_ids(&metadata_type_id, Source::Input)?;
    let input_metadata = get_metada_data_by_type_id(&metadata_type_id, Source::Input)?;
    let output_metadata = get_metada_data_by_type_id(&metadata_type_id, Source::Output)?;
    debug!("verify_chain_config");
    verify_chain_config(&input_metadata, &output_metadata)?;

    // debug!("checkpoint type id: {:?}", type_ids.checkpoint_type_id());
    // debug!(
    //     "checkpoint code hash: {:?}",
    //     type_ids.checkpoint_code_hash()
    // );
    let checkpoint_script_hash = get_script_hash(
        &type_ids.checkpoint_code_hash(),
        &type_ids.checkpoint_type_id(),
    );
    debug!("checkpoint_script_hash: {:?}", checkpoint_script_hash);
    let (_, checkpoint_data) =
        get_checkpoint_by_type_id(&checkpoint_script_hash.to_vec(), Source::CellDep)?;

    debug!("verify_last_checkpoint_of_epoch");
    verify_last_checkpoint_of_epoch(&metadata_type_id.to_vec(), &checkpoint_data)?;

    debug!("verify_propose_counts");
    verify_propose_counts(&checkpoint_data, &output_metadata)?;

    debug!("verify_election");
    verify_election(&type_ids)?;

    // verify lock_info smt root of stake in epoch n + 1 is equal to n

    // just to pass compile
    // let staker_identity = vec![0u8; 20];
    // if !secp256k1::verify_signature(&staker_identity) {
    //     return Err(Error::SignatureMismatch);
    // }

    Ok(())
}

// verify data correctness exclude propose count and election
fn verify_chain_config(
    input_metadata: &MetadataCellData,
    output_metadata: &MetadataCellData,
) -> Result<(), Error> {
    // metadata do not need epoch? checkpoint is enough
    if input_metadata.epoch() + 1 != output_metadata.epoch() {
        return Err(Error::MetadataEpochWrong);
    }

    let input_metadatas = input_metadata.metadata();
    let output_metadatas = output_metadata.metadata();
    if input_metadatas.len() != 3 || output_metadatas.len() != 3 {
        return Err(Error::MetadataSizeWrong);
    }

    let input_metadata1 = input_metadatas.get(1);
    let input_metadata2 = input_metadatas.get(2);
    let output_metadata0 = output_metadatas.get(0);
    let output_metadata1 = output_metadatas.get(1);
    let output_metadata2 = output_metadatas.get(2);

    if !is_metadata_equal(&input_metadata1, &output_metadata0)
        || !is_metadata_equal(&input_metadata2, &output_metadata1)
    {
        debug!("input_metadata1: MetadataInputOutputMismatch");
        return Err(Error::MetadataInputOutputMismatch);
    }

    // output metadata2 will update something, like validators, block height, etc.
    if output_metadata1.brake_ratio() != output_metadata2.brake_ratio()
        || output_metadata1.epoch_len() != output_metadata2.epoch_len()
        || output_metadata1.gas_limit() != output_metadata2.gas_limit()
        || output_metadata1.gas_price() != output_metadata2.gas_price()
        || output_metadata1.interval() != output_metadata2.interval()
        || output_metadata1.max_tx_size() != output_metadata2.max_tx_size()
        || output_metadata1.period_len() != output_metadata2.period_len()
        || output_metadata1.precommit_ratio() != output_metadata2.precommit_ratio()
        || output_metadata1.prevote_ratio() != output_metadata2.prevote_ratio()
        || output_metadata1.propose_ratio() != output_metadata2.propose_ratio()
        || output_metadata1.quorum() != output_metadata2.quorum()
        || output_metadata1.tx_num_limit() != output_metadata2.tx_num_limit()
    {
        debug!("output_metadata1: MetadataInputOutputMismatch");
        return Err(Error::MetadataInputOutputMismatch);
    }

    Ok(())
}

fn verify_last_checkpoint_of_epoch(
    metadata_type_id: &Vec<u8>,
    checkpoint: &CheckpointCellData,
) -> Result<(), Error> {
    let epoch_len = get_epoch_len(
        metadata_type_id.as_slice().try_into().unwrap(),
        Source::GroupInput,
    )?;
    let period = checkpoint.period();
    if period != epoch_len {
        return Err(Error::NotLastCheckpoint);
    }
    Ok(())
}

fn verify_propose_counts(
    checkpoint_data: &CheckpointCellData,
    output_metadata: &MetadataCellData,
) -> Result<(), Error> {
    let propose_counts = checkpoint_data.propose_count();
    let mut propose_count_objs: Vec<ProposeCountObject> = vec![];
    for i in 0..propose_counts.len() {
        let propose_count = &propose_counts.get(i);
        let id: [u8; 20] = propose_count.address().as_slice().try_into().unwrap();
        let count = propose_count.count();
        let propose_count_obj = ProposeCountObject {
            addr: id,
            count: count,
        };
        debug!("propose_count_obj: {:?}", propose_count_obj);
        propose_count_objs.push(propose_count_obj);
    }
    // verify new data by propose_smt_root from output
    let epoch_proof: Vec<u8>;
    let witness_args = load_witness_args(0, Source::Input);
    match witness_args {
        Ok(witness) => {
            // epoch_proof = witness.lock().as_slice().to_vec();
            // let input_type = witness.input_type().to_opt();
            // if input_type.is_none() {
            //     return Err(Error::BadWitnessInputType);
            // }
            // let input_type: u8 = input_type.unwrap().raw_data().to_vec().first().unwarp();
            // let input_type = *input_type.unwrap().raw_data().to_vec().first().unwrap();
            // debug!("input_type: {:?}", input_type);
            epoch_proof = {
                let witness_lock = witness.lock().to_opt();
                debug!("witness_lock: {:?}", witness_lock);
                if witness_lock.is_none() {
                    return Err(Error::WitnessLockError);
                }
                let value: metadata_reader::MetadataWitness =
                    Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
                value.new_propose_proof()
            };
        }
        Err(_) => {
            return Err(Error::UnknownMode);
        }
    };
    let epoch_root: [u8; 32] = output_metadata.propose_count_smt_root().try_into().unwrap();
    let epoch_root: H256 = epoch_root.into();
    let epoch_proof = CompiledMerkleProof(epoch_proof);
    let result = verify_2layer_smt_propose(
        &propose_count_objs,
        u64_to_h256(checkpoint_data.epoch()),
        epoch_root,
        epoch_proof,
    )?;
    debug!("verify_2layer_smt_propose result: {:?}", result);

    Ok(())
}

fn verify_election(type_ids: &TypeIds) -> Result<(), Error> {
    /*
        let stake_smt_type_id = type_ids.stake_smt_type_id();
        // check stake smt cell in input and output
        let mut stake_smt_cell_count = 0;
        QueryIter::new(load_cell_type_hash, Source::Input).for_each(|type_hash| {
            if type_hash.unwrap_or([0u8; 32]) == stake_smt_type_id[..] {
                stake_smt_cell_count += 1;
            }
        });
        if stake_smt_cell_count != 1 {
            return Err(Error::MetadataNoStakeSmt);
        }

        stake_smt_cell_count = 0;
        QueryIter::new(load_cell_type_hash, Source::Output).for_each(|type_hash| {
            if type_hash.unwrap_or([0u8; 32]) == stake_smt_type_id[..] {
                stake_smt_cell_count += 1;
            }
        });
        if stake_smt_cell_count != 1 {
            return Err(Error::MetadataNoStakeSmt);
        }
    */
    let quorum = get_quorum_size(
        type_ids.metadata_type_id().as_slice().try_into().unwrap(),
        Source::Input,
    )?;
    debug!("quorum: {:?}", quorum);
    verify_election_metadata(&type_ids, quorum)?;

    Ok(())
}

pub fn is_metadata_equal(left: &Metadata, right: &Metadata) -> bool {
    if left.brake_ratio() == right.brake_ratio()
        && left.epoch_len() == right.epoch_len()
        && left.gas_limit() == right.gas_limit()
        && left.gas_price() == right.gas_price()
        && left.interval() == right.interval()
        && left.max_tx_size() == right.max_tx_size()
        && left.period_len() == right.period_len()
        && left.precommit_ratio() == right.precommit_ratio()
        && left.prevote_ratio() == right.prevote_ratio()
        && left.propose_ratio() == right.propose_ratio()
        && left.quorum() == right.quorum()
        && left.tx_num_limit() == right.tx_num_limit()
        // && left.block_height() != right.block_height()
        && is_validators_equal(&left.validators(), &right.validators())
    {
        true
    } else {
        false
    }
}

pub fn is_validators_equal(left: &ValidatorList, right: &ValidatorList) -> bool {
    if left.len() != right.len() {
        false
    } else {
        for i in 0..left.len() {
            let lv = left.get(i);
            let rv = right.get(i);
            if lv.address() != rv.address()
                || lv.bls_pub_key() != rv.bls_pub_key()
                || lv.propose_count() != rv.propose_count()
                || lv.propose_weight() != rv.propose_weight()
                || lv.pub_key() != rv.pub_key()
                || lv.vote_weight() != rv.vote_weight()
            {
                return false;
            }
        }
        true
    }
}

// pub fn verify_2layer_smt(
//     propose_counts: &Vec<ProposeCountObject>,
//     epoch: u64,
//     epoch_proof: &Vec<u8>,
//     epoch_root: &[u8; 32],
// ) -> Result<(), Error> {
//     // construct old stake smt root & verify
//     let mut tree_buf = [Pair::default(); 100];
//     let mut tree = Tree::new(&mut tree_buf);
//     propose_counts.iter().for_each(|propose_count| {
//         let _ = tree
//             .update(
//                 &bytes_to_h256(&propose_count.identity.to_vec()),
//                 &bytes_to_h256(&propose_count.count.to_le_bytes().to_vec()),
//             )
//             .map_err(|err| {
//                 debug!("update smt tree error: {}", err);
//                 Error::MerkleProof
//             });
//     });

//     let proof = [0u8; 32];
//     let stake_root = tree.calculate_root(&proof)?; // epoch smt value
//     let mut tree_buf = [Pair::default(); 100];
//     let mut epoch_tree = Tree::new(&mut tree_buf[..]);
//     epoch_tree
//         .update(&bytes_to_h256(&epoch.to_le_bytes().to_vec()), &stake_root)
//         .map_err(|err| {
//             debug!("update smt tree error: {}", err);
//             Error::MerkleProof
//         })?;
//     epoch_tree
//         .verify(&epoch_root, &epoch_proof)
//         .map_err(|err| {
//             debug!("verify smt tree error: {}", err);
//             Error::OldStakeInfosErr
//         })?;
//     Ok(())
// }

// should be checked in metadata script
fn verify_election_metadata(type_ids: &TypeIds, quorum_size: u16) -> Result<(), Error> {
    // get stake & delegate data of epoch n + 1 & n + 2,  from witness of stake smt cell
    let election_infos = {
        let witness_args = load_witness_args(0, Source::Input);
        let witness_lock = witness_args.unwrap().lock().to_opt();
        if witness_lock.is_none() {
            return Err(Error::WitnessLockError);
        }
        let value: MetadataWitness = Cursor::from(witness_lock.unwrap().raw_data().to_vec()).into();
        value.smt_election_info()
    };

    // staker info of n + 2
    let election_info_n2 = election_infos.n2();
    let mut miners_n2 = BTreeSet::new();
    let checkpoint_script_hash = get_script_hash(
        &type_ids.checkpoint_code_hash(),
        &type_ids.checkpoint_type_id(),
    );
    let epoch = get_current_epoch(&checkpoint_script_hash.to_vec())?;
    debug!("get_current_epoch: {:?}", epoch);
    // verify stake and delegate infos in witness is correct, construct miners to get updated data
    verify_stake_delegate(
        &election_info_n2,
        epoch + 2,
        &mut miners_n2,
        type_ids,
        Source::Input,
    )?;

    // only keep top quorum stakers as validators, others as delete_stakers & delete_delegators
    let iter = miners_n2.iter();
    let mut top_quorum = iter.take(quorum_size.into());
    let mut validators = BTreeSet::new();
    while let Some(elem) = top_quorum.next() {
        validators.insert((*elem).clone());
    }
    // get output metadata, verify the validators data.
    debug!("verify_new_validators, {:?}", validators);
    verify_new_validators(&validators, epoch, type_ids, &election_infos)?;

    // verify validators' stake amount, verify delete_stakers & delete_delegators all zero & withdraw At cell amount is equal.

    Ok(())
}

// verify stake and delegate infos, fill miners with respect to election_info
pub fn verify_stake_delegate(
    election_info: &ElectionSmtProof,
    epoch: u64,
    miners: &mut BTreeSet<MinerGroupInfoObject>,
    type_ids: &TypeIds,
    source: Source,
) -> Result<(), Error> {
    let miner_infos = election_info.miners();
    let mut stake_infos = BTreeSet::new();

    // get stake infos and miner group info
    for i in 0..miner_infos.len() {
        let miner_info = &miner_infos.get(i);
        let miner_group_obj = MinerGroupInfoObject::new(miner_info);

        stake_infos.insert(LockInfo {
            addr: miner_group_obj.staker,
            amount: miner_group_obj.stake_amount,
        });
        miners.insert(miner_group_obj);
    }

    // verify stake info of epoch n
    let epoch_proof = CompiledMerkleProof(election_info.staker_epoch_proof());
    let epoch_root = get_stake_smt_root(
        type_ids.stake_smt_type_id().as_slice().try_into().unwrap(),
        source,
    )?;
    let epoch_root: H256 = epoch_root.into();
    let result =
        util::smt::verify_2layer_smt(&stake_infos, u64_to_h256(epoch), epoch_root, epoch_proof)?;
    debug!("staker verify_2layer_smt result: {:?}", result);

    let new_miners = miners.clone();
    debug!("new_miners: {:?}", new_miners);
    for miner in new_miners {
        let mut delegate_infos = BTreeSet::new();
        for i in 0..miner.delegators.len() {
            let delegate_info = miner.delegators.get(i).unwrap();
            delegate_infos.insert(*delegate_info);
        }
        debug!("delegator_epoch_proof");
        let epoch_proof = CompiledMerkleProof(miner.delegator_epoch_proof);
        debug!("delegate_smt_type_hash");
        let delegate_smt_type_hash = get_script_hash(
            &type_ids.checkpoint_code_hash(),
            &type_ids.delegate_smt_type_id(),
        );
        let epoch_root = get_delegate_smt_root(&delegate_smt_type_hash, &miner.staker, source)?;
        let epoch_root: H256 = epoch_root.into();
        let result = util::smt::verify_2layer_smt(
            &delegate_infos,
            u64_to_h256(epoch),
            epoch_root,
            epoch_proof,
        )?;
        debug!(
            "miner: {:?}, verify_2layer_smt result: {:?}",
            miner.staker, result
        );
    }

    Ok(())
}

fn verify_new_validators(
    validators: &BTreeSet<MinerGroupInfoObject>,
    epoch: u64,
    type_ids: &TypeIds,
    election_infos: &StakeSmtElectionInfo,
) -> Result<(), Error> {
    let mut stake_infos = BTreeSet::new();
    // get stake infos and miner group info
    for validator in validators {
        stake_infos.insert(LockInfo {
            addr: validator.staker,
            amount: validator.stake_amount,
        });
    }

    debug!("verify_new_validators new_stake_proof");
    // verify stake info of epoch n
    let epoch_proof = CompiledMerkleProof(election_infos.new_stake_proof());
    debug!("verify_new_validators get_stake_smt_root");
    let epoch_root = get_stake_smt_root(
        type_ids.stake_smt_type_id().as_slice().try_into().unwrap(),
        Source::Output,
    )?;
    let epoch_root: H256 = epoch_root.into();
    let result = util::smt::verify_2layer_smt(&stake_infos, u64_to_h256(epoch), epoch_root, epoch_proof)?;
    debug!("verify_new_validators verify_2layer_smt result: {:?}", result);

    let new_miners = validators.clone();
    let epoch_proofs = election_infos.new_delegate_proofs();
    for miner in new_miners {
        let mut delegate_infos = BTreeSet::new();
        for i in 0..miner.delegators.len() {
            let delegate_info = miner.delegators.get(i).unwrap();
            delegate_infos.insert(*delegate_info);
        }
        let mut epoch_proof = vec![];
        for i in 0..epoch_proofs.len() {
            let proof = epoch_proofs.get(i);
            if proof.staker() == miner.staker {
                epoch_proof = proof.proof();
                break;
            }
        }
        let epoch_proof = CompiledMerkleProof(epoch_proof);
        let delegate_smt_type_hash = get_script_hash(
            &type_ids.checkpoint_code_hash(),
            &type_ids.delegate_smt_type_id(),
        );
        let epoch_root =
            get_delegate_smt_root(&delegate_smt_type_hash, &miner.staker, Source::Output)?;
        let epoch_root: H256 = epoch_root.into();
        let result = util::smt::verify_2layer_smt(
            &delegate_infos,
            u64_to_h256(epoch),
            epoch_root,
            epoch_proof,
        )?;
        debug!("verify_new_validators verify_2layer_smt result: {:?}", result);
    }

    Ok(())
}
