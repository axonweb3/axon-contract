extern crate alloc;
use ckb_std::debug;
use secp256k1_utils::recover_uncompressed_key;
use sha3::{Digest, Keccak256};

use crate::error::Error;

// pub type EthAddress = [u8; 20];

#[derive(Default)]
pub struct Secp256k1Eth;

impl Secp256k1Eth {
    pub fn verify_alone(
        &self,
        eth_address: [u8; 20],
        signature: [u8; 65],
        message: [u8; 32],
    ) -> Result<bool, Error> {
        // ) -> bool {
        let pubkey = recover_uncompressed_key(message.into(), signature).map_err(|err| {
            debug!(
                "Secp256k1Eth failed to recover secp256k1 pubkey, error number: {}",
                err
            );
            Error::EthPubkeyRecoverFail
        })?;
        let pubkey_hash = {
            let mut hasher = Keccak256::new();
            hasher.update(&pubkey[1..]);
            let buf = hasher.finalize();
            let mut pubkey_hash = [0u8; 20];
            pubkey_hash.copy_from_slice(&buf[12..]);
            pubkey_hash
        };
        debug!(
            "Secp256k1Eth pubkey: {:?}, pubkey_hash: {:?}, eth_address: {:?}",
            pubkey, pubkey_hash, eth_address
        );
        if pubkey_hash != eth_address {
            return Ok(false);
        }
        Ok(true)
    }
}
