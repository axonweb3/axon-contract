#![no_std]
#![feature(asm)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;
use alloc::vec::Vec;

#[link(name = "ckb-lib-secp256k1-blst", kind = "static")]
extern "C" {
    fn verify_secp256k1_blake160_sighash_all(pubkey_hash: *const u8) -> i32;
    fn verify_bls12_381_blake160_sighash_all(pubkey_hash: *const u8) -> i32;
}

pub fn verify_secp256k1_signature(pubkey_hash: &mut Vec<u8>) -> bool {
    let error_code = unsafe { verify_secp256k1_blake160_sighash_all(pubkey_hash.as_mut_ptr()) };
    return error_code == 0;
}

pub fn verify_blst_signature(pubkey_hash: &mut Vec<u8>) -> bool {
    let error_code = unsafe { verify_bls12_381_blake160_sighash_all(pubkey_hash.as_mut_ptr()) };
    return error_code == 0;
}
