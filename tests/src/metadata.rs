use crate::smt::{
    construct_epoch_smt, construct_lock_info_smt, construct_propose_count_smt, TopSmtInfo,
};
use std::collections::BTreeSet;
use std::convert::TryInto;
use std::iter::FromIterator;

use super::*;
use axon_types::checkpoint::{CheckpointCellData, ProposeCount, ProposeCounts};
use axon_types::metadata::{
    DelegateProof, DelegateProofs, ElectionSmtProof, Metadata, MetadataArgs, MetadataList,
    MetadataWitness, MinerGroupInfo, MinerGroupInfos, StakeSmtElectionInfo,
};
use ckb_testtool::ckb_crypto::secp::Generator;
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use util::helper::ProposeCountObject;
use util::smt::{u64_to_h256, LockInfo};

#[test]
fn test_metadata_creation_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("metadata");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");

    let metadata_args = MetadataArgs::new_builder()
        .metadata_type_id(axon_byte32(&[1u8; 32].pack()))
        .build();
    let metadata_type_script = context
        .build_script_with_hash_type(
            &contract_out_point,
            ScriptHashType::Type,
            Bytes::from(metadata_args.as_bytes()),
        )
        .expect("metadata type script");
    println!(
        "metadata type script: {:?}",
        metadata_type_script.calc_script_hash().as_bytes().to_vec()
    );
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let keypair = Generator::random_keypair();
    let staker_addr = pubkey_to_addr(&keypair.1.serialize());
    // prepare checkpoint lock_script
    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");
    let checkpoint_data = CheckpointCellData::new_builder().build();
    // prepare checkpoint cell_dep
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(checkpoint_type_script.clone()).pack())
                    .build(),
                checkpoint_data.as_bytes(),
            ),
        )
        .build();

    // prepare tx inputs and outputs
    // prepare metadata
    let metadata0 = Metadata::new_builder()
        .epoch_len(axon_u32(100))
        .quorum(axon_u16(2))
        .build();
    let metadata1 = metadata0.clone();
    let metadata2 = metadata0.clone();
    let metadata_list = MetadataList::new_builder()
        .push(metadata0)
        .push(metadata1)
        .push(metadata2)
        .build();
    println!(
        "checkpoint script: {:?}",
        checkpoint_type_script.calc_script_hash()
    );

    let inputs = vec![];
    let outputs = vec![
        // metadata cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(metadata_type_script.clone()).pack())
            .build(),
    ];

    let propose_count = ProposeCountObject {
        addr: staker_addr,
        count: 100 as u64,
    };
    let propose_infos = vec![propose_count];
    let (propose_count_root, _) = construct_propose_count_smt(&propose_infos);
    println!("propose_count_root: {:?}", propose_count_root);
    let top_smt_info = TopSmtInfo {
        epoch: 1,
        smt_root: propose_count_root,
    };
    let (top_smt_root, _proof) = construct_epoch_smt(&vec![top_smt_info]);

    let output_meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &metadata_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &metadata_type_script,
        &metadata_type_script,
        metadata_list,
        2,
        top_smt_root.as_slice().try_into().unwrap(),
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
    );

    let outputs_data = vec![output_meta_data.as_bytes()];

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(contract_dep)
        .cell_dep(checkpoint_script_dep)
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
fn test_metadata_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("metadata");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");

    let metadata_args = MetadataArgs::new_builder()
        .metadata_type_id(axon_byte32(&[1u8; 32].pack()))
        .build();
    let metadata_type_script = context
        .build_script_with_hash_type(
            &contract_out_point,
            ScriptHashType::Type,
            Bytes::from(metadata_args.as_bytes()),
        )
        .expect("metadata type script");
    println!(
        "metadata type script: {:?}",
        metadata_type_script.calc_script_hash()
    );
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let keypair = Generator::random_keypair();
    let staker_addr = pubkey_to_addr(&keypair.1.serialize());
    let propose_count = ProposeCount::new_builder()
        .address(axon_byte20(&staker_addr))
        .count(axon_u64(100))
        .build();
    let propose_counts = vec![propose_count];
    let propose_counts = ProposeCounts::new_builder().set(propose_counts).build();
    // prepare checkpoint lock_script
    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");
    println!(
        "checkpoint script: {:?}",
        checkpoint_type_script.calc_script_hash()
    );
    let current_epoch = 0;
    let epoch_len = 100;
    let checkpoint_data = CheckpointCellData::new_builder()
        .version(0.into())
        .epoch(axon_u64(current_epoch))
        .period(axon_u32(epoch_len))
        // .latest_block_hash(v)
        .latest_block_height(axon_u64(10))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .state_root(axon_byte32(&[0u8; 32].pack()))
        .timestamp(axon_u64(11111))
        .propose_count(propose_counts)
        .build();
    // println!("checkpoint data: {:?}", checkpoint_data.as_bytes().len());
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(checkpoint_type_script.clone()).pack())
                    .build(),
                checkpoint_data.as_bytes(),
            ),
        )
        .build();

    // prepare stake smt lock_script
    let stake_smt_args = axon_types::stake::StakeArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        // .stake_addr(axon_identity_none())
        .build();
    let stake_smt_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            stake_smt_args.as_bytes(),
        )
        .expect("stake smt type script");

    let delegate_smt_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![1]),
        )
        .expect("delegate smt type script");

    // prepare tx inputs and outputs
    let stake_amount = 1000;
    let input_stake_infos = BTreeSet::from_iter(vec![LockInfo {
        addr: staker_addr,
        amount: stake_amount,
    }]);
    let input_waiting_epoch = current_epoch + 2;
    let input_stake_smt_data = axon_stake_smt_cell_data(
        &input_stake_infos,
        &metadata_type_script.calc_script_hash(),
        input_waiting_epoch,
    );

    // prepare metadata
    let metadata0 = Metadata::new_builder()
        .epoch_len(axon_u32(epoch_len))
        .quorum(axon_u16(2))
        .build();
    let metadata1 = metadata0.clone();
    let metadata2 = metadata0.clone();
    let metadata_list = MetadataList::new_builder()
        .push(metadata0)
        .push(metadata1)
        .push(metadata2)
        .build();

    let propose_count_smt_root = [0u8; 32];
    let input_meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &stake_smt_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &stake_smt_type_script,
        &delegate_smt_type_script,
        metadata_list.clone(),
        input_waiting_epoch,
        propose_count_smt_root,
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
    );

    let delegate_infos = BTreeSet::new();
    let (delegate_smt_cell_data, delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &keypair.1,
        input_waiting_epoch,
    );

    let inputs = vec![
        // stake smt cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(stake_smt_type_script.clone()).pack())
                        .build(),
                    input_stake_smt_data.as_bytes(),
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
                    delegate_smt_cell_data.as_bytes(),
                ),
            )
            .build(),
        // metadata cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(always_success_lock_script.clone())
                        .type_(Some(metadata_type_script.clone()).pack())
                        .build(),
                    input_meta_data.as_bytes(),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // stake smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
        // metadata cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(metadata_type_script.clone()).pack())
            .build(),
    ];

    let output_stake_infos = input_stake_infos.clone();
    let output_waiting_epoch = input_waiting_epoch + 1;
    let output_stake_smt_data = axon_stake_smt_cell_data(
        &output_stake_infos,
        &metadata_type_script.calc_script_hash(),
        output_waiting_epoch,
    );
    let propose_count = ProposeCountObject {
        addr: staker_addr,
        count: 100 as u64,
    };
    let propose_infos = vec![propose_count];
    let (propose_count_root, _) = construct_propose_count_smt(&propose_infos);
    println!("propose_count_root: {:?}", propose_count_root);
    let top_smt_info = TopSmtInfo {
        epoch: current_epoch,
        smt_root: propose_count_root,
    };
    let (top_smt_root, proof) = construct_epoch_smt(&vec![top_smt_info]);
    let propose_count_proof = proof.compile(vec![u64_to_h256(1)]).unwrap().0;

    let output_meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &stake_smt_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &stake_smt_type_script,
        &delegate_smt_type_script,
        metadata_list,
        output_waiting_epoch,
        top_smt_root.as_slice().try_into().unwrap(),
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
    );

    let outputs_data = vec![
        output_stake_smt_data.as_bytes(), // stake smt cell
        delegate_smt_cell_data.as_bytes(),
        output_meta_data.as_bytes(),
    ];

    let stake_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(vec![2])).pack())
        .build();

    let (stake_root, _stake_proof) = construct_lock_info_smt(&input_stake_infos);
    let stake_top_smt_infos = vec![TopSmtInfo {
        epoch: output_waiting_epoch,
        smt_root: stake_root,
    }];
    let (_stake_root, staker_epoch_proof) = construct_epoch_smt(&stake_top_smt_infos);
    let staker_epoch_proof = staker_epoch_proof
        .compile(vec![u64_to_h256(output_waiting_epoch)])
        .unwrap()
        .0;

    let delegate_infos = axon_types::metadata::DelegateInfos::new_builder().build();
    let delegate_epoch_proof = delegate_epoch_proof.0;
    let miner_group_info = MinerGroupInfo::new_builder()
        .staker(axon_identity(&keypair.1.serialize()))
        .amount(axon_u128(stake_amount))
        .delegate_epoch_proof(axon_bytes(&delegate_epoch_proof))
        .delegate_infos(delegate_infos)
        .build();
    let miner_group_infos = MinerGroupInfos::new_builder()
        .push(miner_group_info)
        .build();
    let election_smt_proof = ElectionSmtProof::new_builder()
        .staker_epoch_proof(axon_bytes(&staker_epoch_proof))
        .miners(miner_group_infos)
        .build();
    let new_stake_proof = staker_epoch_proof;
    let new_delegate_proof = DelegateProof::new_builder()
        .staker(axon_identity(&keypair.1.serialize()))
        .proof(axon_bytes(&delegate_epoch_proof))
        .build();
    let new_delegate_proofs = DelegateProofs::new_builder()
        .push(new_delegate_proof)
        .build();
    let stake_smt_election_info = StakeSmtElectionInfo::new_builder()
        .n2(election_smt_proof)
        .new_stake_proof(axon_bytes(&new_stake_proof))
        .new_delegate_proofs(new_delegate_proofs)
        .build();
    let metadata_witness = MetadataWitness::new_builder()
        .new_propose_proof(axon_bytes(&propose_count_proof))
        .smt_election_info(stake_smt_election_info)
        .build();
    let metadata_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(metadata_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(vec![
            stake_smt_witness.as_bytes().pack(),
            Bytes::default().pack(),
            // metadata_witness.as_bytes().pack(),
            metadata_witness.as_bytes().pack(),
        ])
        .cell_dep(contract_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(always_success_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}
