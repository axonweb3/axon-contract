extern crate alloc;

use ckb_std::debug;
use secp256k1_utils::recover_uncompressed_key;
use util::{error::Error, helper::pubkey_to_eth_addr};

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
        let pubkey = recover_uncompressed_key(message.into(), signature).map_err(|err| {
            debug!("failed to recover secp256k1 pubkey, error number: {}", err);
            Error::EthPubkeyRecoverError
        })?;

        let pubkey_hash = pubkey_to_eth_addr(&pubkey.to_vec());
        debug!(
            "verify_alone pubkey: {:?}, pubkey_hash: {:?}, eth_address: {:?}",
            pubkey, pubkey_hash, eth_address
        );
        if pubkey_hash != eth_address {
            return Ok(false);
        }
        Ok(true)
    }
}
