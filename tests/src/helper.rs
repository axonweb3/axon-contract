#![allow(dead_code)]

use crate::axon;
use ckb_testtool::ckb_crypto::secp::{Privkey, Pubkey};
use ckb_testtool::ckb_hash::{blake2b_256, new_blake2b};
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::TransactionView,
    packed::{self, *},
    prelude::*,
    H256,
};
use molecule::prelude::*;

pub fn blake160(data: &[u8]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    let hash = blake2b_256(data);
    buf.clone_from_slice(&hash[..20]);
    buf
}

pub fn axon_byte32(bytes: &Byte32) -> axon::Byte32 {
    let bytes: [u8; 32] = bytes.unpack();
    axon::Byte32::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte20(bytes: &[u8; 20]) -> axon::Byte20 {
    axon::Byte20::new_unchecked(bytes.to_vec().into())
}

pub fn axon_byte8(value: u64) -> axon::Byte8 {
    axon::Byte8::new_unchecked(value.to_le_bytes().to_vec().into())
}

pub fn axon_identity(pubkey: &Pubkey) -> axon::Identity {
    let compressed_pubkey = pubkey.serialize();
    let pubkey_hash = blake160(compressed_pubkey.to_vec().as_slice());
    axon::Identity::new_builder()
        .flag(Byte::from(0))
        .content(axon_byte20(&pubkey_hash))
        .build()
}

pub fn axon_identity_opt(pubkey: &Pubkey) -> axon::IdentityOpt {
    axon::IdentityOpt::new_builder()
        .set(Some(axon_identity(pubkey)))
        .build()
}

pub fn axon_checkpoint_data(
    period: u64,
    half_period: u64,
    sudt_type_hash: &Byte32,
) -> axon::CheckpointLockCellData {
    axon::CheckpointLockCellData::new_builder()
        .period(axon_byte8(period))
        .half_period(axon_byte8(half_period))
        .sudt_type_hash(axon_byte32(sudt_type_hash))
        .build()
}

pub fn axon_withdrawal_data(period: u64) -> Vec<u8> {
    let mut data = vec![];
    data.append(&mut 0u128.to_le_bytes().to_vec());
    data.append(&mut period.to_le_bytes().to_vec());
    data
}

pub fn axon_stake_info(pubkey_hash: &[u8; 20], era: u64) -> axon::StakeInfo {
    let identity = axon::Identity::new_builder()
        .flag(Byte::from(0))
        .content(axon_byte20(&pubkey_hash))
        .build();
    axon::StakeInfo::new_builder()
        .identity(identity)
        .inauguration_era(axon_byte8(era))
        .build()
}

pub fn axon_stake_data(
    quorum: u8,
    checkpoint_type_hash: &Byte32,
    infos: Vec<axon::StakeInfo>,
) -> axon::StakeLockCellData {
    let stake_infos = axon::StakeInfoVec::new_builder().set(infos).build();
    axon::StakeLockCellData::new_builder()
        .checkpoint_type_hash(axon_byte32(checkpoint_type_hash))
        .stake_infos(stake_infos)
        .quorum_size(quorum.into())
        .build()
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
