extern crate alloc;

use core::cmp::Ordering;

use alloc::{collections::BTreeSet, vec::Vec};
use ckb_smt::smt::{Pair, Tree};
use ckb_std::debug;

use crate::{error::Error, helper::bytes_to_h256};

#[derive(Clone, Copy, Default, Eq, PartialOrd, Debug)]
pub struct LockInfo {
    pub addr: [u8; 20], // address of locker(staker or delegator), smt key
    pub amount: u128,   // amount locked, smt value
}

// impl LockInfo {
//     pub fn new(stake_info: &stake_reader::StakeInfo) -> Self {
//         let mut identity = [0u8; 20];
//         identity.copy_from_slice(&stake_info.addr());
//         Self {
//             addr: identity,
//             amount: bytes_to_u128(&stake_info.amount()),
//         }
//     }
// }

impl Ord for LockInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = other.addr.cmp(&self.addr);
        order
    }
}

impl PartialEq for LockInfo {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

pub fn build_smt_tree_and_get_root(lock_infos: &BTreeSet<LockInfo>) -> Result<[u8; 32], Error> {
    // construct smt root & verify
    let mut tree_buf = [Pair::default(); 100];
    let mut tree = Tree::new(&mut tree_buf);
    lock_infos.iter().for_each(|lock_info| {
        let _ = tree
            .update(
                &bytes_to_h256(&lock_info.addr.to_vec()),
                &bytes_to_h256(&lock_info.amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    let proof = [0u8; 32];
    let root = tree.calculate_root(&proof)?; // epoch smt value

    Ok(root)
}

pub fn verify_smt_leaf(
    key: u64,
    value: &[u8; 32],
    root: &[u8; 32],
    proof: &Vec<u8>,
) -> Result<(), Error> {
    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(&bytes_to_h256(&key.to_le_bytes().to_vec()), &value)
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::SmterrorCodeErrorUpdate
        })?;
    epoch_tree.verify(&root, &proof).map_err(|err| {
        debug!("verify smt error: {}", err);
        Error::SmterrorCodeErrorVerify
    })?;
    Ok(())
}

pub fn verify_2layer_smt(
    lock_infos: &BTreeSet<LockInfo>,
    epoch: u64,
    top_proof: &Vec<u8>,
    top_root: &[u8; 32],
) -> Result<(), Error> {
    // construct old stake smt root & verify
    let bottom_root = build_smt_tree_and_get_root(lock_infos)?;
    verify_smt_leaf(epoch, &bottom_root, top_root, top_proof)?;
    Ok(())
}
