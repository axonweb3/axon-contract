use std::collections::BTreeSet;
use std::iter::FromIterator;

use super::*;
use axon_types::delegate::*;
use axon_types::metadata::{Metadata, MetadataList};
use axon_types::withdraw::WithdrawArgs;
use ckb_testtool::ckb_crypto::secp::{Generator, Privkey, Pubkey};
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{
    bytes::Bytes, core::TransactionBuilder, core::TransactionView, packed::*, prelude::*,
};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use util::{
    error::Error::{
        DelegateSelf, DelegateSmtVerifySelectionError, InputOutputAtAmountNotEqual,
        UnDelegateTooMuch,
    },
    smt::LockInfo,
};

// newly added delegate info
fn construct_delegate_tx(context: &mut Context) -> TransactionView {
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let output_at_amount = 1000;
    let output_delegate_at_amount = 100;
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    construct_delegate_tx_with_args(
        context,
        delegator_keypair,
        None,
        output_delegate_info_delta,
        output_at_amount,
        0,
        output_at_amount - output_delegate_at_amount,
        output_delegate_at_amount,
    )
}

fn construct_delegate_tx_with_args(
    context: &mut Context,
    delegator_keypair: (Privkey, Pubkey),
    input_delegate_info_delta: Option<delegate::DelegateInfoDelta>,
    output_delegate_info_delta: delegate::DelegateInfoDelta,
    input_normal_at_amount: u128,
    input_delegate_at_amount: u128,
    output_normal_at_amount: u128,
    output_delegate_at_amount: u128,
) -> TransactionView {
    let current_epoch = 1;

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

    let delegate_args = delegate::DelegateArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();

    let input_delegate_info_deltas =
        if let Some(input_delegate_info_delta) = input_delegate_info_delta {
            DelegateInfoDeltas::new_builder()
                .push(input_delegate_info_delta)
                .build()
        } else {
            DelegateInfoDeltas::new_builder().build()
        };

    let input_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
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
                    Bytes::from(axon_delegate_at_cell_data(
                        input_delegate_at_amount,
                        input_delegate_at_data,
                    )),
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
                    Bytes::from((input_normal_at_amount).to_le_bytes().to_vec()),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // delegate at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(delegate_at_lock_script.clone())
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
    let output_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![output_delegate_info_delta.clone()])
        .build();
    let output_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        output_delegate_info_deltas,
    );

    let outputs_data = vec![
        Bytes::from(axon_delegate_at_cell_data(
            output_delegate_at_amount,
            output_delegate_at_data,
        )), // stake at cell
        Bytes::from((output_normal_at_amount).to_le_bytes().to_vec()), // normal at cell
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
        current_epoch,
        100,
        100,
        propose_count_smt_root,
        &metadata_type_script.code_hash(),
        &delegate_at_lock_script.code_hash(),
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
    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(
        &metadata_type_script.clone().calc_script_hash(),
        current_epoch,
    );
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
    tx
}

#[test]
fn test_delegate_at_success_add_new() {
    // init context
    let mut context = Context::default();
    let tx = construct_delegate_tx(&mut context);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_at_success_increase_existing() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 900;
    let output_delegate_at_amount = 200;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_at_success_decrease_increase_more() {
    // existing decrease, but increase more at
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let delegate_amount = 200;
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 800;
    let output_delegate_at_amount = input_delegate_at_amount + delegate_amount;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(300))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(delegate_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_at_success_stale_increase_increase() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 900;
    let output_delegate_at_amount = 200;

    let waiting_epoch: u64 = 3;
    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_delegate_at_amount))
        .inauguration_epoch(axon_u64(waiting_epoch - 1))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(waiting_epoch))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_at_success_stale_decrease_increase() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 900;
    let output_delegate_at_amount = 200;

    let waiting_epoch: u64 = 3;
    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(500))
        .inauguration_epoch(axon_u64(waiting_epoch - 1))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(
            output_delegate_at_amount - input_delegate_at_amount,
        ))
        .inauguration_epoch(axon_u64(waiting_epoch))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_at_fail_increase_existing_more() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 1000;
    let output_delegate_at_amount = 200;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("InputOutputAtAmountNotEqual");
    assert_script_error(err, InputOutputAtAmountNotEqual as i8);
}

#[test]
fn test_delegate_self_fail() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = delegator_keypair.clone();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        None,
        output_delegate_info_delta,
        1000,
        0,
        900,
        100,
    );

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("DelegateSelf");
    assert_script_error(err, DelegateSelf as i8);
}

#[test]
fn test_undelegate_at_fail_too_much_at() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 1000;
    let output_delegate_at_amount = 100;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_delegate_at_amount + 100))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_delegate_at_amount + 300))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("UnDelegateTooMuch");
    assert_script_error(err, UnDelegateTooMuch as i8);
}

#[test]
fn test_undelegate_at_success_decrease_decrease() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 1000;
    let output_normal_at_amount = 1000;
    let output_delegate_at_amount = 1000;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(200))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(500))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_undelegate_at_success_increase_decrease_more() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 1000;
    let output_normal_at_amount = 1500;
    let output_delegate_at_amount = 500;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(500))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(500))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_undelegate_at_success_increase_decrease_less() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 1000;
    let output_normal_at_amount = 1500;
    let output_delegate_at_amount = 500;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_undelegate_at_success_stale_increase_decrease_less() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 1000;
    let delegate_amount = 800;
    let output_normal_at_amount = input_delegate_at_amount + delegate_amount;
    let output_delegate_at_amount = input_delegate_at_amount - delegate_amount;
    let wait_epoch: u64 = 3;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(delegate_amount))
        .inauguration_epoch(axon_u64(wait_epoch - 1))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(output_delegate_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_undelegate_at_success_stale_decrease_decrease_less() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let staker_keypair = Generator::random_keypair();
    let input_normal_at_amount = 1000;
    let input_delegate_at_amount = 100;
    let output_normal_at_amount = 1000;
    let output_delegate_at_amount = 100;
    let wait_epoch: u64 = 3;

    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(20))
        .inauguration_epoch(axon_u64(wait_epoch - 1))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let output_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(100))
        .inauguration_epoch(axon_u64(3 as u64))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();

    let tx = construct_delegate_tx_with_args(
        &mut context,
        delegator_keypair,
        Some(input_delegate_info_delta),
        output_delegate_info_delta,
        input_normal_at_amount,
        input_delegate_at_amount,
        output_normal_at_amount,
        output_delegate_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

// common tx for update undelegate tx to delegate smt cells
fn construct_delegate_smt_undelegate_tx(context: &mut Context) -> TransactionView {
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
    let staker_addr = pubkey_to_addr(&staker_keypair.1.serialize());
    println!(
        "staker pubkey: {:?}",
        blake160(&staker_keypair.1.serialize())
    );
    let inauguration_epoch = 3;
    let new_undelegate_amount = 10 as u128;
    let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(new_undelegate_amount))
        .inauguration_epoch(axon_u64(inauguration_epoch))
        .staker(axon_identity(&staker_keypair.1.serialize()))
        .build();
    let input_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
        .set(vec![input_delegate_info_delta.clone()])
        .build();
    let input_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        input_delegate_info_deltas,
    );

    // prepare delegate at lock_script
    let delegate_at_args = delegate::DelegateArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();
    let delegate_at_lock_script = context
        .build_script(&at_contract_out_point, delegate_at_args.as_bytes())
        .expect("delegate script");

    let delegate_smt_type_script = context
        .build_script_with_hash_type(
            &smt_contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![3u8; 32]),
        )
        .expect("delegate smt type script");
    let old_delegate_amount = 100;
    let old_delegate_info = LockInfo {
        addr: blake160(delegator_keypair.1.serialize().as_slice()),
        amount: old_delegate_amount,
    };
    let delegate_infos = BTreeSet::from([old_delegate_info]);

    let (input_delegate_smt_cell_data, input_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
        inauguration_epoch,
    );

    let withdraw_lock_args = WithdrawArgs::new_builder()
        .addr(axon_identity(&delegator_keypair.1.serialize()))
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .build();
    let withdraw_lock_script = context
        .build_script_with_hash_type(
            &always_success_out_point,
            ScriptHashType::Type,
            withdraw_lock_args.as_bytes(),
        )
        .expect("withdraw lock script");
    println!(
        "withdraw_lock_script code hash: {:?}, addr: {:?}, metadata_type_id: {:?}, args: {:?}",
        withdraw_lock_script.code_hash().as_slice(),
        axon_identity(&delegator_keypair.1.serialize()).as_slice(),
        metadata_type_script.calc_script_hash().as_slice(),
        withdraw_lock_args.as_slice()
    );
    let input_withdraw_infos = vec![
        (inauguration_epoch - 2 as u64, 0 as u128),
        (inauguration_epoch - 1, 0),
        (inauguration_epoch, 0),
    ];
    let input_withdraw_data = axon_withdraw_at_cell_data_without_amount(input_withdraw_infos);
    let input_withdraw_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdraw_lock_script.clone())
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
        Bytes::from(axon_withdraw_at_cell_data(0, input_withdraw_data)), // delegate at cell
    );

    let (delegate_requirement_script_dep, stake_at_script_dep, stake_at_lock_script) =
        axon_delegate_requirement_and_stake_at_cell(
            &metadata_type_script,
            &always_success_out_point,
            &always_success_lock_script,
            context,
            &staker_keypair,
            &staker_addr,
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
        // withdraw at cell
        CellInput::new_builder()
            .previous_output(input_withdraw_out_point)
            .build(),
    ];
    let outputs = vec![
        // delegate at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(delegate_at_lock_script.clone())
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
        // withdraw at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdraw_lock_script.clone())
            .type_(Some(delegate_at_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    let output_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder().build();
    let output_delegate_at_data = axon_delegate_at_cell_data_without_amount(
        0,
        &delegator_keypair.1.serialize(),
        &delegator_keypair.1.serialize(),
        &metadata_type_script.calc_script_hash(),
        output_delegate_info_deltas,
    );

    let new_delegate_info = LockInfo {
        addr: pubkey_to_addr(&delegator_keypair.1.serialize()),
        amount: old_delegate_amount - new_undelegate_amount,
    };
    let output_delegate_infos = BTreeSet::from_iter(vec![new_delegate_info]);
    let (output_delegate_smt_cell_data, output_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &output_delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
        inauguration_epoch,
    );

    let output_withdraw_infos = vec![
        (inauguration_epoch - 2 as u64, 0 as u128),
        (inauguration_epoch - 1, 0),
        (inauguration_epoch, new_undelegate_amount),
    ];
    let output_withdraw_data = axon_withdraw_at_cell_data_without_amount(output_withdraw_infos);

    let outputs_data = vec![
        Bytes::from(axon_delegate_at_cell_data(
            old_delegate_amount - new_undelegate_amount,
            output_delegate_at_data,
        )), // delegate at cell
        output_delegate_smt_cell_data.as_bytes(), // delegate smt cell
        Bytes::from(axon_withdraw_at_cell_data(
            new_undelegate_amount,
            output_withdraw_data,
        )), // withdraw at cell
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
        100,
        100,
        propose_count_smt_root,
        &stake_at_lock_script.code_hash(),
        &delegate_at_lock_script.code_hash(),
        &withdraw_lock_script.code_hash(),
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
    let checkpoint_data = axon_checkpoint_data(&metadata_type_script.clone().calc_script_hash(), 1);
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

    let old_delegate_info = DelegateInfo::new_builder()
        .amount(axon_u128(old_delegate_amount))
        .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
        .build();
    let delegate_infos = DelegateInfos::new_builder().push(old_delegate_info).build();
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
        .cell_dep(delegate_requirement_script_dep)
        .cell_dep(stake_at_script_dep)
        .build();
    let tx = context.complete_tx(tx);
    tx
}

#[test]
fn test_delegate_smt_redeem_success() {
    // init context
    let mut context = Context::default();
    let tx = construct_delegate_smt_undelegate_tx(&mut context);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

fn construct_delegate_smt_delegate_tx(
    context: &mut Context,
    delegators: Vec<((Privkey, Pubkey), u128)>,
    input_delegate_infos: BTreeSet<LockInfo>,
    intend_fail: bool,
) -> TransactionView {
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

    let staker_keypair = Generator::random_keypair();
    let staker_addr = pubkey_to_addr(&staker_keypair.1.serialize());
    println!(
        "staker pubkey: {:?}",
        blake160(&staker_keypair.1.serialize())
    );

    let current_epoch = 1u64;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut output_datas = Vec::new();
    let mut output_delegate_infos = BTreeSet::new();
    let mut witnesses = Vec::new();
    for delegator in delegators {
        let delegator_keypair = delegator.0;
        let delegate_amount = delegator.1;
        let input_delegate_info_delta = delegate::DelegateInfoDelta::new_builder()
            .is_increase(1.into())
            .amount(axon_u128(delegate_amount))
            .inauguration_epoch(axon_u64(current_epoch + 2))
            .staker(axon_identity(&staker_keypair.1.serialize()))
            .build();
        let input_delegate_info_deltas: DelegateInfoDeltas = DelegateInfoDeltas::new_builder()
            .set(vec![input_delegate_info_delta.clone()])
            .build();
        let input_delegate_at_data = axon_delegate_at_cell_data_without_amount(
            0,
            &delegator_keypair.1.serialize(),
            &delegator_keypair.1.serialize(),
            &metadata_type_script.calc_script_hash(),
            input_delegate_info_deltas,
        );
        let input_delegate_at_data = Bytes::from(axon_delegate_at_cell_data(
            delegate_amount,
            input_delegate_at_data,
        ));

        // prepare delegate at lock_script
        let delegate_at_args = delegate::DelegateArgs::new_builder()
            .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
            .delegator_addr(axon_identity(&delegator_keypair.1.serialize()))
            .build();
        let delegate_at_lock_script = context
            .build_script(&at_contract_out_point, delegate_at_args.as_bytes())
            .expect("delegate script");

        let input_delegate_at_cell = CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(delegate_at_lock_script.clone())
                        .type_(Some(delegate_at_type_script.clone()).pack())
                        .build(),
                    input_delegate_at_data.clone(),
                ),
            )
            .build();
        inputs.push(input_delegate_at_cell);

        // the 1st record is the lowest, so not selected
        if !intend_fail && delegate_amount > 1000 {
            let output_delegate_at_cell = CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(delegate_at_lock_script.clone())
                .type_(Some(delegate_at_type_script.clone()).pack())
                .build();
            outputs.push(output_delegate_at_cell);

            let output_delegate_info_deltas: DelegateInfoDeltas =
                DelegateInfoDeltas::new_builder().build();
            let output_delegate_at_data = axon_delegate_at_cell_data_without_amount(
                0,
                &delegator_keypair.1.serialize(),
                &delegator_keypair.1.serialize(),
                &metadata_type_script.calc_script_hash(),
                output_delegate_info_deltas,
            );
            let output_delegate_at_data = Bytes::from(axon_delegate_at_cell_data(
                delegate_amount,
                output_delegate_at_data,
            )); // delegate at cell
            output_datas.push(output_delegate_at_data);

            let delegate_addr = pubkey_to_addr(&delegator_keypair.1.serialize());
            let output_delegate_info = LockInfo {
                addr: delegate_addr,
                amount: delegate_amount,
            };
            output_delegate_infos.insert(output_delegate_info);
        } else {
            let output_delegate_at_cell = CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(delegate_at_lock_script.clone())
                .type_(Some(delegate_at_type_script.clone()).pack())
                .build();
            outputs.push(output_delegate_at_cell);
            output_datas.push(input_delegate_at_data);
        }

        let delegate_at_witness = DelegateAtWitness::new_builder().mode(1.into()).build();
        let delegate_at_witness = WitnessArgs::new_builder()
            .lock(Some(Bytes::from(delegate_at_witness.as_bytes())).pack())
            .build();
        witnesses.push(delegate_at_witness.as_bytes().pack());
    }

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
    let (input_delegate_smt_cell_data, input_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &input_delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
        current_epoch + 2,
    );

    let (delegate_requirement_script_dep, stake_at_script_dep, stake_at_lock_script) =
        axon_delegate_requirement_and_stake_at_cell(
            &metadata_type_script,
            &always_success_out_point,
            &always_success_lock_script,
            context,
            &staker_keypair,
            &staker_addr,
        );

    // prepare tx inputs and outputs
    inputs.push(
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
    );

    outputs.push(
        // delegate smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(delegate_smt_type_script.clone()).pack())
            .build(),
    );

    let (output_delegate_smt_cell_data, output_delegate_epoch_proof) = axon_delegate_smt_cell_data(
        &output_delegate_infos,
        &metadata_type_script.calc_script_hash(),
        &staker_keypair.1,
        current_epoch + 2,
    );
    output_datas.push(
        output_delegate_smt_cell_data.as_bytes(), // delegate smt cell
    );

    // this indicates the specific case to withdraw AT updated to smt
    if input_delegate_infos.len() > 0 {
        // input_delegate_infos contains only 1 that needs withdraw
        let withdraw_lock_args = WithdrawArgs::new_builder()
            .addr(axon_byte20_identity(
                &input_delegate_infos.first().unwrap().addr,
            ))
            .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
            .build();
        let withdraw_lock_script = context
            .build_script_with_hash_type(
                &always_success_out_point,
                ScriptHashType::Type,
                withdraw_lock_args.as_bytes(),
            )
            .expect("withdraw lock script");
        println!(
            "withdraw_lock_script code hash: {:?}, addr: {:?}, metadata_type_id: {:?}, args: {:?}",
            withdraw_lock_script.code_hash().as_slice(),
            axon_byte20_identity(&input_delegate_infos.first().unwrap().addr).as_slice(),
            metadata_type_script.calc_script_hash().as_slice(),
            withdraw_lock_args.as_slice()
        );
        let input_withdraw_infos = vec![
            (current_epoch, 0 as u128),
            (current_epoch + 1, 0),
            (current_epoch + 2, 0),
        ];
        let input_withdraw_data = axon_withdraw_at_cell_data_without_amount(input_withdraw_infos);
        let input_withdraw_out_point = context.create_cell(
            CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(withdraw_lock_script.clone())
                .type_(Some(delegate_at_type_script.clone()).pack())
                .build(),
            Bytes::from(axon_withdraw_at_cell_data(0, input_withdraw_data)), // delegate at cell
        );

        inputs.push(
            // withdraw at cell
            CellInput::new_builder()
                .previous_output(input_withdraw_out_point)
                .build(),
        );
        outputs.push(
            // withdraw at cell
            CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(withdraw_lock_script.clone())
                .type_(Some(delegate_at_type_script.clone()).pack())
                .build(),
        );
        let output_withdraw_infos = vec![
            (current_epoch, 0 as u128),
            (current_epoch + 1, 0),
            (
                current_epoch + 2,
                input_delegate_infos.first().unwrap().amount,
            ),
        ];
        let output_withdraw_data = axon_withdraw_at_cell_data_without_amount(output_withdraw_infos);
        let output_withdraw_data = Bytes::from(axon_withdraw_at_cell_data(
            input_delegate_infos.first().unwrap().amount,
            output_withdraw_data,
        )); // withdraw at cell
        output_datas.push(
            output_withdraw_data, // delegate smt cell
        );
    }
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
    let delegate_at_lock_script = context
        .build_script(&at_contract_out_point, Bytes::from(vec![9u8]))
        .expect("delegate script");
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &delegate_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &delegate_at_type_script,  // needless here
        &delegate_smt_type_script, // needless here
        metadata_list,
        current_epoch,
        100,
        100,
        propose_count_smt_root,
        &stake_at_lock_script.code_hash(),
        &delegate_at_lock_script.code_hash(),
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
    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(&metadata_type_script.clone().calc_script_hash(), 1);
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

    let mut delegate_infos = Vec::new();
    for info in input_delegate_infos {
        let delegate_info = DelegateInfo::new_builder()
            .amount(axon_u128(info.amount))
            .delegator_addr(axon_byte20_identity(&info.addr))
            .build();
        delegate_infos.push(delegate_info)
    }
    let delegate_infos = DelegateInfos::new_builder().set(delegate_infos).build();
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

    let delegate_smt_witness = DelegateSmtWitness::new_builder()
        .mode(0.into())
        .update_info(delegate_smt_update_info)
        .build();
    let delegate_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(delegate_smt_witness.as_bytes())).pack())
        .build();
    witnesses.push(delegate_smt_witness.as_bytes().pack());

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(output_datas.pack())
        .witnesses(witnesses)
        .cell_dep(at_contract_dep)
        .cell_dep(smt_contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        .cell_dep(delegate_requirement_script_dep)
        .cell_dep(stake_at_script_dep)
        .build();
    let tx = context.complete_tx(tx);
    tx
}

#[test]
fn test_delegate_smt_increase_success() {
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    // must larger than 1000, to simpilfy test
    let delegate_amount = 2000 as u128;
    let delegator_keypair1 = Generator::random_keypair();
    let delegate_amount1 = 2000 as u128;

    let input_delegate_infos = BTreeSet::new();

    let delegators = vec![
        (delegator_keypair, delegate_amount),
        (delegator_keypair1, delegate_amount1),
    ];
    let tx =
        construct_delegate_smt_delegate_tx(&mut context, delegators, input_delegate_infos, false);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_smt_increase_success_toomany_delegator() {
    // we need to delete 1 delegator out of 4, because only 3 delegator is allowed for every staker
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let delegate_amount = 1000 as u128;
    let delegator_keypair1 = Generator::random_keypair();
    let delegate_amount1 = 2000 as u128;
    let delegator_keypair2 = Generator::random_keypair();
    let delegate_amount2 = 3000 as u128;
    let delegator_keypair3 = Generator::random_keypair();
    let delegate_amount3 = 4000 as u128;
    let input_delegate_infos = BTreeSet::new();

    let delegators = vec![
        (delegator_keypair, delegate_amount),
        (delegator_keypair1, delegate_amount1),
        (delegator_keypair2, delegate_amount2),
        (delegator_keypair3, delegate_amount3),
    ];
    let tx =
        construct_delegate_smt_delegate_tx(&mut context, delegators, input_delegate_infos, false);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_smt_increase_success_toomany_delegator_withdraw() {
    // we need to delete 1 delegator out of 4, because only 3 delegator is allowed for every staker
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let delegate_amount = 500 as u128;
    let delegator_keypair1 = Generator::random_keypair();
    let delegate_amount1 = 2000 as u128;
    let delegator_keypair2 = Generator::random_keypair();
    let delegate_amount2 = 3000 as u128;
    let delegator_keypair3 = Generator::random_keypair();
    let delegate_amount3 = 4000 as u128;
    let input_delegate_infos = BTreeSet::from_iter(vec![LockInfo {
        addr: pubkey_to_addr(&delegator_keypair.1.serialize()),
        amount: 600,
    }]);

    let delegators = vec![
        (delegator_keypair, delegate_amount),
        (delegator_keypair1, delegate_amount1),
        (delegator_keypair2, delegate_amount2),
        (delegator_keypair3, delegate_amount3),
    ];
    let tx =
        construct_delegate_smt_delegate_tx(&mut context, delegators, input_delegate_infos, false);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_delegate_smt_increase_fail_toomany_delegator_not() {
    // we need to delete 1 delegator out of 4, because only 3 delegator is allowed for every staker
    // this time, we did not delete it
    // init context
    let mut context = Context::default();
    let delegator_keypair = Generator::random_keypair();
    let delegate_amount = 1000 as u128;
    let delegator_keypair1 = Generator::random_keypair();
    let delegate_amount1 = 2000 as u128;
    let delegator_keypair2 = Generator::random_keypair();
    let delegate_amount2 = 3000 as u128;
    let delegator_keypair3 = Generator::random_keypair();
    let delegate_amount3 = 4000 as u128;
    let input_delegate_infos = BTreeSet::new();

    let delegators = vec![
        (delegator_keypair, delegate_amount),
        (delegator_keypair1, delegate_amount1),
        (delegator_keypair2, delegate_amount2),
        (delegator_keypair3, delegate_amount3),
    ];
    let tx =
        construct_delegate_smt_delegate_tx(&mut context, delegators, input_delegate_infos, true);

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("DelegateSmtVerifySelectionError");
    assert_script_error(err, DelegateSmtVerifySelectionError as i8);
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

    let input_hash = calc_type_id(&input, 0);
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
        3,
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
        3,
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
        3,
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

#[test]
fn test_delegate_requirement_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("requirement");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");

    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let delegate_cell_data = axon_delegate_requirement_cell_data(10, 3);

    // prepare tx inputs and outputs
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .build(),
                delegate_cell_data.as_bytes(),
            ),
        )
        .build();

    let input_hash = get_input_hash(&input);
    let delegate_requirement_args = DelegateRequirementArgs::new_builder()
        .metadata_type_id(axon_array32_byte32([0u8; 32]))
        .requirement_type_id(axon_bytes_byte32(&input_hash))
        .build();

    let delegate_requirement_type_script = context
        .build_script(&contract_out_point, delegate_requirement_args.as_bytes())
        .expect("delegate requirement type script");

    let outputs = vec![
        // delegate at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script)
            .type_(Some(delegate_requirement_type_script.clone()).pack())
            .build(),
    ];

    let outputs_data = vec![delegate_cell_data.as_bytes()];

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
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
