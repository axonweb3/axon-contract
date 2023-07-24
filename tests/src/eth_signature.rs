// //! Secp256k1 Eth implementation

// use std::convert::TryFrom;

// use ophelia::{Crypto, PrivateKey, Signature, ToPublicKey, UncompressedPublicKey};
// use ophelia_secp256k1::{Secp256k1Recoverable, Secp256k1RecoverablePrivateKey};
// // use gw_utils::{
// //     ckb_std::debug,
// //     error::Error,
// //     gw_types::{bytes::Bytes, h256::H256},
// // };
// use secp256k1_utils::recover_uncompressed_key;
// use sha3::{Digest, Keccak256};

// pub type EthAddress = [u8; 20];

// #[derive(Default)]
// pub struct Secp256k1Eth;

// impl Secp256k1Eth {
//     pub fn verify_alone(
//         &self,
//         eth_address: EthAddress,
//         signature: [u8; 65],
//         // message: H256,
//         message: [u8; 32],
//         // ) -> Result<bool, Error> {
//     ) -> bool {
//         // let pubkey = recover_uncompressed_key(message.into(), signature).map_err(|err| {
//         //     debug!("failed to recover secp256k1 pubkey, error number: {}", err);
//         //     Error::WrongSignature
//         // })?;
//         let pubkey = recover_uncompressed_key(message.into(), signature)
//             .map_err(|err| {
//                 // debug!("failed to recover secp256k1 pubkey, error number: {}", err);
//                 // Error::WrongSignature
//                 println!("failed to recover secp256k1 pubkey, error number: {}", err);
//             })
//             .unwrap();
//         let pubkey_hash = {
//             let mut hasher = Keccak256::new();
//             hasher.update(&pubkey[1..]);
//             let buf = hasher.finalize();
//             let mut pubkey_hash = [0u8; 20];
//             pubkey_hash.copy_from_slice(&buf[12..]);
//             pubkey_hash
//         };
//         if pubkey_hash != eth_address {
//             return false;
//         }
//         true
//     }

//     // pub fn verify_message(
//     //     &self,
//     //     eth_address: EthAddress,
//     //     signature: [u8; 65],
//     //     message: H256,
//     // ) -> Result<bool, Error> {
//     //     let mut hasher = Keccak256::new();
//     //     hasher.update("\x19Ethereum Signed Message:\n32");
//     //     hasher.update(message.as_slice());
//     //     let buf = hasher.finalize();
//     //     let mut signing_message = [0u8; 32];
//     //     signing_message.copy_from_slice(&buf[..]);
//     //     let signing_message = H256::from(signing_message);

//     //     self.verify_alone(eth_address, signature, signing_message)
//     // }
// }

// #[test]
// fn test_gw_eth_success() {
//     // let msg = [0u8; 32];
//     // // let priv_key = Secp256k1RecoverablePrivateKey::generate(&mut OsRng);
//     // let hex_privkey = [0xcd; 32];
//     // let priv_key = Secp256k1RecoverablePrivateKey::try_from(hex_privkey.as_slice()).unwrap();
//     // let signature = Secp256k1Recoverable::sign_message(&msg, &priv_key.to_bytes())
//     //     .unwrap()
//     //     .to_bytes();

//     // let pubkey = priv_key.pub_key();
//     // let pubkey = pubkey.to_uncompressed_bytes();

//     // let secp_eth = Secp256k1Eth::default();
//     // secp_eth.verify_alone(
//     //     , signature, message)
// }
