use std::convert::TryFrom;

use ophelia::{Crypto, PrivateKey, Signature, ToPublicKey, UncompressedPublicKey};
// use ophelia_secp256k1::Secp256k1PrivateKey;
use ophelia_secp256k1::{Secp256k1Recoverable, Secp256k1RecoverablePrivateKey};

// pub fn hex_decode(src: &str) -> Vec<u8> {
//     if src.is_empty() {
//         return Vec::new();
//     }

//     let src = if src.starts_with("0x") {
//         src.split_at(2).1
//     } else {
//         src
//     };

//     let src = src.as_bytes();
//     let mut ret = vec![0u8; src.len() / 2];
//     let result = faster_hex::hex_decode(src, &mut ret);

//     ret
// }

// pub fn recover_intact_pub_key(public: &Vec<u8>) -> Vec<u8> {
//     let mut inner = vec![4u8];
//     inner.extend_from_slice(public);
//     inner
// }

#[test]
fn test_eth_success() {
    {
        let msg = [0u8; 32];
        // let priv_key = Secp256k1RecoverablePrivateKey::generate(&mut OsRng);
        let hex_privkey = [0xcd; 32];
        let priv_key = Secp256k1RecoverablePrivateKey::try_from(hex_privkey.as_slice()).unwrap();
        let signature = Secp256k1Recoverable::sign_message(&msg, &priv_key.to_bytes())
            .unwrap()
            .to_bytes();

        let pubkey = priv_key.pub_key();
        let pubkey = pubkey.to_uncompressed_bytes();

        {
            let result = Secp256k1Recoverable::verify_signature(&msg, &signature, &pubkey);
            match result {
                Ok(_) => println!("Verify secp256k1 signature success!"),
                Err(err) => println!("Verify secp256k1 signature failed! {}", err),
            }
        }

        {
            let msg = [1u8; 32];
            let result = Secp256k1Recoverable::verify_signature(&msg, &signature, &pubkey);
            match result {
                Ok(_) => println!("Verify secp256k1 signature success!"),
                Err(err) => println!("Verify secp256k1 signature failed! {}", err),
            }
        }
    }

    // let msg = [0u8; 32];
    // let msg: HashValue = HashValue::from_bytes_unchecked(msg);

    // let hex_privkey = [0xcd; 32];
    // let privkey = Secp256k1PrivateKey::try_from(hex_privkey.as_slice()).unwrap();
    // let pubkey = privkey.pub_key();
    // // let pubkey = pubkey.to_bytes().to_vec();
    // // println!("pub_key, {}", pubkey.len());
    // // let pubkey = recover_intact_pub_key(&pubkey);
    // let pubkey = pubkey.to_uncompressed_bytes();
    // println!("pub_key, {}", pubkey.len());

    // let signature = privkey.sign_message(&msg);
    // let signature = signature.to_bytes();
    // println!("sign_message signature: {}", signature.len());

    // println!("verify_signature");
    // let result = Secp256k1Recoverable::verify_signature(&msg.to_bytes(), &signature, &pubkey);
    // match result {
    //     Ok(_) => println!("Verify secp256k1 signature success!"),
    //     Err(err) => println!("Verify secp256k1 signature failed! {}", err),
    // }

    // let mut rng = rand::thread_rng();
    // let private_key = Secp256k1KeyPair::generate(&mut rng);
    // let pub_key = private_key.public_key();

    // let msg = Message::from_slice(&[1u8; 32]).unwrap();
    // let signature = private_key.sign(&msg);

    // let result = Secp256k1Recoverable::verify_signature(&msg, &signature, &pub_key);

    // // Get the private key.
    // let signature = [1u8; 65];
    // let pub_key = [1u8; 64];
    // // Verify secp256k1 signature
    // let result = Secp256k1Recoverable::verify_signature(&msg, &signature, &pub_key);
    // // assert!(result.is_ok());
    // match result {
    //     Ok(_) => println!("Verify secp256k1 signature success!"),
    //     Err(err) => println!("Verify secp256k1 signature failed! {}", err),
    // }

    // let priv_key = "0x37aa0f893d05914a4def0460c0a984d3611546cfb26924d7a7ca6e0db9950a2d";
    // let hex_privkey = hex_decode(priv_key);
    // println!("hex_privkey: {:?}, len: {}", hex_privkey, hex_privkey.len());
    // let mut rng = thread_rng();
    // let privkey = Secp256k1PrivateKey::generate(&mut OsRng);
}
