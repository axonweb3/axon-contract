use crate::helper::axon_byte32;

use super::*;
use axon_types::checkpoint::*;
use axon_types::metadata::{Metadata, MetadataList, Validator, ValidatorList};
use bit_vec::BitVec;
use blst::min_pk::{AggregatePublicKey, AggregateSignature, SecretKey};
// use ckb_system_scripts::BUNDLED_CELL;
use ckb_testtool::ckb_crypto::secp::Generator;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use rand::prelude::*;
use rlp::RlpStream;
use util::helper::keccak256;

#[test]
fn test_checkpoint_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("checkpoint");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");

    let metadata_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![5]))
        .expect("metadata type script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // prepare stake_args and stake_data
    let _keypair = Generator::random_keypair();
    let checkpoint_args = CheckpointArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .build();
    let checkpoint_type_script = context
        .build_script(&contract_out_point, Bytes::from(checkpoint_args.as_bytes()))
        .expect("checkpoint script");
    println!(
        "checkpoint type hash: {:?}",
        checkpoint_type_script.calc_script_hash().as_slice()
    );

    let input_checkpoint_data = CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(1))
        .period(axon_u32(2))
        // .latest_block_hash(v)
        .latest_block_height(axon_u64(10))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        // .propose_count(v)
        .state_root(axon_byte32(&[0u8; 32].pack()))
        .timestamp(axon_u64(11111))
        .build();

    // prepare tx inputs and outputs
    let inputs = vec![CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(checkpoint_type_script.clone()).pack())
                    .build(),
                Bytes::from(input_checkpoint_data.as_bytes()),
            ),
        )
        .build()];
    let outputs = vec![CellOutput::new_builder()
        .capacity(1000.pack())
        .lock(always_success_lock_script.clone())
        .type_(Some(checkpoint_type_script.clone()).pack())
        .build()];

    // prepare outputs_data
    let output_checkpoint_data = CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(1))
        .period(axon_u32(3))
        // .latest_block_hash(v)
        .latest_block_height(axon_u64(10))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        // .propose_count(v)
        .state_root(axon_byte32(&[0u8; 32].pack()))
        .timestamp(axon_u64(11111))
        .build();

    let outputs_data = vec![Bytes::from(output_checkpoint_data.as_bytes())];

    // prepare metadata cell_dep
    let bls_keypairs = vec![0; 8]
        .iter()
        .map(|_| random_bls_keypair())
        .collect::<Vec<_>>();
    let validators = vec![1u64; 8]
        .into_iter()
        .enumerate()
        .map(|(i, _era)| {
            let mut bls_pubkey = [0u8; 48];
            bls_pubkey.copy_from_slice(&bls_keypairs[i].1);
            Validator::new_builder()
                .bls_pub_key(axon_array48_byte48(bls_pubkey))
                .build()
        })
        .collect::<Vec<_>>();
    let validatorlist = ValidatorList::new_builder().set(validators).build();
    let metadata = Metadata::new_builder()
        .epoch_len(axon_u32(100))
        .validators(validatorlist)
        .build();
    // let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let metadata_cell_data = axon_metadata_data(
        &metadata_type_script.clone().calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(), // needless here
        metadata_list,
    );
    let metadata_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(metadata_type_script.clone()).pack())
                    .build(),
                metadata_cell_data.as_bytes(),
            ),
        )
        .build();

    let (proposal, proof) = mock_witness(&bls_keypairs);
    // prepare witness
    let witness_input_type = CheckpointWitness::new_builder()
        .proposal(axon_bytes(&proposal))
        .proof(axon_bytes(&proof))
        .build();
    let witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(witness_input_type.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .witness(witness.as_bytes().pack())
        .outputs_data(outputs_data.pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        // .cell_dep(secp256k1_data_dep)
        .cell_dep(metadata_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_checkpoint_create() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("checkpoint");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");

    let metadata_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![5]))
        .expect("metadata type script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // prepare stake_args and stake_data
    let _keypair = Generator::random_keypair();
    let checkpoint_args = CheckpointArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .build();
    let checkpoint_type_script = context
        .build_script(&contract_out_point, Bytes::from(checkpoint_args.as_bytes()))
        .expect("checkpoint script");
    println!(
        "checkpoint type hash: {:?}",
        checkpoint_type_script.calc_script_hash().as_slice()
    );

    // prepare tx inputs and outputs
    let inputs = vec![CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .build(),
                Bytes::from([0u8; 1].to_vec()),
            ),
        )
        .build()];
    let outputs = vec![CellOutput::new_builder()
        .capacity(1000.pack())
        .lock(always_success_lock_script.clone())
        .type_(Some(checkpoint_type_script.clone()).pack())
        .build()];

    // prepare outputs_data
    let output_checkpoint_data = CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(1))
        .period(axon_u32(3))
        // .latest_block_hash(v)
        .latest_block_height(axon_u64(10))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        // .propose_count(v)
        .state_root(axon_byte32(&[0u8; 32].pack()))
        .timestamp(axon_u64(11111))
        .build();

    let outputs_data = vec![Bytes::from(output_checkpoint_data.as_bytes())];

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data(
        &metadata_type_script.clone().calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(), // needless here
        metadata_list,
    );
    let metadata_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(metadata_type_script.clone()).pack())
                    .build(),
                meta_data.as_bytes(),
            ),
        )
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        // .cell_dep(secp256k1_data_dep)
        .cell_dep(metadata_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_checkpoint_axon_blst() {
    let private_keys = [
        [
            55 as u8, 170, 15, 137, 61, 5, 145, 74, 77, 239, 4, 96, 192, 169, 132, 211, 97, 21, 70,
            207, 178, 105, 36, 215, 167, 202, 110, 13, 185, 149, 10, 45,
        ],
        [
            56, 63, 207, 248, 104, 59, 129, 21, 227, 22, 19, 148, 155, 226, 66, 84, 180, 32, 79,
            251, 228, 60, 34, 116, 8, 167, 99, 52, 162, 227, 251, 50,
        ],
        [
            81, 206, 33, 100, 59, 145, 19, 71, 197, 213, 200, 92, 50, 61, 157, 84, 33, 129, 13,
            200, 159, 70, 182, 136, 114, 11, 39, 21, 245, 232, 233, 54,
        ],
        [
            105, 255, 81, 244, 194, 47, 48, 97, 95, 104, 184, 142, 250, 116, 15, 143, 27, 145, 105,
            232, 136, 66, 184, 61, 24, 151, 72, 208, 111, 26, 148, 142,
        ],
    ];
    let sk0 = SecretKey::from_bytes(private_keys[0].as_slice()).unwrap();
    let sk1 = SecretKey::from_bytes(private_keys[1].as_slice()).unwrap();
    // let sk2 = SecretKey::from_bytes(private_keys[2].as_slice()).unwrap();
    let sk3 = SecretKey::from_bytes(private_keys[3].as_slice()).unwrap();
    println!("sk0 pubkey: {:?}", sk0.sk_to_pk().to_bytes());
    println!("sk1 pubkey: {:?}", sk1.sk_to_pk().to_bytes());
    // println!("sk2 pubkey: {:?}", sk2.sk_to_pk().to_bytes());
    println!("sk3 pubkey: {:?}", sk3.sk_to_pk().to_bytes());

    let message = [
        228 as u8, 10, 128, 2, 160, 166, 132, 67, 59, 186, 214, 105, 136, 22, 3, 194, 228, 4, 233,
        114, 201, 143, 214, 101, 177, 122, 165, 253, 118, 57, 55, 55, 237, 163, 135, 104, 225,
    ];
    let message = keccak256(&message.to_vec());

    let bls_keypairs = [
        (
            sk0,
            [
                172 as u8, 133, 187, 180, 3, 71, 182, 224, 106, 194, 220, 45, 161, 247, 94, 236,
                224, 41, 205, 192, 237, 45, 69, 108, 69, 125, 39, 226, 136, 191, 191, 188, 212,
                197, 193, 151, 22, 233, 178, 80, 19, 74, 14, 118, 206, 80, 250, 34,
            ]
            .to_vec(),
        ),
        (
            sk1,
            [
                145, 237, 159, 60, 81, 197, 128, 229, 105, 72, 177, 189, 169, 208, 12, 33, 89, 102,
                95, 138, 110, 40, 65, 145, 171, 129, 110, 230, 78, 242, 72, 125, 120, 69, 58, 84,
                122, 15, 20, 239, 191, 132, 43, 186, 91, 90, 59, 79,
            ]
            .to_vec(),
        ),
        (
            sk3,
            [
                166, 148, 244, 228, 138, 90, 23, 59, 97, 115, 25, 152, 248, 241, 32, 67, 66, 220,
                92, 142, 177, 227, 44, 218, 227, 116, 21, 194, 13, 17, 174, 3, 93, 218, 196, 163,
                159, 16, 94, 156, 45, 77, 54, 145, 2, 77, 56, 93,
            ]
            .to_vec(),
        ),
    ];
    let signature = generate_bls_signature(&message, &bls_keypairs[..]);
    // println!("signature: {:?}, len: {},", signature, signature.len());
    let axon_signature = [
        172 as u8, 213, 108, 242, 233, 92, 3, 169, 148, 71, 184, 41, 255, 253, 253, 222, 214, 76,
        99, 223, 165, 60, 157, 108, 45, 200, 126, 102, 103, 237, 219, 250, 121, 12, 219, 64, 204,
        162, 10, 50, 180, 188, 154, 215, 247, 37, 1, 100, 13, 121, 53, 171, 22, 184, 88, 241, 19,
        152, 172, 178, 147, 130, 111, 236, 4, 45, 151, 164, 137, 134, 105, 231, 62, 178, 112, 235,
        72, 30, 42, 82, 22, 95, 208, 32, 9, 25, 54, 150, 77, 248, 43, 135, 222, 170, 229, 51,
    ];
    assert_eq!(signature, axon_signature);
}

fn mock_witness(bls_keypairs: &[(SecretKey, Vec<u8>)]) -> (Vec<u8>, Vec<u8>) {
    // prepare proposal rlp
    // refer to https://github.com/axonweb3/axon-tools/blob/main/axon-tools-riscv/src/types.rs#L76
    // only 10 fields are needed here
    let proposal = {
        let proposal_field_count = 10;
        let mut proposal = RlpStream::new_list(proposal_field_count);
        proposal.append_empty_data();
        proposal.append(&vec![0u8; 20]); // proposer_address
        vec![0; 8].iter().for_each(|_| {
            proposal.append_empty_data();
        });
        proposal.as_raw().to_vec()
    };

    // prepare proof rlp
    let proposal_hash = keccak_hash::keccak(proposal.clone());
    let message = {
        let mut vote = RlpStream::new_list(4);
        vote.append(&200u64);
        vote.append(&100u64);
        vote.append(&2u8);
        vote.append(&proposal_hash.as_bytes().to_vec());
        vote.as_raw().to_vec()
    };
    // println!(
    //     "generate_bls_signature raw message: {:?}, message len: {}",
    //     message.to_vec(),
    // );
    let message = keccak256(&message);
    // println!(
    //     "generate_bls_signature hash message: {:?}, message len: {}",
    //     message,
    // );
    let signature = generate_bls_signature(&message, &bls_keypairs[1..]);
    let mut bitmap = BitVec::from_elem(8, true);
    bitmap.set(0, false);
    let proof = {
        let mut proof = RlpStream::new_list(5);
        proof.append(&200u64);
        proof.append(&100u64);
        proof.append(&proposal_hash.as_bytes().to_vec());
        proof.append(&signature.to_vec());
        proof.append(&bitmap.to_bytes());
        proof.as_raw().to_vec()
    };
    (proposal, proof)
}

pub fn generate_bls_signature(message: &[u8], bls_keypairs: &[(SecretKey, Vec<u8>)]) -> [u8; 96] {
    let mut ref_signatures = vec![];
    let mut ref_pubkeys = vec![];
    for (privkey, _) in bls_keypairs.to_vec() {
        let signature = privkey.sign(&message, b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RONUL", &[]);
        // println!("sig: {:?}", signature.to_bytes());
        let pubkey = privkey.sk_to_pk();
        ref_signatures.push(signature);
        ref_pubkeys.push(pubkey);
    }
    let ref_signatures = ref_signatures.iter().collect::<Vec<_>>();
    let signature = AggregateSignature::aggregate(&ref_signatures.as_slice(), true)
        .unwrap()
        .to_signature();
    let ref_pubkeys = ref_pubkeys.iter().collect::<Vec<_>>();
    let pubkey = AggregatePublicKey::aggregate(&ref_pubkeys, false)
        .unwrap()
        .to_public_key();
    let result = signature.verify(
        true,
        &message,
        b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RONUL",
        &[],
        &pubkey,
        false,
    );
    assert!(
        result == blst::BLST_ERROR::BLST_SUCCESS,
        "pubkeys not match signatures"
    );
    signature.compress()
}

pub fn random_bls_keypair() -> (SecretKey, Vec<u8>) {
    let mut rng = thread_rng();
    let mut ikm = [0u8; 32];
    rng.fill_bytes(&mut ikm);
    let privkey = SecretKey::key_gen(&ikm, &[]).unwrap();
    let pubkey = privkey.sk_to_pk();
    (privkey, pubkey.compress().to_vec())
}
