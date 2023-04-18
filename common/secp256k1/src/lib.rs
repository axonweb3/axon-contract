#![no_std]
// #![feature(asm)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;
use alloc::vec::Vec;

#[link(name = "ckb-lib-secp256k1", kind = "static")]
extern "C" {
    fn verify_secp256k1_blake160_sighash_all(pubkey_hash: *const u8) -> i32;
}

pub fn verify_signature(pubkey_hash: &Vec<u8>) -> bool {
    let error_code = unsafe { verify_secp256k1_blake160_sighash_all(pubkey_hash.as_ptr()) };
    return error_code == 0;
}
