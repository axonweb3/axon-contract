extern crate alloc;

use core::cmp::Ordering;

use alloc::vec;
use alloc::{collections::BTreeSet, vec::Vec};
use blake2b_ref::{Blake2b, Blake2bBuilder};
use ckb_smt::smt::{Pair, Tree};
use ckb_std::debug;
use sparse_merkle_tree::CompiledMerkleProof;
use sparse_merkle_tree::{
    default_store::DefaultStore,
    traits::{Hasher, Value},
    SparseMerkleTree, H256,
};

use crate::error::Error;
use crate::helper::ProposeCountObject;
pub struct Blake2bHasher(Blake2b);

impl Default for Blake2bHasher {
    fn default() -> Self {
        let blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        Blake2bHasher(blake2b)
    }
}

impl Hasher for Blake2bHasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }
    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}

// define SMT value
#[derive(Default, Clone, Copy)]
pub struct BottomValue(pub u128);
impl Value for BottomValue {
    fn to_h256(&self) -> H256 {
        let mut buf = [0u8; 32];
        let mut hasher = new_blake2b();
        // println!("amount: {}", self.0);
        hasher.update(&self.0.to_le_bytes());
        hasher.finalize(&mut buf);
        buf.into()
    }
    fn zero() -> Self {
        Default::default()
    }
}

// define SMT value
#[derive(Default, Clone, Copy)]
pub struct ProposeBottomValue(pub u32);
impl Value for ProposeBottomValue {
    fn to_h256(&self) -> H256 {
        let mut buf = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(&self.0.to_le_bytes());
        hasher.finalize(&mut buf);
        buf.into()
    }
    fn zero() -> Self {
        Default::default()
    }
}

// define SMT
#[allow(non_camel_case_types)]
pub type TOP_SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;
#[allow(non_camel_case_types)]
pub type BOTTOM_SMT = SparseMerkleTree<Blake2bHasher, BottomValue, DefaultStore<BottomValue>>;
#[allow(non_camel_case_types)]
pub type PROPOSE_BOTTOM_SMT =
    SparseMerkleTree<Blake2bHasher, ProposeBottomValue, DefaultStore<ProposeBottomValue>>;
// helper function
fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build()
}

pub fn addr_to_byte32(addr: &[u8; 20]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(addr);
    hasher.finalize(&mut buf);
    buf
}

pub fn u128_to_byte32(amount: &u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&amount.to_le_bytes());
    hasher.finalize(&mut buf);
    buf
}

pub fn u64_to_byte32(epoch: &u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&epoch.to_le_bytes());
    hasher.finalize(&mut buf);
    buf
}

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

pub fn addr_to_h256(addr: &[u8; 20]) -> H256 {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(addr);
    hasher.finalize(&mut buf);
    buf.into()
}

pub fn u64_to_h256(epoch: u64) -> H256 {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&epoch.to_le_bytes());
    hasher.finalize(&mut buf);
    buf.into()
}

pub fn get_bottom_smt_root(lock_infos: &BTreeSet<LockInfo>) -> H256 {
    let mut tree = BOTTOM_SMT::default();
    // travese lock_infos and insert into smt
    for lock_info in lock_infos.iter() {
        let key: H256 = addr_to_h256(&lock_info.addr);
        let value = BottomValue(lock_info.amount);
        tree.update(key, value).expect("update");
    }

    // let root: [u8; 32] = tree.root().as_slice().try_into().unwrap();
    *tree.root()
}

pub fn get_bottom_smt_root_propose(propose_infos: &Vec<ProposeCountObject>) -> H256 {
    let mut tree = PROPOSE_BOTTOM_SMT::default();
    for propose_info in propose_infos.iter() {
        let key: H256 = addr_to_h256(&propose_info.addr);
        let value = ProposeBottomValue(propose_info.count);
        tree.update(key, value).expect("update");
    }

    // let root: [u8; 32] = tree.root().as_slice().try_into().unwrap();
    *tree.root()
}

pub fn verify_top_smt(
    key: H256,
    value: H256,
    root: H256,
    proof: CompiledMerkleProof,
) -> Result<bool, Error> {
    let leaves = vec![(key, value)];
    let result = proof
        .verify::<Blake2bHasher>(&root, leaves)
        .map_err(|_err| {
            debug!("update smt tree error: {}", _err);
            Error::SmterrorCodeErrorUpdate
        })?;
    Ok(result)
}

pub fn verify_2layer_smt(
    lock_infos: &BTreeSet<LockInfo>,
    epoch: H256,
    top_root: H256,
    top_proof: CompiledMerkleProof,
) -> Result<bool, Error> {
    // construct old stake smt root & verify
    let bottom_root = get_bottom_smt_root(lock_infos);
    debug!("bottom_root: {:?}", bottom_root);
    verify_top_smt(epoch, bottom_root, top_root, top_proof)
}

pub fn verify_2layer_smt_propose(
    propose_counts: &Vec<ProposeCountObject>,
    epoch: H256,
    top_root: H256,
    top_proof: CompiledMerkleProof,
) -> Result<bool, Error> {
    // construct old stake smt root & verify
    let bottom_root = get_bottom_smt_root_propose(propose_counts);
    verify_top_smt(epoch, bottom_root, top_root, top_proof)
}

// pub fn build_smt_tree_and_get_root(
//     lock_infos: &BTreeSet<LockInfo>,
//     proof: &Option<Vec<u8>>,
// ) -> Result<[u8; 32], Error> {
//     // construct smt root & verify
//     let mut tree_buf = [Pair::default(); 1];
//     // debug!("tree_buf len: {}", tree_buf.len());
//     let mut tree = Tree::new(&mut tree_buf);
//     // debug!("lock_infos len: {}", lock_infos.len());
//     lock_infos.iter().for_each(|lock_info| {
//         let _ = tree
//             .update(
//                 &addr_to_byte32(&lock_info.addr),
//                 &u128_to_byte32(&lock_info.amount),
//             )
//             .map_err(|_err| {
//                 debug!("update smt tree error: {}", _err);
//                 Error::MerkleProof
//             });
//     });

//     let root = if proof.is_none() {
//         // the old smt is empty, so return default root directly
//         [0u8; 32]
//     } else {
//         tree.calculate_root(&proof.as_ref().unwrap()[..])
//             .map_err(|_err| {
//                 debug!("calculate root error: {}", _err);
//                 Error::MerkleProof
//             })?
//     };

//     Ok(root)
// }

pub fn verify_smt_leaf(
    key: &[u8; 32],
    value: &[u8; 32],
    root: &[u8; 32],
    proof: &Vec<u8>,
) -> Result<(), Error> {
    let mut tree_buf = [Pair::default(); 1];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree.update(&key, &value).map_err(|_err| {
        debug!("update smt tree error: {}", _err);
        Error::SmterrorCodeErrorUpdate
    })?;
    epoch_tree.verify(&root, &proof).map_err(|_err| {
        debug!("smt verify smt error: {}", _err);
        Error::SmterrorCodeErrorVerify
    })?;
    Ok(())
}

// pub fn verify_2layer_smt(
//     lock_infos: &BTreeSet<LockInfo>,
//     epoch: u64,
//     top_proof: &Vec<u8>,
//     top_root: &[u8; 32],
//     old_bottm_proof: &Option<Vec<u8>>,
// ) -> Result<(), Error> {
//     // construct old stake smt root & verify
//     let bottom_root = build_smt_tree_and_get_root(lock_infos, &old_bottm_proof)?;
//     verify_smt_leaf(&u64_to_byte32(&epoch), &bottom_root, top_root, top_proof)?;
//     Ok(())
// }
