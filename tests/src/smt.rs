use std::collections::BTreeSet;

use blake2b_rs::{Blake2b, Blake2bBuilder};
use sparse_merkle_tree::{
    blake2b::Blake2bHasher, default_store::DefaultStore, traits::Value, CompiledMerkleProof,
    MerkleProof, SMTBuilder, SparseMerkleTree, H256,
};
use util::{
    helper::ProposeCountObject,
    smt::{
        addr_to_h256, u64_to_h256, verify_2layer_smt, BottomValue, LockInfo, ProposeBottomValue,
        BOTTOM_SMT, PROPOSE_BOTTOM_SMT, TOP_SMT,
    },
};

// define SMT
pub type SMT = SparseMerkleTree<Blake2bHasher, Word, DefaultStore<Word>>;

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

// helper function
fn new_blake2b() -> Blake2b {
    // Blake2bBuilder::new(32).personal(b"SMT").build()
    Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build()
}

// bottom smt tree
pub fn construct_lock_info_smt(lock_infos: &BTreeSet<LockInfo>) -> (H256, Option<MerkleProof>) {
    let mut tree = BOTTOM_SMT::default();
    let mut keys = Vec::<H256>::new();
    // travese lock_infos and insert into smt
    for lock_info in lock_infos.iter() {
        let key: H256 = addr_to_h256(&lock_info.addr);
        keys.push(key);
        // println!("key: {:?}", key);
        let value = BottomValue(lock_info.amount);
        tree.update(key.to_owned(), value).expect("update");
    }

    // println!("lock_infos len: {}, root: {:?}", lock_infos.len(), tree.root());
    if keys.is_empty() {
        return (H256::zero(), None);
    } else {
        return (
            *tree.root(),
            Some(tree.merkle_proof(keys).expect("merkle proof")),
        );
    }
}

// bottom smt tree
pub fn construct_propose_count_smt(
    propose_infos: &Vec<ProposeCountObject>,
) -> (H256, Option<MerkleProof>) {
    let mut tree = PROPOSE_BOTTOM_SMT::default();
    let mut keys = Vec::<H256>::new();
    // travese lock_infos and insert into smt
    for propose_info in propose_infos.iter() {
        let key: H256 = addr_to_h256(&propose_info.addr);
        keys.push(key);
        // println!("key: {:?}", key);
        let value = ProposeBottomValue(propose_info.count);
        tree.update(key.to_owned(), value).expect("update");
    }

    if keys.is_empty() {
        return (H256::zero(), None);
    } else {
        return (
            *tree.root(),
            Some(tree.merkle_proof(keys).expect("merkle proof")),
        );
    }
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

// top smt tree, only suitable for metadata update and top_smt_infos.len() == 1
pub fn construct_epoch_smt_for_metadata_update(
    top_smt_infos: &Vec<TopSmtInfo>,
) -> (H256, MerkleProof) {
    assert_eq!(top_smt_infos.len(), 1);

    let mut tree = TOP_SMT::default();
    let top_smt_info = top_smt_infos.get(0).unwrap();
    let epoch = top_smt_info.epoch;
    let key0: H256 = u64_to_h256(epoch);
    let key1: H256 = u64_to_h256(epoch + 1);
    let value = top_smt_info.smt_root;
    let leaves = vec![(key0, value), (key1, value)];
    tree.update_all(leaves).expect("update");

    (
        *tree.root(),
        tree.merkle_proof(vec![key0, key1]).expect("merkle proof"),
    )
}

#[test]
fn test_smt_compiled_proof() {
    let mut tree = BOTTOM_SMT::default();
    let key1 = addr_to_h256(&[0u8; 20]);
    let leaf1 = BottomValue(100);
    tree.update(key1.to_owned(), leaf1).expect("update");

    let smt_root = *tree.root();
    let leaves_keys = vec![key1];
    let proof = tree
        .merkle_proof(vec![key1])
        .expect("merkle proof")
        .compile(leaves_keys.clone())
        .unwrap();

    let leaves = vec![(key1, leaf1.clone().to_h256())];
    match proof
        .clone()
        .verify::<Blake2bHasher>(&smt_root, leaves.clone())
    {
        Ok(is_exist) => println!("verify success, exist:{}", is_exist),
        Err(err) => println!("verify error: {}", err),
    }

    let proof = proof.0;
    let compiled_merkle_proof = CompiledMerkleProof(proof);
    match compiled_merkle_proof.verify::<Blake2bHasher>(&smt_root, leaves) {
        Ok(is_exist) => println!("verify success, exist:{}", is_exist),
        Err(err) => println!("verify error: {}", err),
    }

    // construct 2 layer smt
    let mut lock_infos = BTreeSet::<LockInfo>::new();
    lock_infos.insert(LockInfo {
        addr: [0u8; 20],
        amount: 100,
    });
    let (bottom_root, _bottom_proof) = construct_lock_info_smt(&lock_infos);
    let top_smt_infos = vec![TopSmtInfo {
        epoch: 3,
        smt_root: bottom_root,
    }];
    let (top_root, top_proof) = construct_epoch_smt(&top_smt_infos);

    // verify 2 layer smt
    let top_proof = top_proof.compile(vec![key1]).unwrap();
    let epoch = u64_to_h256(3);
    let result = verify_2layer_smt(&lock_infos, epoch, top_root, top_proof);
    match result {
        Ok(true) => println!("Success!"),
        Ok(false) => println!("Failure!"),
        Err(e) => println!("Error: {:?}", e as u32),
    }
}

#[test]
fn test_2leaves_smt() {
    {
        let mut tree = TOP_SMT::default();
        let key0 = u64_to_h256(2);
        let key1 = u64_to_h256(2 + 1);
        let value = H256::from([1u8; 32]);
        println!("key0: {:?}, key1: {:?}, value: {:?}", key0, key1, value);
        let leaves = vec![(key0, value), (key1, value)];
        let _ = tree.update_all(leaves.clone());
        let root = tree.root();

        let keys = vec![key0, key1];
        let proof = tree.merkle_proof(keys.clone()).unwrap();
        let proof = proof.compile(keys.clone()).unwrap();
        println!("root: {:?}, proof: {:?}", root, proof);
        let result = proof.verify::<Blake2bHasher>(&root, leaves).unwrap();
        println!("result: {}", result);

        let key2 = u64_to_h256(4);
        let leaves = vec![(key0, value), (key2, value)];
        let result = proof.verify::<Blake2bHasher>(&root, leaves);
        match result {
            Ok(true) => println!("Success!"),
            Ok(false) => println!("Failure!"),
            Err(e) => println!("Error: {:?}", e),
        }

        {
            let leaves = vec![(key0, value), (key1, value), (key2, value)];
            let result = proof.verify::<Blake2bHasher>(&root, leaves);
            match result {
                Ok(true) => println!("Success!"),
                Ok(false) => println!("Failure!"),
                Err(e) => println!("Error: {:?}", e),
            }
        }
    }
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
    match proof.clone().verify::<Blake2bHasher>(&smt_root, leaves) {
        Ok(is_exist) => println!("verify success, exist:{}", is_exist),
        Err(err) => println!("verify error: {}", err),
    }

    let proof = proof.clone().compile(vec![key1]).unwrap().0;

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key1, &leaf1.to_h256()).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&smt_root, &proof).is_ok());
}
