// use std::collections::BTreeSet;
// use std::convert::TryInto;

// use crate::smt::{construct_epoch_smt, construct_lock_info_smt, u64_to_h256, TopSmtInfo};

use std::collections::BTreeSet;
use std::iter::FromIterator;

use super::*;
use axon_types::delegate::*;
use axon_types::metadata::{Metadata, MetadataList};
// use bit_vec::BitVec;
// use ckb_system_scripts::BUNDLED_CELL;
use ckb_testtool::ckb_crypto::secp::Generator;
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use util::smt::{new_blake2b, LockInfo};

#[test]
fn test_delegate_at_increase_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("delegate");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2u8; 32]),
        )
        .expect("checkpoint script");
    println!(
        "checkpoint type script: {:?}",
        checkpoint_type_script.calc_script_hash()
    );

    let delegate_at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("sudt script");
    let metadata_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![5]),
        )
        .expect("metadata type script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let delegator_keypair = Generator::random_keypair();
    let delegate_args = delegate::DelegateArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();

    let staker_keypair = Generator::random_keypair();
    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(0 as u128))
        .total_amount(axon_u128(0 as u128))
        .inauguration_epoch(axon_u64(0 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let input_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![input_delegate_info_delta.clone()])
        .build();
    let input_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        input_delegate_info_deltas,
    );

    // prepare stake lock_script
    let delegate_at_lock_script = context
        .build_script(&contract_out_point, delegate_args.as_bytes())
        .expect("stake script");

    // prepare tx inputs and outputs
    let inputs = vec![
        // delegate AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(delegate_at_lock_script.clone())
                        .type_(Some(delegate_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_delegate_at_cell_data(0, input_delegate_at_data)),
                ),
            )
            .build(),
        // normal AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(delegate_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from((1000 as u128).to_le_bytes().to_vec()),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // delegate at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(delegate_at_lock_script)
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
        // normal at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(100 as u128))
        .total_amount(axon_u128(100 as u128))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![output_delegate_info_delta.clone()])
        .build();
    let output_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        output_delegate_info_deltas,
    );

    let outputs_data = vec![
        Bytes::from(axon_delegate_at_cell_data(100, output_delegate_at_data)), // stake at cell
        Bytes::from((900 as u128).to_le_bytes().to_vec()),                     // normal at cell
                                                                               // Bytes::from(axon_withdrawal_data(50, 2)),
    ];

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let propose_count_smt_root = [0u8; 32];
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &delegate_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &delegate_at_type_script, // needless here
        &delegate_at_type_script, // needless here
        metadata_list,
        1,
        propose_count_smt_root,
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
    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(&metadata_type_script.clone().calc_script_hash());
    println!("checkpoint data: {:?}", checkpoint_data.as_bytes().len());
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(checkpoint_type_script).pack())
                    .build(),
                checkpoint_data.as_bytes(),
            ),
        )
        .build();

    let delegate_at_witness = DelegateAtWitness::new_builder().mode(0.into()).build();
    println!(
        "delegate at witness: {:?}",
        delegate_at_witness.as_bytes().len()
    );
    let delegate_at_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(delegate_at_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .witness(delegate_at_witness.as_bytes().pack())
        .outputs_data(outputs_data.pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // sign tx for delegate at cell (update stake at cell delta mode)
    // let tx = sign_tx(tx, &delegator_keypair.0, 0);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_smt_success() {
    // init context
    let mut context = Context::default();

    let at_contract_bin: Bytes = Loader::default().load_binary("delegate");
    let at_contract_out_point = context.deploy_cell(at_contract_bin);
    let at_contract_dep = CellDep::new_builder()
        .out_point(at_contract_out_point.clone())
        .build();
    let smt_contract_bin: Bytes = Loader::default().load_binary("delegate-smt");
    let smt_contract_out_point = context.deploy_cell(smt_contract_bin);
    let smt_contract_dep = CellDep::new_builder()
        .out_point(smt_contract_out_point.clone())
        .build();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2u8; 32]),
        )
        .expect("checkpoint script");
    println!(
        "checkpoint type script: {:?}",
        checkpoint_type_script
            .calc_script_hash()
            .as_bytes()
            .to_vec()
    );

    let delegate_at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("sudt script");
    let metadata_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![5]),
        )
        .expect("metadata type script");
    println!(
        "metadata type script: {:?}",
        metadata_type_script.calc_script_hash().as_bytes().to_vec()
    );
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    println!(
        "staker pubkey: {:?}",
        blake160(&staker_keypair.1.serialize())
    );
    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(100 as u128))
        .total_amount(axon_u128(100 as u128))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let input_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![input_delegate_info_delta.clone()])
        .build();
    let input_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        input_delegate_info_deltas,
    );

    // prepare stake lock_script
    let delegate_at_args = delegate::DelegateArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();
    let delegate_at_lock_script = context
        .build_script(&at_contract_out_point, delegate_at_args.as_bytes())
        .expect("delegate script");

    // let delegate_smt_args = delegate::DelegateArgs::new_builder()
    //     .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
    //     .delegator_addr(axon_identity_none())
    //     .build();
    let delegate_smt_type_script = context
        .build_script_with_hash_type(
            &smt_contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![3u8; 32]),
        )
        .expect("delegate smt type script");
    let delegate_infos = BTreeSet::new();
    let (input_delegate_smt_cell_data, input_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
    );

    // prepare tx inputs and outputs
    let inputs = vec![
        // delegate AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(delegate_at_lock_script.clone())
                        .type_(Some(delegate_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_delegate_at_cell_data(1000, input_delegate_at_data)),
                ),
            )
            .build(),
        // delegate smt cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(delegate_smt_type_script.clone()).pack())
                        .build(),
                    input_delegate_smt_cell_data.as_bytes(),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // delegate at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(delegate_at_lock_script)
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(0 as u128))
        .total_amount(axon_u128(0 as u128))
        .inauguration_epoch(axon_u64(0 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![output_delegate_info_delta.clone()])
        .build();
    let output_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        output_delegate_info_deltas,
    );

    let delegate_addr = pubkey_to_addr(&delegator_keypair.1.serialize());
    let output_delegate_infos = BTreeSet::from_iter(vec![LockInfo {
        addr: delegate_addr,
        amount: 1000,
    }]);
    let (output_delegate_smt_cell_data, output_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &output_delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
    );

    let outputs_data = vec![
        Bytes::from(axon_delegate_at_cell_data(1000, output_delegate_at_data)), // delegate at cell
        output_delegate_smt_cell_data.as_bytes(),                               // delegate smt cell
                                                                                // Bytes::from(axon_withdrawal_data(50, 2)),
    ];

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let propose_count_smt_root = [0u8; 32];
    println!(
        "delegate smt type hash: {:?}",
        delegate_smt_type_script
            .calc_script_hash()
            .as_bytes()
            .to_vec()
    );
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &delegate_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &delegate_at_type_script,  // needless here
        &delegate_smt_type_script, // needless here
        metadata_list,
        1,
        propose_count_smt_root,
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
    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(&metadata_type_script.clone().calc_script_hash());
    println!("checkpoint data: {:?}", checkpoint_data.as_bytes().len());
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(checkpoint_type_script).pack())
                    .build(),
                checkpoint_data.as_bytes(),
            ),
        )
        .build();

    let delegate_info = DelegateInfo::new_builder()
        .amount(axon_u128(1000))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();
    let delegate_infos = DelegateInfos::new_builder().push(delegate_info).build();
    let stake_group_info = StakeGroupInfo::new_builder()
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .delegate_infos(delegate_infos)
        .delegate_old_epoch_proof(axon_bytes(&input_delegate_epoch_proof.0))
        .delegate_new_epoch_proof(axon_bytes(&output_delegate_epoch_proof.0))
        .build();
    let stake_group_infos = StakeGroupInfos::new_builder()
        .push(stake_group_info)
        .build();
    let delegate_smt_update_info = DelegateSmtUpdateInfo::new_builder()
        .all_stake_group_infos(stake_group_infos)
        .build();
    println!(
        "delegate smt update info: {:?}, {}",
        delegate_smt_update_info.as_bytes().pack(),
        delegate_smt_update_info.as_bytes().len()
    );

    let delegate_at_witness = DelegateAtWitness::new_builder().mode(1.into()).build();
    let delegate_at_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(delegate_at_witness.as_bytes())).pack())
        .build();

    let delegate_smt_witness = DelegateSmtWitness::new_builder()
        .mode(0.into())
        .update_info(delegate_smt_update_info)
        .build();
    let delegate_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(delegate_smt_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(vec![
            delegate_at_witness.as_bytes().pack(),
            delegate_smt_witness.as_bytes().pack(),
        ])
        .cell_dep(at_contract_dep)
        .cell_dep(smt_contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // sign tx for stake at cell (update stake at cell delta mode)
    // let tx = sign_tx(tx, &delegator_keypair.0, 0);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_smt_create_success() {
    // init context
    let mut context = Context::default();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let smt_contract_bin: Bytes = Loader::default().load_binary("delegate-smt");
    let smt_contract_out_point = context.deploy_cell(smt_contract_bin);
    let smt_contract_dep = CellDep::new_builder()
        .out_point(smt_contract_out_point.clone())
        .build();

    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(500.pack())
                    .lock(always_success_lock_script.clone())
                    .build(),
                Bytes::new(),
            ),
        )
        .build();

    let input_hash = {
        let mut blake2b = new_blake2b();
        blake2b.update(input.as_slice());
        blake2b.update(&0u64.to_le_bytes());
        let mut ret = [0; 32];
        blake2b.finalize(&mut ret);
        Bytes::from(ret.to_vec())
    };
    // let input_hash = {
    //     let ret = [0; 32];
    //     Bytes::from(ret.to_vec())
    // };
    let delegate_smt_type_script = context
        .build_script(&smt_contract_out_point, input_hash)
        .expect("delegate smt type script");
    println!(
        "delegate_smt_type_script: {:?}",
        delegate_smt_type_script
            .calc_script_hash()
            .as_bytes()
            .to_vec()
    );

    let outputs = vec![
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
    ];

    let delegate_addr = [0u8; 20];
    let output_delegate_infos = BTreeSet::from_iter(vec![LockInfo {
        addr: delegate_addr,
        amount: 1000,
    }]);
    let staker_keypair = Generator::random_keypair();
    let (output_delegate_smt_cell_data, _output_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &output_delegate_infos,
        &always_success_lock_script.calc_script_hash(),
        &staker_keypair.1,
    );
    println!(
        "output delegate smt data: {:?}",
        output_delegate_smt_cell_data.as_bytes().len()
    );
    let outputs_data = vec![
        output_delegate_smt_cell_data.as_bytes(), // stake smt cell
    ];

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(smt_contract_dep)
        .cell_dep(always_success_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_election_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("delegate-smt");
    let smt_contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(smt_contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");
    println!(
        "checkpoint type hash: {:?}",
        checkpoint_type_script.calc_script_hash().as_slice()
    );

    let metadata_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![5]),
        )
        .expect("metadata type script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // prepare stake lock_script
    // let stake_smt_args = stake::StakeArgs::new_builder()
    //     .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
    //     // .stake_addr(axon_identity_none())
    //     .build();
    let delegate_smt_type_script = context
        .build_script_with_hash_type(
            &smt_contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![6u8; 32]),
        )
        .expect("stake smt type script");

    let staker_keypair = Generator::random_keypair();
    // prepare tx inputs and outputs
    let delegate_infos = BTreeSet::new();
    let (input_delegate_smt_cell_data, _input_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
    );
    println!(
        "metadata_type_script:{:?}",
        metadata_type_script.calc_script_hash().as_bytes().to_vec()
    );

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data(
        &metadata_type_script.clone().calc_script_hash(),
        &delegate_smt_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &delegate_smt_type_script.calc_script_hash(),
        metadata_list,
    );

    let inputs = vec![
        // stake smt cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(delegate_smt_type_script.clone()).pack())
                        .build(),
                    input_delegate_smt_cell_data.as_bytes(),
                ),
            )
            .build(),
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(metadata_type_script.clone()).pack())
                        .build(),
                    meta_data.as_bytes(),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(metadata_type_script.clone()).pack())
            .build(),
    ];

    let delegate_addr = [0u8; 20];
    let output_delegate_infos = BTreeSet::from_iter(vec![LockInfo {
        addr: delegate_addr,
        amount: 1000,
    }]);
    let (output_delegate_smt_cell_data, _output_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &output_delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
    );

    let outputs_data = vec![
        output_delegate_smt_cell_data.as_bytes(), // delegate smt cell
        meta_data.as_bytes(),
    ];

    let delegate_smt_witness = DelegateSmtWitness::new_builder().mode(1.into()).build();
    let delegate_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(delegate_smt_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(vec![delegate_smt_witness.as_bytes().pack()])
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}
