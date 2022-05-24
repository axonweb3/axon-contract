// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::load_script,
};

use util::error::Error;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // args contains Axon Admin address
    if args.len() != 20 {
        return Err(Error::BadScriptArgs);
    }

    // check Admin signature
    if !secp256k1::verify_signature(&args.to_vec()) {
        return Err(Error::SignatureMismatch);
    }

    Ok(())
}
