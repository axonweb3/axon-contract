// use std::convert::TryInto;

use super::*;
use axon_types::checkpoint::CheckpointCellData;
use axon_types::metadata::MetadataList;
use axon_types::withdraw::{WithdrawArgs, WithdrawWitness};
// use bit_vec::BitVec;
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;

#[test]
fn test_withdraw_success() {
    // init context
    let mut context = Context::default();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let contract_bin: Bytes = Loader::default().load_binary("withdraw");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    let metadata_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2]),
        )
        .expect("metadata type script");

    let withdraw_lock_args = WithdrawArgs::new_builder()
        .addr(axon_identity(&[0u8; 20].to_vec()))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .build();
    let withdraw_lock_script = context
        .build_script(&contract_out_point, withdraw_lock_args.as_bytes())
        .expect("withdraw lock script");

    let at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![6]))
        .expect("sudt script");

    let input_withdraw_infos = vec![(3 as u64, 1000 as u128), (4, 2000), (5, 3000)];
    let input_withdraw_at_cell_data =
        axon_withdraw_at_cell_data_without_amount(input_withdraw_infos);
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(2000.pack())
                    .lock(withdraw_lock_script.clone())
                    .type_(Some(at_type_script.clone()).pack())
                    .build(),
                Bytes::from(axon_withdraw_at_cell_data(
                    6000,
                    input_withdraw_at_cell_data,
                )), // delegate at cell
            ),
        )
        .build();

    let output_withdraw_infos = vec![(4 as u64, 2000 as u128), (5, 3000)];
    let output_withdraw_at_cell_data =
        axon_withdraw_at_cell_data_without_amount(output_withdraw_infos);
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdraw_lock_script.clone())
            .type_(Some(at_type_script.clone()).pack())
            .build(),
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(at_type_script.clone()).pack())
            .build(),
    ];

    let outputs_data = vec![
        Bytes::from(axon_withdraw_at_cell_data(
            5000,
            output_withdraw_at_cell_data.clone(),
        )),
        Bytes::from(axon_normal_at_cell_data(
            1000,
            output_withdraw_at_cell_data.clone(),
        )),
    ];

    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![3]),
        )
        .expect("checkpoint script");

    // epoch must be 3, so that the reward of epoch 1 can be claimed
    let checkpoint_data = CheckpointCellData::new_builder().epoch(axon_u64(3)).build();
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

    let metadata_list = MetadataList::new_builder().build();
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &always_success_lock_script,
        &always_success_lock_script,
        metadata_list.clone(),
        3,
        100,
        100,
        [0u8; 32],
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
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

    let withdraw_witness = WithdrawWitness::new_builder()
        .signature(axon_bytes(&[0u8; 65].to_vec()))
        .build();
    let withdraw_witness = WitnessArgs::new_builder()
        .lock(Some(withdraw_witness.as_bytes()).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(withdraw_witness.as_bytes().pack())
        .cell_dep(contract_dep)
        .cell_dep(metadata_script_dep)
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
fn test_increase_withdraw_success() {
    // init context
    let mut context = Context::default();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let contract_bin: Bytes = Loader::default().load_binary("withdraw");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    let metadata_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![2]),
        )
        .expect("metadata type script");

    let withdraw_lock_args = WithdrawArgs::new_builder()
        .addr(axon_identity(&[0u8; 20].to_vec()))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .build();
    let withdraw_lock_script = context
        .build_script(&contract_out_point, withdraw_lock_args.as_bytes())
        .expect("withdraw lock script");

    let at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![6]))
        .expect("sudt script");

    let input_withdraw_infos = vec![(3 as u64, 1000 as u128), (4, 2000)];
    let input_withdraw_at_cell_data =
        axon_withdraw_at_cell_data_without_amount(input_withdraw_infos);
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(2000.pack())
                    .lock(withdraw_lock_script.clone())
                    .type_(Some(at_type_script.clone()).pack())
                    .build(),
                Bytes::from(axon_withdraw_at_cell_data(
                    3000,
                    input_withdraw_at_cell_data,
                )), // delegate at cell
            ),
        )
        .build();

    let output_withdraw_infos = vec![
        (3 as u64, 1000 as u128),
        (4 as u64, 2000 as u128),
        (5, 3000),
    ];
    let output_withdraw_at_cell_data =
        axon_withdraw_at_cell_data_without_amount(output_withdraw_infos);
    let outputs = vec![CellOutput::new_builder()
        .capacity(1000.pack())
        .lock(withdraw_lock_script.clone())
        .type_(Some(at_type_script.clone()).pack())
        .build()];

    let outputs_data = vec![Bytes::from(axon_withdraw_at_cell_data(
        6000,
        output_withdraw_at_cell_data.clone(),
    ))];

    let checkpoint_type_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![3]),
        )
        .expect("checkpoint script");

    // epoch must be 3, so that the reward of epoch 1 can be claimed
    let checkpoint_data = CheckpointCellData::new_builder().epoch(axon_u64(3)).build();
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

    let metadata_list = MetadataList::new_builder().build();
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &always_success_lock_script,
        &always_success_lock_script,
        metadata_list.clone(),
        3,
        100,
        100,
        [0u8; 32],
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
        &metadata_type_script.code_hash(),
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

    let withdraw_witness = WitnessArgs::new_builder().build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(withdraw_witness.as_bytes().pack())
        .cell_dep(contract_dep)
        .cell_dep(metadata_script_dep)
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
