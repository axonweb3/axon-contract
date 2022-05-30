// Import from `core` instead of from `std` since we are in no-std mode
use alloc::vec;
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    high_level::{load_script, load_tx_hash, load_witness_args},
};

use protocol::{crosschain, Cursor};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // args contains metadata_typehash
    if args.len() != 32 {
        return Err(Error::BadScriptArgs);
    }

    // find metadata from CellDep
    let metadata = get_metadata_from_celldep(&args.to_vec())?;

    // find signature_pubkey_list from witness lock
    let sig;
    let mut sig_pubkeys = vec![];
    let witness_args = load_witness_args(0, Source::GroupInput)?;
    if let Some(data) = witness_args.lock().to_opt() {
        // find bls_pubkey_list from stake info
        let bls_pubkeys = get_bls_pubkeys_from_celldep(&metadata.stake_typehash())?;
        let (signature, pubkeys) = {
            let witness = crosschain::Witness::from(Cursor::from(data.raw_data().to_vec()));
            (witness.signature(), witness.bls_pubkeys())
        };
        for i in 0..pubkeys.len() {
            let mut pubkey = [0u8; 48];
            pubkey.copy_from_slice(pubkeys.get(i).as_slice());
            if !bls_pubkeys.contains(&pubkey) {
                return Err(Error::UnexpectedBlsPubkey);
            }
            sig_pubkeys.push(pubkey);
        }
        sig = signature;
    } else {
        return Err(Error::BadWitnessLock);
    }

    // check bls signature validation
    let digest = load_tx_hash()?;
    if !blst::verify_blst_signature(&sig_pubkeys, &sig, &digest.to_vec()) {
        return Err(Error::SignatureMismatch);
    }

    Ok(())
}
