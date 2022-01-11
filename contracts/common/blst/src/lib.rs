#![no_std]
#![feature(asm)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;
use alloc::{vec, vec::Vec};

#[link(name = "ckb-lib-secp256k1-blst", kind = "static")]
extern "C" {
    fn verify_secp256k1_blake160_sighash_all(pubkey_hash: *const u8) -> i32;
    fn blst_verify_aggregate(
        sig: *const u8,
        pkvec: *const u8,
        pkvec_len: usize,
        msg: *const u8,
        msg_len: usize,
    ) -> i32;
}

pub fn verify_secp256k1_signature(pubkey_hash: &mut Vec<u8>) -> bool {
    let error_code = unsafe { verify_secp256k1_blake160_sighash_all(pubkey_hash.as_mut_ptr()) };
    return error_code == 0;
}

pub fn verify_blst_signature(
    pubkeys: &Vec<[u8; 48]>,
    signature: &[u8; 96],
    message: &Vec<u8>,
) -> bool {
    let mut pkstream = vec![];
    pubkeys
        .iter()
        .for_each(|pk| pkstream.append(&mut pk.to_vec()));
    let error_code = unsafe {
        blst_verify_aggregate(
            signature.as_ptr(),
            pkstream.as_ptr(),
            pubkeys.len(),
            message.as_ptr(),
            message.len(),
        )
    };
    return error_code == 0;
}
