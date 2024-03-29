#![allow(dead_code)]

use std::{collections::BTreeSet, convert::TryInto};

use axon_types::{
    basic::{self, Identity},
    delegate::{StakerSmtRoot, StakerSmtRoots},
    metadata::MetadataList,
    stake::{DelegateRequirementArgs, DelegateRequirementInfo, StakeArgs},
    withdraw::{WithdrawInfo, WithdrawInfos},
};
use ckb_testtool::{
    ckb_crypto::secp::{Privkey, Pubkey},
    ckb_error::Error,
    ckb_hash::{blake2b_256, new_blake2b},
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionView},
        packed::{self, *},
        prelude::*,
        H256,
    },
    context::Context,
};
use molecule::prelude::*;
// use sha3::{Digest, Keccak256};
use sparse_merkle_tree::CompiledMerkleProof;
// use tiny_keccak::{Keccak, Hasher};
use util::{
    helper::pubkey_to_eth_addr,
    smt::{u64_to_h256, LockInfo, TOP_SMT},
};

use crate::{
    delegate::TestDelegateInfo,
    smt::{
        construct_epoch_smt, construct_epoch_smt_for_metadata_update, construct_lock_info_smt,
        TopSmtInfo,
    },
};

pub const MAX_CYCLES: u64 = 200_000_000;

pub fn blake160(data: &[u8]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    let hash = blake2b_256(data);
    buf.clone_from_slice(&hash[..20]);
    buf
}

pub fn pubkey_to_addr(pubkey: &Vec<u8>) -> [u8; 20] {
    blake160(pubkey.as_slice())
}

pub fn calc_type_id(input: &CellInput, index: u64) -> Bytes {
    let mut blake2b = new_blake2b();
    blake2b.update(input.as_slice());
    blake2b.update(&index.to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    Bytes::from(ret.to_vec())
}

// pub fn axon_byte48(bytes: &[u8; 48]) -> basic::Byte48 {
//     axon::Byte48::new_unchecked(bytes.to_vec().into())
// }

pub fn axon_array48_byte48(bytes: [u8; 48]) -> basic::Byte48 {
    basic::Byte48::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte32(bytes: &Byte32) -> basic::Byte32 {
    let bytes: [u8; 32] = bytes.unpack().into();
    basic::Byte32::new_unchecked(bytes.to_vec().into())
}

pub fn axon_array32_byte32(bytes: [u8; 32]) -> basic::Byte32 {
    basic::Byte32::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte33(bytes: Vec<u8>) -> basic::Byte33 {
    basic::Byte33::new_unchecked(bytes.into())
}

pub fn axon_byte65(bytes: Vec<u8>) -> basic::Byte65 {
    basic::Byte65::new_unchecked(bytes.into())
}

pub fn axon_array65_byte65(bytes: [u8; 65]) -> basic::Byte65 {
    basic::Byte65::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte20(bytes: &[u8; 20]) -> basic::Byte20 {
    basic::Byte20::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte16(value: u128) -> basic::Byte16 {
    basic::Byte16::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_byte8(value: u64) -> basic::Byte8 {
    basic::Byte8::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_byte4(value: u32) -> basic::Byte4 {
    basic::Byte4::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_bytes(bytes: &Vec<u8>) -> basic::Bytes {
    let bytes = bytes.into_iter().map(|value| (*value).into()).collect();
    basic::Bytes::new_builder().set(bytes).build()
}

pub fn axon_bytes_byte32(bytes: &Bytes) -> basic::Byte32 {
    basic::Byte32::new_unchecked(bytes.to_vec().into())
}

pub fn axon_bytes_some(bytes: &Vec<u8>) -> basic::BytesOpt {
    basic::BytesOpt::new_builder()
        .set(Some(axon_bytes(bytes)))
        .build()
}

pub fn axon_bytes_none() -> basic::BytesOpt {
    basic::BytesOpt::new_builder().set(None).build()
}

// convert u128 to basic::Uint128
pub fn axon_u128(value: u128) -> basic::Uint128 {
    basic::Uint128::new_unchecked(value.to_le_bytes().to_vec().into())
}

// convert u64 to basic::Uint64
pub fn axon_u64(value: u64) -> basic::Uint64 {
    basic::Uint64::new_unchecked(value.to_le_bytes().to_vec().into())
}

// convert u32 to basic::Uint32
pub fn axon_u32(value: u32) -> basic::Uint32 {
    basic::Uint32::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_u16(value: u16) -> basic::Uint16 {
    basic::Uint16::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_byte20_identity(pubkey_hash: &[u8; 20]) -> basic::Identity {
    // convert [u8; 20] to [Byte; 20]
    let pubkey_hash = pubkey_hash
        .iter()
        .map(|value| (*value).into())
        .collect::<Vec<Byte>>();

    basic::Identity::new_builder()
        .set(pubkey_hash.as_slice().try_into().unwrap())
        .build()
}

pub fn axon_identity(pubkey: &Vec<u8>) -> basic::Identity {
    let pubkey_hash = blake160(pubkey.as_slice());
    // convert [u8; 20] to [Byte; 20]
    axon_byte20_identity(&pubkey_hash)
}

pub fn axon_identity_opt(pubkey: &Vec<u8>) -> basic::IdentityOpt {
    basic::IdentityOpt::new_builder()
        .set(Some(axon_identity(pubkey)))
        .build()
}

pub fn axon_identity_none() -> basic::IdentityOpt {
    basic::IdentityOpt::new_builder().set(None).build()
}

pub fn eth_addr(pubkey: Vec<u8>) -> basic::Identity {
    let pubkey_hash = pubkey_to_eth_addr(&pubkey.try_into().unwrap());

    let pubkey_hash = pubkey_hash
        .iter()
        .map(|value| (*value).into())
        .collect::<Vec<Byte>>();

    basic::Identity::new_builder()
        .set(pubkey_hash.as_slice().try_into().unwrap())
        .build()
}

// construct stake_at cell data based on version, l1_address, l2_address, metadata_type_id, delta
pub fn axon_stake_at_cell_data_without_amount(
    version: u8,
    l1_address: &Vec<u8>,
    l2_address: Identity,
    metadata_type_id: &packed::Byte32,
    delta: axon_types::stake::StakeInfoDelta,
    requirement_info: DelegateRequirementInfo,
) -> axon_types::stake::StakeAtCellData {
    let xudt_data_lock = axon_types::stake::StakeAtCellLockData::new_builder()
        .version(version.into())
        .l1_address(axon_identity(l1_address))
        .l2_address(l2_address)
        .metadata_type_id(axon_byte32(metadata_type_id))
        .delta(delta)
        .requirement_info(requirement_info)
        .build();
    axon_types::stake::StakeAtCellData::new_builder()
        .lock(xudt_data_lock)
        .build()
}

pub fn axon_stake_at_cell_data(
    amount: u128,
    stake_at_cell_data: axon_types::stake::StakeAtCellData,
) -> Vec<u8> {
    // merge amount and stake_at_cell_data to Vec<u8>
    let mut data = Vec::new();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(stake_at_cell_data.as_slice());
    data
}

pub fn axon_delegate_at_cell_data_without_amount(
    version: u8,
    l1_address: &Vec<u8>,
    l2_address: &Vec<u8>,
    metadata_type_id: &packed::Byte32,
    delta: axon_types::delegate::DelegateInfoDeltas,
) -> axon_types::delegate::DelegateAtCellData {
    let lock_data = axon_types::delegate::DelegateAtCellLockData::new_builder()
        .version(version.into())
        .l1_address(axon_identity(l1_address))
        .l2_address(axon_identity(l2_address))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .delegator_infos(delta)
        .build();
    axon_types::delegate::DelegateAtCellData::new_builder()
        .lock(lock_data)
        .build()
}

pub fn axon_delegate_at_cell_data(
    amount: u128,
    delegate_at_cell_data: axon_types::delegate::DelegateAtCellData,
) -> Vec<u8> {
    // merge amount and stake_at_cell_data to Vec<u8>
    let mut data = Vec::new();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(delegate_at_cell_data.as_slice());
    data
}

pub fn axon_delegate_requirement_cell_data(
    commission_rate: u8,
    max_delegator_size: u32,
) -> axon_types::delegate::DelegateCellData {
    let requirement = axon_types::delegate::DelegateRequirement::new_builder()
        .commission_rate(commission_rate.into())
        .max_delegator_size(axon_u32(max_delegator_size))
        .build();
    axon_types::delegate::DelegateCellData::new_builder()
        .delegate_requirement(requirement)
        .build()
}

pub fn axon_delegate_requirement_and_stake_at_cell(
    metadata_type_script: &Script,
    always_success_out_point: &OutPoint,
    always_success_lock_script: &Script,
    context: &mut Context,
    keypair: &(Privkey, Pubkey),
    staker_addr: &[u8; 20],
    max_delegator_size: u32,
) -> (CellDep, CellDep, Script) {
    let requirement_type_id = [1u8; 32];
    let delegate_requirement_args = DelegateRequirementArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .requirement_type_id(axon_array32_byte32(requirement_type_id))
        .build();
    let delegate_requirement_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            delegate_requirement_args.as_bytes(),
        )
        .expect("delegate requirement type script");
    let delegate_requirement_cell_data =
        axon_delegate_requirement_cell_data(10, max_delegator_size);
    let delegate_requirement_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(delegate_requirement_type_script.clone()).pack())
                    .build(),
                delegate_requirement_cell_data.as_bytes(),
            ),
        )
        .build();

    let delegate_requirement_info = DelegateRequirementInfo::new_builder()
        .code_hash(axon_byte32(&delegate_requirement_type_script.code_hash()))
        .requirement(delegate_requirement_args)
        .build();
    let stake_args = StakeArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .stake_addr(axon_byte20_identity(&staker_addr))
        .build();
    // prepare stake lock_script
    let stake_at_lock_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            stake_args.as_bytes(),
        )
        .expect("stake script");
    // println!(
    //     "stake_at_code_hash: {:?}, staker:{:?}, stake_args: {:?}",
    //     stake_at_lock_script.code_hash().as_slice(),
    //     staker_addr,
    //     stake_args.as_slice()
    // );
    let input_stake_info_delta = axon_types::stake::StakeInfoDelta::new_builder().build();
    let input_stake_at_data = axon_stake_at_cell_data_without_amount(
        0,
        &keypair.1.serialize(),
        axon_byte20_identity(&staker_addr),
        &metadata_type_script.calc_script_hash(),
        input_stake_info_delta,
        delegate_requirement_info,
    );
    let stake_at_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(stake_at_lock_script.clone())
                    .build(),
                Bytes::from(axon_stake_at_cell_data(100, input_stake_at_data)),
            ),
        )
        .build();
    (
        delegate_requirement_script_dep,
        stake_at_script_dep,
        stake_at_lock_script,
    )
}

// construct stake_at cell data based on version, l1_address, l2_address, metadata_type_id, delta
pub fn axon_withdraw_at_cell_data_without_amount(
    withdraw_infos: Vec<(u64, u128)>,
) -> axon_types::withdraw::WithdrawAtCellData {
    let mut infos = Vec::new();
    for i in 0..withdraw_infos.len() {
        let (unlock_epoch, amount) = withdraw_infos[i];
        let info = WithdrawInfo::new_builder()
            .unlock_epoch(axon_u64(unlock_epoch))
            .amount(axon_u128(amount))
            .build();
        infos.push(info);
    }
    let withdraw_infos = WithdrawInfos::new_builder().set(infos).build();

    let xudt_data_lock = axon_types::withdraw::WithdrawAtCellLockData::new_builder()
        .withdraw_infos(withdraw_infos)
        .build();
    axon_types::withdraw::WithdrawAtCellData::new_builder()
        .lock(xudt_data_lock)
        .build()
}

pub fn axon_withdraw_at_cell_data(
    amount: u128,
    withdraw_at_cell_data: axon_types::withdraw::WithdrawAtCellData,
) -> Vec<u8> {
    // merge amount and stake_at_cell_data to Vec<u8>
    let mut data = Vec::new();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(withdraw_at_cell_data.as_slice());
    data
}

pub fn axon_normal_at_cell_data(amount: u128, normal_at_cell_data: &[u8]) -> Vec<u8> {
    // merge amount and stake_at_cell_data to Vec<u8>
    let mut data = Vec::new();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(normal_at_cell_data);
    data
}

pub fn axon_checkpoint_data(
    metadata_type_id: &packed::Byte32,
    epoch: u64,
) -> axon_types::checkpoint::CheckpointCellData {
    // build CheckpointCellData from scrach
    axon_types::checkpoint::CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(epoch))
        .period(axon_u32(2))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn axon_metadata_data(
    metadata_type_id: &packed::Byte32,
    xudt_type_hash: &packed::Byte32,
    checkpoint_type_id: &packed::Byte32,
    stake_smt_type_id: &packed::Byte32,
    metadata_list: MetadataList,
) -> axon_types::metadata::MetadataCellData {
    let type_ids = axon_types::metadata::TypeIds::new_builder()
        .metadata_type_id(axon_byte32(metadata_type_id))
        .xudt_type_hash(axon_byte32(xudt_type_hash))
        .checkpoint_type_id(axon_byte32(checkpoint_type_id))
        .stake_smt_type_id(axon_byte32(stake_smt_type_id))
        .build();
    axon_types::metadata::MetadataCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(1))
        .metadata(metadata_list)
        .type_ids(type_ids)
        .build()
}

pub fn axon_metadata_data_by_script(
    metadata_type_id: &Script,
    xudt_type_hash: &packed::Byte32,
    checkpoint_type_id: &Script,
    stake_smt_type_id: &Script,
    delegate_smt_type_id: &Script,
    metadata_list: MetadataList,
    epoch: u64,
    base_reward: u128,
    half_epoch: u64,
    propose_count_smt_root: [u8; 32],
    stake_at_code_hash: &packed::Byte32,
    delegate_at_code_hash: &packed::Byte32,
    withdraw_at_code_hash: &packed::Byte32,
) -> axon_types::metadata::MetadataCellData {
    let checkpoint_args = checkpoint_type_id.args();
    let type_ids = axon_types::metadata::TypeIds::new_builder()
        .metadata_code_hash(axon_byte32(&metadata_type_id.code_hash()))
        .metadata_type_id(axon_bytes_byte32(&metadata_type_id.args().raw_data()))
        .xudt_type_hash(axon_byte32(xudt_type_hash))
        .checkpoint_type_id(axon_bytes_byte32(&checkpoint_args.raw_data()))
        .checkpoint_code_hash(axon_byte32(&checkpoint_type_id.code_hash()))
        .stake_smt_code_hash(axon_byte32(&stake_smt_type_id.code_hash()))
        .stake_smt_type_id(axon_bytes_byte32(&stake_smt_type_id.args().raw_data()))
        .delegate_smt_code_hash(axon_byte32(&delegate_smt_type_id.code_hash()))
        .delegate_smt_type_id(axon_bytes_byte32(&delegate_smt_type_id.args().raw_data()))
        .stake_at_code_hash(axon_byte32(stake_at_code_hash))
        .delegate_at_code_hash(axon_byte32(delegate_at_code_hash))
        .withdraw_code_hash(axon_byte32(withdraw_at_code_hash))
        .build();
    axon_types::metadata::MetadataCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(epoch))
        .base_reward(axon_u128(base_reward))
        .half_epoch(axon_u64(half_epoch))
        .propose_minimum_rate(95.into())
        .propose_discount_rate(95.into())
        .metadata(metadata_list)
        .type_ids(type_ids)
        .propose_count_smt_root(axon_array32_byte32(propose_count_smt_root))
        .build()
}

pub fn delegate_2layer_smt_root_proof(
    epoch: u64,
    delegate_infos: &BTreeSet<LockInfo>,
) -> (sparse_merkle_tree::H256, CompiledMerkleProof) {
    let (delegate_root, _delegate_proof) = construct_lock_info_smt(&delegate_infos);
    let delegate_top_smt_infos = vec![TopSmtInfo {
        epoch: epoch,
        smt_root: delegate_root,
    }];
    let (delegate_epoch_root, delegate_epoch_proof) = construct_epoch_smt(&delegate_top_smt_infos);
    let delegate_epoch_proof = CompiledMerkleProof(
        delegate_epoch_proof
            .compile(vec![u64_to_h256(epoch)])
            .unwrap()
            .0,
    );
    println!(
        "axon_delegate_smt_cell_data delegate_epoch_root: {:?}, delegate_epoch_proof: {:?}, delegate_root: {:?}",
        delegate_epoch_root, delegate_epoch_proof.0, delegate_root
    );
    (delegate_epoch_root, delegate_epoch_proof)
}

pub fn axon_delegate_smt_cell_data(
    delegate_infos: &BTreeSet<LockInfo>,
    metadata_type_id: &packed::Byte32,
    staker_pubkey: &Pubkey,
    epoch: u64,
) -> (
    axon_types::delegate::DelegateSmtCellData,
    CompiledMerkleProof,
) {
    let (delegate_epoch_root, delegate_epoch_proof) =
        delegate_2layer_smt_root_proof(epoch, delegate_infos);

    let stake_smt_root = StakerSmtRoot::new_builder()
        .staker(axon_identity(&staker_pubkey.serialize()))
        .root(axon_array32_byte32(
            delegate_epoch_root.as_slice().try_into().unwrap(),
        ))
        .build();
    let stake_smt_roots = StakerSmtRoots::new_builder().push(stake_smt_root).build();

    (
        axon_types::delegate::DelegateSmtCellData::new_builder()
            .version(0.into())
            .smt_roots(stake_smt_roots)
            .metadata_type_id(axon_byte32(metadata_type_id))
            .build(),
        delegate_epoch_proof,
    )
}

pub fn axon_delegate_smt_cell_data_multiple(
    delegate_infos: &Vec<TestDelegateInfo>,
    metadata_type_id: &packed::Byte32,
    epoch: u64,
) -> axon_types::delegate::DelegateSmtCellData {
    let mut stake_smt_roots = Vec::new();
    for delegate in delegate_infos {
        // let mut delegate_set = BTreeSet::new();
        // for i in delegate.delegates {
        //     delegate_set.insert(i);
        // }
        let (delegate_epoch_root, _delegate_epoch_proof) =
            delegate_2layer_smt_root_proof(epoch, &delegate.delegates);

        let stake_smt_root = StakerSmtRoot::new_builder()
            .staker(axon_byte20_identity(&delegate.staker))
            .root(axon_array32_byte32(
                delegate_epoch_root.as_slice().try_into().unwrap(),
            ))
            .build();
        stake_smt_roots.push(stake_smt_root);
    }

    let stake_smt_roots = StakerSmtRoots::new_builder().set(stake_smt_roots).build();

    axon_types::delegate::DelegateSmtCellData::new_builder()
        .version(0.into())
        .smt_roots(stake_smt_roots)
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn delegate_2layer_smt_root_proof_for_metadata_update(
    epoch: u64,
    delegate_infos: &BTreeSet<LockInfo>,
) -> (sparse_merkle_tree::H256, CompiledMerkleProof) {
    let (delegate_root, _delegate_proof) = construct_lock_info_smt(&delegate_infos);
    let delegate_top_smt_infos = vec![TopSmtInfo {
        epoch: epoch,
        smt_root: delegate_root,
    }];
    let (delegate_epoch_root, delegate_epoch_proof) =
        construct_epoch_smt_for_metadata_update(&delegate_top_smt_infos);
    let delegate_epoch_proof = CompiledMerkleProof(
        delegate_epoch_proof
            .compile(vec![u64_to_h256(epoch), u64_to_h256(epoch + 1)])
            .unwrap()
            .0,
    );
    println!(
        "axon_delegate_smt_cell_data delegate_epoch_root: {:?}, delegate_epoch_proof: {:?}, delegate_root: {:?}",
        delegate_epoch_root, delegate_epoch_proof.0, delegate_root
    );
    (delegate_epoch_root, delegate_epoch_proof)
}

pub fn axon_delegate_smt_cell_data_multiple_for_metadata_update(
    delegate_infos: &Vec<TestDelegateInfo>,
    metadata_type_id: &packed::Byte32,
    epoch: u64,
) -> axon_types::delegate::DelegateSmtCellData {
    let mut stake_smt_roots = Vec::new();
    for delegate in delegate_infos {
        let (delegate_epoch_root, _delegate_epoch_proof) =
            delegate_2layer_smt_root_proof_for_metadata_update(epoch, &delegate.delegates);

        let stake_smt_root = StakerSmtRoot::new_builder()
            .staker(axon_byte20_identity(&delegate.staker))
            .root(axon_array32_byte32(
                delegate_epoch_root.as_slice().try_into().unwrap(),
            ))
            .build();
        stake_smt_roots.push(stake_smt_root);
    }
    let stake_smt_roots = StakerSmtRoots::new_builder().set(stake_smt_roots).build();

    axon_types::delegate::DelegateSmtCellData::new_builder()
        .version(0.into())
        .smt_roots(stake_smt_roots)
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn axon_delegate_smt_cell_data_for_metadata_update(
    delegate_infos: &BTreeSet<LockInfo>,
    metadata_type_id: &packed::Byte32,
    pubkey: &Pubkey,
    epoch: u64,
) -> (
    axon_types::delegate::DelegateSmtCellData,
    CompiledMerkleProof,
) {
    let (delegate_epoch_root, delegate_epoch_proof) =
        delegate_2layer_smt_root_proof_for_metadata_update(epoch, delegate_infos);

    let stake_smt_root = StakerSmtRoot::new_builder()
        .staker(axon_identity(&pubkey.serialize()))
        .root(axon_array32_byte32(
            delegate_epoch_root.as_slice().try_into().unwrap(),
        ))
        .build();
    let stake_smt_roots = StakerSmtRoots::new_builder().push(stake_smt_root).build();

    (
        axon_types::delegate::DelegateSmtCellData::new_builder()
            .version(0.into())
            .smt_roots(stake_smt_roots)
            .metadata_type_id(axon_byte32(metadata_type_id))
            .build(),
        delegate_epoch_proof,
    )
}

pub fn axon_reward_smt_data(
    metadata_type_id: [u8; 32],
    claim_smt_root: [u8; 32],
) -> axon_types::reward::RewardSmtCellData {
    axon_types::reward::RewardSmtCellData::new_builder()
        .version(0.into())
        .metadata_type_id(axon_array32_byte32(metadata_type_id))
        .claim_smt_root(axon_array32_byte32(claim_smt_root))
        .build()
}

pub fn get_input_hash(input: &CellInput) -> Bytes {
    let mut blake2b = new_blake2b();
    blake2b.update(input.as_slice());
    blake2b.update(&0u64.to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    Bytes::from(ret.to_vec())
}

pub fn sign_tx(tx: TransactionView, key: &Privkey, mode: u8) -> TransactionView {
    let mut signed_witnesses: Vec<packed::Bytes> = Vec::new();
    let mut blake2b = new_blake2b();
    blake2b.update(&tx.hash().raw_data());
    // digest the first witness
    let witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .input_type(Some(Bytes::from(vec![mode])).pack())
        .build();
    let witness_size = witness.as_bytes().len() as u64;
    let mut message = [0u8; 32];
    blake2b.update(&witness_size.to_le_bytes());
    blake2b.update(&witness.as_bytes());
    blake2b.finalize(&mut message);
    let message = H256::from(message);
    let sig = key.sign_recoverable(&message).expect("sign");
    signed_witnesses.push(
        witness
            .as_builder()
            .lock(Some(Bytes::from(sig.serialize())).pack())
            .build()
            .as_bytes()
            .pack(),
    );
    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}

pub fn sign_stake_tx(tx: TransactionView, key: &Privkey, witness: WitnessArgs) -> TransactionView {
    let mut signed_witnesses: Vec<packed::Bytes> = Vec::new();
    let mut blake2b = new_blake2b();
    blake2b.update(&tx.hash().raw_data());
    // digest the first witness
    // let witness = WitnessArgs::new_builder()
    //     .lock(Some(Bytes::from(vec![0u8; 65])).pack())
    //     .build();
    let witness_size = witness.as_bytes().len() as u64;
    let mut message = [0u8; 32];
    blake2b.update(&witness_size.to_le_bytes());
    blake2b.update(&witness.as_bytes());
    blake2b.finalize(&mut message);
    let message = H256::from(message);
    let sig = key.sign_recoverable(&message).expect("sign");
    signed_witnesses.push(
        witness
            .as_builder()
            .lock(Some(Bytes::from(sig.serialize())).pack())
            .build()
            .as_bytes()
            .pack(),
    );
    println!("signed_witnesses: {:?}", signed_witnesses.len());
    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}

pub fn sign_eth_tx(tx: TransactionView, witness: WitnessArgs) -> TransactionView {
    let mut signed_witnesses: Vec<packed::Bytes> = Vec::new();

    signed_witnesses.push(witness.as_bytes().pack());
    println!("signed_witnesses: {:?}", signed_witnesses.len());
    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}

pub fn axon_stake_smt_cell_data(
    stake_infos: &BTreeSet<LockInfo>,
    metadata_type_id: &packed::Byte32,
    epoch: u64,
) -> axon_types::stake::StakeSmtCellData {
    // call build_smt_tree_and_get_root and print error message
    let (root, _proof) = crate::smt::construct_lock_info_smt(stake_infos);
    println!(
        "axon_stake_smt_cell_data bottom root: {:?}, top tree epoch: {}",
        root, epoch
    );

    let mut stake_smt_top_tree = TOP_SMT::default();
    let result = stake_smt_top_tree.update(u64_to_h256(epoch), root);
    println!(
        "axon_stake_smt_cell_data update top tree result: {:?}",
        result
    );
    // println!(
    //     "axon_stake_smt_cell_data top root: {:?}",
    //     stake_smt_top_tree.root()
    // );

    axon_types::stake::StakeSmtCellData::new_builder()
        .version(0.into())
        .smt_root(basic::Byte32::new_unchecked(
            stake_smt_top_tree.root().as_slice().to_vec().into(),
        ))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn axon_stake_smt_cell_data_for_update_metadata_cell(
    stake_infos: &BTreeSet<LockInfo>,
    metadata_type_id: &packed::Byte32,
    epoch: u64,
) -> axon_types::stake::StakeSmtCellData {
    // call build_smt_tree_and_get_root and print error message
    let (root, _proof) = crate::smt::construct_lock_info_smt(stake_infos);
    println!(
        "axon_stake_smt_cell_data bottom root: {:?}, top tree epoch: {}",
        root, epoch
    );

    let mut stake_smt_top_tree = TOP_SMT::default();
    let leaves = vec![(u64_to_h256(epoch), root), (u64_to_h256(epoch + 1), root)];
    let result = stake_smt_top_tree.update_all(leaves);
    println!(
        "axon_stake_smt_cell_data update top tree result: {:?}",
        result
    );

    axon_types::stake::StakeSmtCellData::new_builder()
        .version(0.into())
        .smt_root(basic::Byte32::new_unchecked(
            stake_smt_top_tree.root().as_slice().to_vec().into(),
        ))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn assert_script_error(err: Error, err_code: i8) {
    let error_string = err.to_string();
    assert!(
        error_string.contains(format!("error code {} ", err_code).as_str()),
        "error_string: {}, expected_error_code: {}",
        error_string,
        err_code
    );
}
// pub fn axon_delegate_smt_cell_data(
//     stake_infos: &BTreeSet<LockInfo>,
//     metadata_type_id: &packed::Byte32,
// ) -> axon_types::stake::StakeSmtCellData {
//     // call build_smt_tree_and_get_root and print error message
//     let (root, _proof) = crate::smt::construct_lock_info_smt(stake_infos);
//     // println!("root: {:?}", root);

//     axon_types::stake::StakeSmtCellData::new_builder()
//         .version(0.into())
//         .smt_root(basic::Byte32::new_unchecked(
//             root.as_slice().to_vec().into(),
//         ))
//         .metadata_type_id(axon_byte32(metadata_type_id))
//         .build()
// }
// pub fn get_bottom_root_smt_proof(lock_infos: &BTreeSet<LockInfo>, epoch: u64) -> Vec<u8> {
//     // construct smt root & verify
//     let mut tree_buf = [Pair::default(); 100];
//     let mut tree = Tree::new(&mut tree_buf);
//     lock_infos.iter().for_each(|lock_info| {
//         let _ = tree
//             .update(
//                 &bytes_to_h256(&lock_info.addr.to_vec()),
//                 &bytes_to_h256(&lock_info.amount.to_le_bytes().to_vec()),
//             )
//             .map_err(|err| {
//                 println!("update smt tree error: {}", err);
//             });
//     });

//     let proof = [0u8; 32];
//     let bottom_root:[u8; 32] = match tree.calculate_root(&proof) {
//         Ok(root) => root,
//         Err(err) => {
//             println!("calculate root error: {}", err);
//             [0u8; 32]
//         }
//     };

//     println!("proof: {:?}", proof);
//     let mut tree_buf = [Pair::default(); 100];
//     let mut tree = Tree::new(&mut tree_buf);
//         let _ = tree
//             .update(
//                 &bytes_to_h256(&epoch.to_le_bytes().to_vec()),
//                 &bytes_to_h256(&bottom_root.to_vec()),
//             )
//             .map_err(|err| {
//                 println!("update top tree error: {}", err);
//             });

//     let proof = [0u8; 32];
//     let bottom_root:[u8; 32] = match tree.calculate_root(&proof) {
//         Ok(root) => root,
//         Err(err) => {
//             println!("calculate root error: {}", err);
//             [0u8; 32]
//         }
//     };

// }
