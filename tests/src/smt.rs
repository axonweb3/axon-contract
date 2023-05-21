use std::{collections::BTreeSet, convert::TryInto};

use blake2b_rs::{Blake2b, Blake2bBuilder};
use ckb_smt::smt::{Pair, Tree};
use sparse_merkle_tree::{
    default_store::DefaultStore,  traits::{Value, Hasher},
    MerkleProof, SparseMerkleTree, H256, SMTBuilder,
};
use util::{
    smt::{verify_smt_leaf, LockInfo}, error::Error,
};

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

// define SMT
pub type SMT = SparseMerkleTree<Blake2bHasher, Word, DefaultStore<Word>>;
pub type TOP_SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;
pub type BOTTOM_SMT = SparseMerkleTree<Blake2bHasher, BottomValue, DefaultStore<BottomValue>>;

// define SMT value
#[derive(Default, Clone)]
pub struct Word(String);
impl Value for Word {
    fn to_h256(&self) -> H256 {
        if self.0.is_empty() {
            return H256::zero();
        }
        let mut buf = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(self.0.as_bytes());
        hasher.finalize(&mut buf);
        buf.into()
    }
    fn zero() -> Self {
        Default::default()
    }
}

fn construct_smt() {
    let mut tree = SMT::default();
    for (i, word) in "The quick brown fox jumps over the lazy dog"
        .split_whitespace()
        .enumerate()
    {
        println!("i: {}, word: {}", i, word);
        let key: H256 = {
            let mut buf = [0u8; 32];
            let mut hasher = new_blake2b();
            hasher.update(&(i as u32).to_le_bytes());
            hasher.finalize(&mut buf);
            buf.into()
        };
        let value = Word(word.to_string());
        // insert key value into tree
        tree.update(key, value).expect("update");
    }

    let root = tree.root();
    println!("SMT root is {:?} ", tree.root());

    let key1: H256 = {
        let mut buf = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(&(1 as u32).to_le_bytes());
        hasher.finalize(&mut buf);
        buf.into()
    };
    let proof = tree.merkle_proof(vec![key1]).expect("merkle proof");
    println!("proof: {:?}", proof);

    {
        let leaf1 = Word("quick".to_string()).to_h256();
        let leaves = vec![(key1, leaf1)];
        match proof.clone().verify::<Blake2bHasher>(root, leaves) {
            Ok(is_exist) => println!("verify success, exist:{}", is_exist),
            Err(err) => println!("verify error: {}", err),
        }
    }

    {
        let leaf1 = Word("quik".to_string()).to_h256();
        let leaves = vec![(key1, leaf1)];
        match proof.clone().verify::<Blake2bHasher>(root, leaves) {
            Ok(is_exist) => println!("verify success, exist:{}", is_exist),
            Err(err) => println!("verify error: {}", err),
        }
    }
}

#[test]
fn test_smt() {
    construct_smt();
}

pub struct TopSmtInfo {
    pub epoch: u64,
    pub smt_root: H256,
}

// define SMT value
#[derive(Default, Clone, Copy)]
pub struct BottomValue(u128);
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

// helper function
fn new_blake2b() -> Blake2b {
    // Blake2bBuilder::new(32).personal(b"SMT").build()
    Blake2bBuilder::new(32).personal(b"ckb-default-hash").build()
}

pub fn addr_to_h256(addr: &[u8; 20]) -> H256 {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(addr);
    hasher.finalize(&mut buf);
    buf.into()
}

// bottom smt tree
pub fn construct_lock_info_smt(lock_infos: &BTreeSet<LockInfo>) -> (H256, MerkleProof) {
    let mut tree = BOTTOM_SMT::default();
    let mut key1 = H256::zero();
    // travese lock_infos and insert into smt
    for lock_info in lock_infos.iter() {
        let key: H256 = addr_to_h256(&lock_info.addr);
        key1 = key;
        // println!("key: {:?}", key);
        let value = BottomValue(lock_info.amount);
        tree.update(key.to_owned(), value).expect("update");
    }

    (
        *tree.root(),
        tree.merkle_proof(vec![key1]).expect("merkle proof"),
    )
}

pub fn u64_to_h256(epoch: u64) -> H256 {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&epoch.to_le_bytes());
    hasher.finalize(&mut buf);
    buf.into()
}

// top smt tree
pub fn construct_epoch_smt(top_smt_infos: &Vec<TopSmtInfo>) -> (H256, MerkleProof) {
    let mut tree = TOP_SMT::default();
    let mut key1 = H256::zero();
    for top_smt_info in top_smt_infos {
        let epoch = top_smt_info.epoch;
        let key: H256 = u64_to_h256(epoch);
        key1 = key;
        let value = top_smt_info.smt_root;
        tree.update(key.to_owned(), value).expect("update");
    }

    (
        *tree.root(),
        tree.merkle_proof(vec![key1]).expect("merkle proof"),
    )
}

#[test]
fn test_ckb_smt() {
    let mut tree = BOTTOM_SMT::default();
    let key1 = addr_to_h256(&[0u8; 20]);
    let leaf1 = BottomValue(100);
    tree.update(key1.to_owned(), leaf1).expect("update");

    let smt_root = *tree.root();
    let proof = tree.merkle_proof(vec![key1]).expect("merkle proof");
    
    let leaves = vec![(key1, leaf1.clone().to_h256())];
    match proof
        .clone()
        .verify::<Blake2bHasher>(&smt_root, leaves)
    {
        Ok(is_exist) => println!("verify success, exist:{}", is_exist),
        Err(err) => println!("verify error: {}", err),
    }

    let proof = proof.clone().compile(vec![key1]).unwrap().0;

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key1, &leaf1.to_h256()).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&smt_root, &proof).is_ok());
}

pub fn build_smt_tree_and_get_root_local(lock_infos: &BTreeSet<LockInfo>, proof: &Vec<u8>) -> Result<[u8; 32], Error> {
    // construct smt root & verify
    let mut tree_buf = [Pair::default(); 1];
    println!("tree_buf len: {}", tree_buf.len());
    let mut tree = Tree::new(&mut tree_buf);
    println!("lock_infos len: {}", lock_infos.len());
    lock_infos.iter().for_each(|lock_info| {
        let _ = tree
            .update(
                &addr_to_h256(&lock_info.addr).as_slice().try_into().unwrap(),
                BottomValue(lock_info.amount).to_h256().as_slice().try_into().unwrap(),
            )
            .map_err(|err| {
                println!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    // let proof = [0u8; 32];
    println!("calculate_root proof len: {}", proof.len());
    let root = tree.calculate_root(&proof)?; // epoch smt value
    println!("calculate_root after proof len: ");

    Ok(root)
}

#[test]
fn test_lock_info_smt() {
{
    let lock_infos = BTreeSet::<LockInfo>::new();
    let (bottom_smt_root, proof) = construct_lock_info_smt(&lock_infos);
    println!("bottom_smt_root: {:?}", bottom_smt_root);

    let proof = proof.compile(vec![addr_to_h256(&[0u8; 20])]).unwrap().0;
    let re = build_smt_tree_and_get_root_local(&lock_infos, &proof);
    match re {
        Ok(root) => println!("ckb smt root: {:?}", root),
        Err(err) => println!("ckb smt root error: {}", err as u32),
    }
}

    let mut lock_infos = BTreeSet::<LockInfo>::new();
    lock_infos.insert(LockInfo {
        addr: [0u8; 20],
        amount: 100,
    });
    let (bottom_smt_root, proof) = construct_lock_info_smt(&lock_infos);
    println!("bottom_smt_root: {:?}", bottom_smt_root);

    let proof = proof.compile(vec![addr_to_h256(&[0u8; 20])]).unwrap().0;
    let re = build_smt_tree_and_get_root_local(&lock_infos, &proof);
    match re {
        Ok(root) => println!("ckb smt root: {:?}", root),
        Err(err) => println!("ckb smt root error: {}", err as u32),
    }

    let top_smt_infos = vec![TopSmtInfo {
        epoch: 3,
        smt_root: bottom_smt_root,
    }];
    let (top_smt_root, proof) = construct_epoch_smt(&top_smt_infos);

    let key = u64_to_h256(3);
    let proof = proof.compile(vec![key]).unwrap().0;

    let result = verify_smt_leaf(
        key.as_slice().try_into().unwrap(),
        bottom_smt_root.as_slice().try_into().unwrap(),
        top_smt_root.as_slice().try_into().unwrap(),
        &proof,
    );
    match result {
        Ok(_is_exist) => println!("ckb smt verify success, exist"),
        Err(err) => println!("ckb smt verify error: {}", err as u32),
    }
}

