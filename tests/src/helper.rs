#![allow(dead_code)]

use std::{collections::BTreeSet, convert::TryInto};

use axon_types::{basic, metadata::MetadataList};
use ckb_testtool::{
    ckb_crypto::secp::Privkey,
    ckb_hash::{blake2b_256, new_blake2b},
    ckb_types::{
        bytes::Bytes,
        core::TransactionView,
        packed::{self, *},
        prelude::*,
        H256,
    },
};
use molecule::prelude::*;
use util::smt::{LockInfo};

pub const MAX_CYCLES: u64 = 100_000_000;

pub fn blake160(data: &[u8]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    let hash = blake2b_256(data);
    buf.clone_from_slice(&hash[..20]);
    buf
}

// pub fn axon_byte48(bytes: &[u8; 48]) -> basic::Byte48 {
//     axon::Byte48::new_unchecked(bytes.to_vec().into())
// }

pub fn axon_byte32(bytes: &Byte32) -> basic::Byte32 {
    let bytes: [u8; 32] = bytes.unpack().into();
    basic::Byte32::new_unchecked(bytes.to_vec().into())
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

pub fn axon_identity(pubkey: &Vec<u8>) -> basic::Identity {
    let pubkey_hash = blake160(pubkey.as_slice());
    // convert [u8; 20] to [Byte; 20]
    let pubkey_hash = pubkey_hash
        .iter()
        .map(|value| (*value).into())
        .collect::<Vec<Byte>>();

    basic::Identity::new_builder()
        .set(pubkey_hash.as_slice().try_into().unwrap())
        .build()
}

pub fn axon_identity_opt(pubkey: &Vec<u8>) -> basic::IdentityOpt {
    basic::IdentityOpt::new_builder()
        .set(Some(axon_identity(pubkey)))
        .build()
}

pub fn axon_identity_none() -> basic::IdentityOpt {
    basic::IdentityOpt::new_builder().set(None).build()
}

// construct stake_at cell data based on version, l1_address, l2_address, metadata_type_id, delta
pub fn axon_stake_at_cell_data_without_amount(
    version: u8,
    l1_address: &Vec<u8>,
    l2_address: &Vec<u8>,
    metadata_type_id: &packed::Byte32,
    delta: axon_types::stake::StakeInfoDelta,
) -> axon_types::stake::StakeAtCellData {
    axon_types::stake::StakeAtCellData::new_builder()
        .version(version.into())
        .l1_address(axon_identity(l1_address))
        .l2_address(axon_identity(l2_address))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .delta(delta)
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

pub fn axon_checkpoint_data(
    metadata_type_id: &packed::Byte32,
) -> axon_types::checkpoint::CheckpointCellData {
    // build CheckpointCellData from scrach
    axon_types::checkpoint::CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(1))
        .period(axon_u32(2))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

pub fn axon_metadata_data(
    metadata_type_id: &packed::Byte32,
    xudt_type_id: &packed::Byte32,
    checkpoint_type_id: &packed::Byte32,
    stake_smt_type_id: &packed::Byte32,
    metadata_list: MetadataList,
) -> axon_types::metadata::MetadataCellData {
    // build CheckpointCellData from scrach
    let type_ids = axon_types::metadata::TypeIds::new_builder()
        .metadata_type_id(axon_byte32(metadata_type_id))
        .xudt_type_id(axon_byte32(xudt_type_id))
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

pub fn axon_stake_smt_cell_data(
    stake_infos: &BTreeSet<LockInfo>,
    metadata_type_id: &packed::Byte32,
) -> axon_types::stake::StakeSmtCellData {
    // call build_smt_tree_and_get_root and print error message
    let (root, _proof) = crate::smt::construct_lock_info_smt(stake_infos);
    // println!("root: {:?}", root);

    axon_types::stake::StakeSmtCellData::new_builder()
        .version(0.into())
        .smt_root(basic::Byte32::new_unchecked(
            root.as_slice().to_vec().into(),
        ))
        .metadata_type_id(axon_byte32(metadata_type_id))
        .build()
}

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
