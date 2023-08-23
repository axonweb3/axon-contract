use std::collections::BTreeSet;
use std::convert::{TryFrom, TryInto};

use crate::smt::{construct_epoch_smt, construct_lock_info_smt, TopSmtInfo};

use super::*;
use axon_types::metadata::{Metadata, MetadataList};
use axon_types::stake::*;
use axon_types::withdraw::WithdrawArgs;
// use bit_vec::BitVec;
use ckb_system_scripts::BUNDLED_CELL;
use ckb_testtool::ckb_crypto::secp::{Generator, Privkey, Pubkey};
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{
    bytes::Bytes, core::TransactionBuilder, core::TransactionView, packed::*, prelude::*,
};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use ophelia::{Crypto, PrivateKey, Signature, ToPublicKey, UncompressedPublicKey};
use ophelia_secp256k1::{Secp256k1Recoverable, Secp256k1RecoverablePrivateKey};
use util::error::Error::{
    BadInaugurationEpoch, BadStakeChange, InputOutputAtAmountNotEqual, UnstakeTooMuch,
};
use util::smt::{u64_to_h256, LockInfo, BOTTOM_SMT};
// use util::helper::pubkey_to_eth_addr;

fn construct_stake_at_tx(
    context: &mut Context,
    input_stake_info_delta: StakeInfoDelta,
    output_stake_info_delta: StakeInfoDelta,
    input_stake_at_amount: u128,
    input_normal_at_amount: u128,
    output_stake_at_amount: u128,
    output_normal_at_amount: u128,
) -> TransactionView {
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());

    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    let contract_bin: Bytes = Loader::default().load_binary("stake");
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
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");

    let stake_at_type_script = context
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

    // eth signature
    let hex_privkey = [0xcd; 32];
    let priv_key = Secp256k1RecoverablePrivateKey::try_from(hex_privkey.as_slice()).unwrap();
    let pubkey = priv_key.pub_key();
    let pubkey = pubkey.to_uncompressed_bytes().to_vec();
    // println!("pubkey: {:?}, len: {:?}", pubkey, pubkey.len());

    // prepare stake_args and stake_data
    println!(
        "metadata_type_script type hash: {:?}",
        metadata_type_script.calc_script_hash().as_slice()
    );
    let l2_addr = eth_addr(pubkey);
    let stake_args = stake::StakeArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .stake_addr(l2_addr.clone())
        .build();

    let keypair = Generator::random_keypair();
    // println!("staker pubkey: {:?}", keypair.1.serialize());
    let input_stake_at_data = axon_stake_at_cell_data_without_amount(
        0,
        &keypair.1.serialize(),
        l2_addr.clone(),
        &metadata_type_script.calc_script_hash(),
        input_stake_info_delta,
        DelegateRequirementInfo::default(),
    );

    // prepare stake lock_script
    let stake_at_lock_script = context
        .build_script(&contract_out_point, stake_args.as_bytes())
        .expect("stake script");

    // prepare tx inputs and outputs
    // println!("stake at cell lock hash:{:?}", stake_at_lock_script.calc_script_hash().as_slice());
    let inputs = vec![
        // stake AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(stake_at_lock_script.clone())
                        .type_(Some(stake_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_stake_at_cell_data(
                        input_stake_at_amount,
                        input_stake_at_data,
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
                        .type_(Some(stake_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(input_normal_at_amount.to_le_bytes().to_vec()),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // stake at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(stake_at_lock_script)
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
        // stake cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    let output_stake_at_data = axon_stake_at_cell_data_without_amount(
        0,
        &keypair.1.serialize(),
        l2_addr.clone(),
        &metadata_type_script.calc_script_hash(),
        output_stake_info_delta,
        DelegateRequirementInfo::default(),
    );
    let outputs_data = vec![
        Bytes::from(axon_stake_at_cell_data(
            output_stake_at_amount,
            output_stake_at_data,
        )), // stake at cell
        Bytes::from(output_normal_at_amount.to_le_bytes().to_vec()), // normal at cell
    ];

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &stake_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &stake_at_type_script, // needless here
        &checkpoint_type_script,
        metadata_list,
        1,
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
    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(&metadata_type_script.clone().calc_script_hash(), 1);
    // println!("checkpoint data: {:?}", checkpoint_data.as_bytes().len());
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

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        // .witness(stake_at_witness.as_bytes().pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(secp256k1_data_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        .build();

    let msg = tx.hash();
    // println!("tx hash: {:?}", msg.clone().as_bytes().to_vec());
    let signature = Secp256k1Recoverable::sign_message(&msg.as_bytes(), &priv_key.to_bytes())
        .unwrap()
        .to_bytes()
        .to_vec();
    println!(
        "eth_addr msg: {:?}, signature:{:?}, len: {:?}",
        tx.hash(),
        signature,
        signature.len()
    );

    let stake_at_witness = StakeAtWitness::new_builder()
        .mode(0.into())
        .eth_sig(axon_byte65(signature))
        .build();
    println!("stake at witness: {:?}", stake_at_witness.as_bytes().len());
    let stake_at_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(stake_at_witness.as_bytes())).pack())
        .build();

    let tx = context.complete_tx(tx);
    let tx = sign_eth_tx(tx, stake_at_witness);
    tx
}

#[test]
fn test_stake_at_success_increase_increase() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 900;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_stake_at_success_stale_increase_increase() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 900;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(1 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_stake_at_fail_wrong_epoch() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 900;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(1 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount))
        .inauguration_epoch(axon_u64(4 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("BadInaugurationEpoch");
    assert_script_error(err, BadInaugurationEpoch as i8);
}

#[test]
fn test_stake_at_fail_more_at() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 950; // should be 900 instead

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(1 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount))
        .inauguration_epoch(axon_u64(4 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("InputOutputAtAmountNotEqual");
    assert_script_error(err, InputOutputAtAmountNotEqual as i8);
}

#[test]
fn test_stake_at_success_decrease_increase() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 900;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount - input_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_stake_at_fail_decrease_increase() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 100;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 900;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();
    // output stake delta increase 200, but output stake at only increase 100(200-100)
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(output_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("BadStakeChange");
    assert_script_error(err, BadStakeChange as i8);
}

#[test]
fn test_stake_at_success_decrease_increase_less() {
    // decrease unstake amount
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 200;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 1000;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(output_stake_at_amount - 100))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_stake_at_success_decrease_decrease_toomuch() {
    // unstake amount more than total stake amount
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 200;
    let input_normal_at_amount = 1000;
    let output_stake_at_amount = 200;
    let output_normal_at_amount = 1000;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(200))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(0.into())
        .amount(axon_u128(output_stake_at_amount + 100))
        .inauguration_epoch(axon_u64(3 as u64))
        .build();

    let tx = construct_stake_at_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        input_normal_at_amount,
        output_stake_at_amount,
        output_normal_at_amount,
    );

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("UnstakeTooMuch");
    assert_script_error(err, UnstakeTooMuch as i8);
}

fn construct_stake_smt_tx(
    context: &mut Context,
    input_stake_info_delta: StakeInfoDelta,
    output_stake_info_delta: StakeInfoDelta,
    input_stake_at_amount: u128,
    output_stake_at_amount: u128,
) -> TransactionView {
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    let at_contract_bin: Bytes = Loader::default().load_binary("stake");
    let at_contract_out_point = context.deploy_cell(at_contract_bin);
    let at_contract_dep = CellDep::new_builder()
        .out_point(at_contract_out_point.clone())
        .build();
    let smt_contract_bin: Bytes = Loader::default().load_binary("stake-smt");
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
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");

    let stake_at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("sudt script");
    println!(
        "stake at type hash: {:?}",
        stake_at_type_script.calc_script_hash().as_bytes().to_vec()
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

    // prepare stake_args and stake_data
    let keypair = Generator::random_keypair();
    let l2_addr = eth_addr(keypair.1.serialize());
    let stake_at_args = stake::StakeArgs::new_builder()
        .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
        .stake_addr(l2_addr.clone())
        .build();

    let inauguration_epoch = 3;
    let input_stake_at_data = axon_stake_at_cell_data_without_amount(
        0,
        &keypair.1.serialize(),
        l2_addr.clone(),
        &metadata_type_script.calc_script_hash(),
        input_stake_info_delta,
        DelegateRequirementInfo::default(),
    );

    // prepare stake lock_script
    let stake_at_lock_script = context
        .build_script(&at_contract_out_point, stake_at_args.as_bytes())
        .expect("stake at lock script");

    let stake_smt_type_script = context
        .build_script_with_hash_type(
            &smt_contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![6u8; 32]),
        )
        .expect("stake smt type script");
    println!(
        "stake_smt_type_script: {:?}",
        stake_smt_type_script.calc_script_hash().as_bytes().to_vec()
    );

    // prepare tx inputs and outputs
    println!("empty input stake infos of stake smt cell");
    let input_stake_infos = BTreeSet::new();
    let input_stake_smt_data = axon_stake_smt_cell_data(
        &input_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );

    let input_stake_smt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
        input_stake_smt_data.as_bytes(),
    );

    let inputs = vec![
        // stake AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(stake_at_lock_script.clone())
                        .type_(Some(stake_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_stake_at_cell_data(
                        input_stake_at_amount,
                        input_stake_at_data,
                    )),
                ),
            )
            .build(),
        // stake smt cell
        CellInput::new_builder()
            .previous_output(input_stake_smt_out_point)
            .build(),
    ];

    let outputs = vec![
        // stake at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(stake_at_lock_script.clone())
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
        // stake smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    let output_stake_at_data = axon_stake_at_cell_data_without_amount(
        0,
        &keypair.1.serialize(),
        l2_addr.clone(),
        &metadata_type_script.calc_script_hash(),
        output_stake_info_delta,
        DelegateRequirementInfo::default(),
    );

    let lock_info = LockInfo {
        // addr: blake160(keypair.1.serialize().as_slice()),
        addr: l2_addr.as_slice().try_into().unwrap(),
        amount: input_stake_at_amount,
    };
    let output_stake_infos = vec![lock_info].into_iter().collect::<BTreeSet<LockInfo>>();
    // let output_stake_infos = BTreeSet::new();
    let output_stake_smt_data = axon_stake_smt_cell_data(
        &output_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );
    println!(
        "output stake smt data: {:?}",
        output_stake_smt_data.as_bytes().len()
    );
    let outputs_data = vec![
        Bytes::from(axon_stake_at_cell_data(
            output_stake_at_amount,
            output_stake_at_data,
        )), // stake at cell
        output_stake_smt_data.as_bytes(), // stake smt cell
    ];

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder()
        .epoch_len(axon_u32(100))
        .quorum(axon_u16(2))
        .build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &stake_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &stake_smt_type_script,
        &stake_smt_type_script,
        metadata_list,
        inauguration_epoch - 2,
        100,
        100,
        [0u8; 32],
        &stake_at_lock_script.code_hash(),
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

    // construct old epoch proof
    let bottom_tree = BOTTOM_SMT::default();
    let old_bottom_root = bottom_tree.root();
    let top_smt_infos = vec![TopSmtInfo {
        epoch: inauguration_epoch,
        smt_root: *old_bottom_root,
    }];
    let (_, old_proof) = construct_epoch_smt(&top_smt_infos);
    let old_proof = old_proof
        .compile(vec![u64_to_h256(inauguration_epoch)])
        .unwrap()
        .0;
    println!("old proof: {:?}", old_proof);

    let lock_infos: BTreeSet<LockInfo> =
        vec![lock_info].into_iter().collect::<BTreeSet<LockInfo>>();
    let (new_bottom_root, _) = construct_lock_info_smt(&lock_infos);
    let new_top_smt_infos = vec![TopSmtInfo {
        epoch: inauguration_epoch,
        smt_root: new_bottom_root,
    }];
    let (new_top_root, new_proof) = construct_epoch_smt(&new_top_smt_infos);
    let new_proof = new_proof
        .compile(vec![u64_to_h256(inauguration_epoch)])
        .unwrap()
        .0;
    println!(
        "new_bottom_root: {:?}, new_top_root: {:?}, epoch: {}, new proof: {:?}",
        new_bottom_root, new_top_root, inauguration_epoch, new_proof
    );

    let _stake_info = stake::StakeInfo::new_builder()
        .addr(axon_identity(&keypair.1.serialize().as_slice().to_vec()))
        .amount(axon_u128(100))
        .build(); // assume old stake smt is empty
    let stake_infos = stake::StakeInfos::new_builder().build();
    let stake_smt_update_info = stake::StakeSmtUpdateInfo::new_builder()
        .all_stake_infos(stake_infos)
        .old_epoch_proof(axon_bytes(&old_proof))
        .new_epoch_proof(axon_bytes(&new_proof))
        // .old_bottom_proof(axon_bytes_none())
        .build();
    let stake_smt_witness = StakeSmtWitness::new_builder()
        .mode(0.into())
        .update_info(stake_smt_update_info)
        .build();
    let stake_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(stake_smt_witness.as_bytes())).pack())
        .build();

    let stake_at_witness = StakeAtWitness::new_builder().mode(1.into()).build();
    println!("stake at witness: {:?}", stake_at_witness.as_bytes().len());
    let stake_at_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(stake_at_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(vec![
            stake_at_witness.as_bytes().pack(),
            stake_smt_witness.as_bytes().pack(),
        ])
        .cell_dep(at_contract_dep)
        .cell_dep(smt_contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(secp256k1_data_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        // .cell_dep(stake_smt_input_dep)
        .build();
    let tx = context.complete_tx(tx);
    tx
}

#[test]
fn test_stake_smt_success() {
    // init context
    let mut context = Context::default();
    let input_stake_at_amount = 200;
    let output_stake_at_amount = 200;

    let input_stake_info_delta = stake::StakeInfoDelta::new_builder()
        .is_increase(1.into())
        .amount(axon_u128(input_stake_at_amount))
        .inauguration_epoch(axon_u64(3))
        .build();
    let output_stake_info_delta = stake::StakeInfoDelta::new_builder().build();

    let tx = construct_stake_smt_tx(
        &mut context,
        input_stake_info_delta,
        output_stake_info_delta,
        input_stake_at_amount,
        output_stake_at_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

fn construct_unstake_smt_tx(
    context: &mut Context,
    stakers: Vec<TestStakeInfos>,
    input_unstake_amount: u128,
    input_stake_smt_amount: u128,
) -> TransactionView {
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    let at_contract_bin: Bytes = Loader::default().load_binary("stake");
    let at_contract_out_point = context.deploy_cell(at_contract_bin);
    let at_contract_dep = CellDep::new_builder()
        .out_point(at_contract_out_point.clone())
        .build();
    let smt_contract_bin: Bytes = Loader::default().load_binary("stake-smt");
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
            Bytes::from(vec![2]),
        )
        .expect("checkpoint script");

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

    let inauguration_epoch = 2; // default epoch of checkpoint cell is 0
    let stake_smt_type_script = context
        .build_script_with_hash_type(
            &smt_contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![6u8; 32]),
        )
        .expect("stake smt type script");
    let stake_at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("sudt script");
    println!(
        "stake_smt_type_script: {:?}, stake at type hash: {:?}",
        stake_smt_type_script.calc_script_hash().as_bytes().to_vec(),
        stake_at_type_script.calc_script_hash().as_bytes().to_vec()
    );
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut outputs_data = Vec::new();
    let mut witnesses = Vec::new();
    let mut is_first = true;
    let mut special_keypair = Generator::random_keypair();
    let mut new_lock_infos = Vec::new();
    let is_too_many_staker = stakers.len() > 1; // means there are election
    for staker_info in stakers {
        // prepare stake_args and stake_data
        let keypair = staker_info.staker;
        let l2_addr = eth_addr(keypair.1.serialize());
        // always make stakers[0] to be the one to be deleted or unstaked
        if is_first {
            special_keypair = keypair.clone();
            is_first = false;
            if !is_too_many_staker {
                // only one staker
                // if input_stake_smt_amount <= input_unstake_amount, amount set to 0, illegal!
                let lock_info = LockInfo {
                    addr: l2_addr.as_slice().try_into().unwrap(),
                    amount: input_stake_smt_amount.saturating_sub(input_unstake_amount),
                };
                println!("only one staker: {:?}", lock_info);
                new_lock_infos.push(lock_info);
            }
        } else {
            // if is_too_many_staker {
            let lock_info = LockInfo {
                addr: l2_addr.as_slice().try_into().unwrap(),
                amount: staker_info.output_stake_at_amount,
            };
            println!("many staker: {:?}", lock_info);
            new_lock_infos.push(lock_info);
            // }
        }

        let stake_at_args = stake::StakeArgs::new_builder()
            .metadata_type_id(axon_byte32(&metadata_type_script.calc_script_hash()))
            .stake_addr(l2_addr.clone())
            .build();
        // prepare stake lock_script
        let stake_at_lock_script = context
            .build_script(&at_contract_out_point, stake_at_args.as_bytes())
            .expect("stake at lock script");

        let input_stake_at_data = axon_stake_at_cell_data_without_amount(
            0,
            &keypair.1.serialize(),
            l2_addr.clone(),
            &metadata_type_script.calc_script_hash(),
            staker_info.input_stake_info_delta,
            DelegateRequirementInfo::default(),
        );

        // stake AT cell
        let input_stake_at_cell = CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(stake_at_lock_script.clone())
                        .type_(Some(stake_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_stake_at_cell_data(
                        staker_info.input_stake_at_amount,
                        input_stake_at_data,
                    )),
                ),
            )
            .build();
        inputs.push(input_stake_at_cell);

        let output_stake_at_cell =         // stake at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(stake_at_lock_script.clone())
            .type_(Some(stake_at_type_script.clone()).pack())
            .build();
        outputs.push(output_stake_at_cell);

        let output_stake_at_data = axon_stake_at_cell_data_without_amount(
            0,
            &keypair.1.serialize(),
            l2_addr.clone(),
            &metadata_type_script.calc_script_hash(),
            staker_info.output_stake_info_delta,
            DelegateRequirementInfo::default(),
        );
        let output_stake_at_data = Bytes::from(axon_stake_at_cell_data(
            staker_info.output_stake_at_amount,
            output_stake_at_data,
        )); // stake at cell
        outputs_data.push(output_stake_at_data);

        let stake_at_witness = StakeAtWitness::new_builder().mode(1.into()).build();
        println!("stake at witness: {:?}", stake_at_witness.as_bytes().len());
        let stake_at_witness = WitnessArgs::new_builder()
            .lock(Some(Bytes::from(stake_at_witness.as_bytes())).pack())
            .build();
        witnesses.push(stake_at_witness.as_bytes().pack());
    }

    println!("input stake infos of stake smt cell");
    let l2_addr = eth_addr(special_keypair.1.serialize());
    let old_lock_info = LockInfo {
        addr: l2_addr.as_slice().try_into().unwrap(),
        amount: input_stake_smt_amount,
    };
    let input_stake_infos = vec![old_lock_info]
        .into_iter()
        .collect::<BTreeSet<LockInfo>>();
    let input_stake_smt_data = axon_stake_smt_cell_data(
        &input_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );

    let input_stake_smt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
        input_stake_smt_data.as_bytes(),
    );

    let withdraw_lock_args = WithdrawArgs::new_builder()
        .addr(l2_addr.clone())
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
        l2_addr.clone(),
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
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
        Bytes::from(axon_withdraw_at_cell_data(0, input_withdraw_data)), // delegate at cell
    );

    inputs.push(
        // stake smt cell
        CellInput::new_builder()
            .previous_output(input_stake_smt_out_point)
            .build(),
    );
    inputs.push(
        // withdraw at cell
        CellInput::new_builder()
            .previous_output(input_withdraw_out_point)
            .build(),
    );

    outputs.push(
        // stake smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
    );
    outputs.push(
        // withdraw at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdraw_lock_script.clone())
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
    );

    // let new_lock_info = LockInfo {
    //     addr: l2_addr.as_slice().try_into().unwrap(),
    //     amount: input_stake_smt_amount.saturating_sub(input_unstake_amount),
    // };
    let output_stake_infos = new_lock_infos.into_iter().collect::<BTreeSet<LockInfo>>();
    let output_stake_smt_data = axon_stake_smt_cell_data(
        &output_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );
    println!(
        "output stake smt data: {:?}",
        output_stake_smt_data.as_bytes().len()
    );

    let mut withdraw_amount = input_unstake_amount;
    if is_too_many_staker {
        withdraw_amount = input_stake_smt_amount; // input at amount of delete staker, delete smt amount may be better?
    }
    let output_withdraw_infos = vec![
        (inauguration_epoch - 2 as u64, 0 as u128),
        (inauguration_epoch - 1, 0),
        (inauguration_epoch, withdraw_amount),
    ];
    let output_withdraw_data = axon_withdraw_at_cell_data_without_amount(output_withdraw_infos);

    outputs_data.push(output_stake_smt_data.as_bytes()); // stake smt cell;
    outputs_data.push(Bytes::from(axon_withdraw_at_cell_data(
        withdraw_amount,
        output_withdraw_data,
    ))); // withdraw at cell

    // prepare metadata cell_dep
    // just for stake at code hash
    let stake_at_lock_script = context
        .build_script(&at_contract_out_point, Bytes::from(vec![9u8]))
        .expect("stake at lock script");
    println!(
        "stake_at_lock_script.code_hash(): {:?}",
        stake_at_lock_script.code_hash().as_slice()
    );
    let metadata = Metadata::new_builder()
        .epoch_len(axon_u32(100))
        .quorum(axon_u16(1)) // so that before election, only 3 stakers are allowed
        .build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data_by_script(
        &metadata_type_script.clone(),
        &stake_at_type_script.calc_script_hash(),
        &checkpoint_type_script,
        &stake_smt_type_script,
        &stake_smt_type_script,
        metadata_list,
        inauguration_epoch - 2,
        100,
        100,
        [0u8; 32],
        &stake_at_lock_script.code_hash(),
        &metadata_type_script.code_hash(),
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
    let checkpoint_data = axon_checkpoint_data(
        &metadata_type_script.clone().calc_script_hash(),
        inauguration_epoch - 2,
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

    // construct old epoch proof
    let old_lock_infos: BTreeSet<LockInfo> = vec![old_lock_info]
        .into_iter()
        .collect::<BTreeSet<LockInfo>>();
    let (old_bottom_root, _) = construct_lock_info_smt(&old_lock_infos);
    let top_smt_infos = vec![TopSmtInfo {
        epoch: inauguration_epoch,
        smt_root: old_bottom_root,
    }];
    let (_, old_proof) = construct_epoch_smt(&top_smt_infos);
    let old_proof = old_proof
        .compile(vec![u64_to_h256(inauguration_epoch)])
        .unwrap()
        .0;
    println!("old proof: {:?}", old_proof);

    let (new_bottom_root, _) = construct_lock_info_smt(&output_stake_infos);
    let new_top_smt_infos = vec![TopSmtInfo {
        epoch: inauguration_epoch,
        smt_root: new_bottom_root,
    }];
    let (new_top_root, new_proof) = construct_epoch_smt(&new_top_smt_infos);
    let new_proof = new_proof
        .compile(vec![u64_to_h256(inauguration_epoch)])
        .unwrap()
        .0;
    println!(
        "new_bottom_root: {:?}, new_top_root: {:?}, epoch: {}, new proof: {:?}",
        new_bottom_root, new_top_root, inauguration_epoch, new_proof
    );

    let stake_info = stake::StakeInfo::new_builder()
        .addr(l2_addr.clone())
        .amount(axon_u128(input_stake_smt_amount))
        .build(); // assume old stake smt is empty
    let stake_infos = stake::StakeInfos::new_builder().push(stake_info).build();
    let stake_smt_update_info = stake::StakeSmtUpdateInfo::new_builder()
        .all_stake_infos(stake_infos)
        .old_epoch_proof(axon_bytes(&old_proof))
        .new_epoch_proof(axon_bytes(&new_proof))
        .build();
    let stake_smt_witness = StakeSmtWitness::new_builder()
        .mode(0.into())
        .update_info(stake_smt_update_info)
        .build();
    let stake_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(stake_smt_witness.as_bytes())).pack())
        .build();
    witnesses.push(stake_smt_witness.as_bytes().pack());

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(witnesses)
        .cell_dep(at_contract_dep)
        .cell_dep(smt_contract_dep)
        // .cell_dep(withdraw_contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(secp256k1_data_dep)
        .cell_dep(checkpoint_script_dep)
        .cell_dep(metadata_script_dep)
        .build();
    let tx = context.complete_tx(tx);
    tx
}

struct TestStakeInfos {
    staker: (Privkey, Pubkey),
    input_stake_info_delta: StakeInfoDelta,
    input_stake_at_amount: u128,
    output_stake_info_delta: StakeInfoDelta,
    output_stake_at_amount: u128,
}

#[test]
fn test_stake_smt_success_toomany_stakers() {
    // init context
    let mut context = Context::default();
    let input_stake_smt_amount = 100;
    let input_stake_at_amount = 200;

    let mut stakers = Vec::new();
    let staker0 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(1.into())
            .amount(axon_u128(input_stake_at_amount - input_stake_smt_amount))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: input_stake_at_amount,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: input_stake_at_amount - input_stake_smt_amount,
    };

    let staker1 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(1.into())
            .amount(axon_u128(1000))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: 1000,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: 1000,
    };

    let staker2 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(1.into())
            .amount(axon_u128(2000))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: 2000,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: 2000,
    };

    let staker3 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(1.into())
            .amount(axon_u128(3000))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: 3000,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: 3000,
    };

    stakers.push(staker0);
    stakers.push(staker1);
    stakers.push(staker2);
    stakers.push(staker3);

    let tx = construct_unstake_smt_tx(&mut context, stakers, 0, input_stake_smt_amount);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

// staker has staked 100 at, and updated to stake smt cell
// staker then want to redeem 5 at , and updated to stake smt cell
#[test]
fn test_unstake_smt_success() {
    // init context
    let mut context = Context::default();
    let input_unstake_amount = 10;
    let input_stake_smt_amount = 100;
    let input_stake_at_amount = 100;

    let mut stakers = Vec::new();
    let staker0 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(0.into())
            .amount(axon_u128(input_unstake_amount))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: input_stake_at_amount,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: input_stake_at_amount - input_unstake_amount,
    };
    stakers.push(staker0);

    let tx = construct_unstake_smt_tx(
        &mut context,
        stakers,
        input_unstake_amount,
        input_stake_smt_amount,
    );
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

// staker has staked 90 at, and updated to stake smt cell
// staker then want to redeem 100 at
#[test]
fn test_unstake_smt_fail_toomuch() {
    // init context
    let mut context = Context::default();
    let input_unstake_amount = 100;
    let input_stake_smt_amount = input_unstake_amount - 10;
    let input_stake_at_amount = input_stake_smt_amount;

    let mut stakers = Vec::new();
    let staker0 = TestStakeInfos {
        staker: Generator::random_keypair(),
        input_stake_info_delta: stake::StakeInfoDelta::new_builder()
            .is_increase(0.into())
            .amount(axon_u128(input_unstake_amount))
            .inauguration_epoch(axon_u64(2))
            .build(),
        input_stake_at_amount: input_stake_at_amount,
        output_stake_info_delta: stake::StakeInfoDelta::new_builder().build(),
        output_stake_at_amount: 0,
    };
    stakers.push(staker0);

    let tx = construct_unstake_smt_tx(
        &mut context,
        stakers,
        input_unstake_amount,
        input_stake_smt_amount,
    );
    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("UnstakeTooMuch");
    assert_script_error(err, UnstakeTooMuch as i8);
}

#[test]
fn test_stake_smt_create_success() {
    // init context
    let mut context = Context::default();

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let smt_contract_bin: Bytes = Loader::default().load_binary("stake-smt");
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
    let stake_smt_type_script = context
        .build_script(&smt_contract_out_point, input_hash)
        .expect("stake smt type script");
    println!(
        "stake_smt_type_script: {:?}",
        stake_smt_type_script.calc_script_hash().as_bytes().to_vec()
    );

    let outputs = vec![
        // stake smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
    ];

    let inauguration_epoch = 3;
    let output_stake_infos = BTreeSet::new();
    let output_stake_smt_data = axon_stake_smt_cell_data(
        &output_stake_infos,
        &always_success_lock_script.calc_script_hash(),
        inauguration_epoch,
    );
    println!(
        "output stake smt data: {:?}",
        output_stake_smt_data.as_bytes().len()
    );
    let outputs_data = vec![
        output_stake_smt_data.as_bytes(), // stake smt cell
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
fn test_stake_election_success() {
    // init context
    let mut context = Context::default();

    let contract_bin: Bytes = Loader::default().load_binary("stake-smt");
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
    let stake_smt_type_script = context
        .build_script_with_hash_type(
            &contract_out_point,
            ScriptHashType::Type,
            Bytes::from(vec![6u8; 32]),
        )
        .expect("stake smt type script");

    // prepare tx inputs and outputs
    let inauguration_epoch = 3;
    let input_stake_infos = BTreeSet::new();
    let input_stake_smt_data = axon_stake_smt_cell_data(
        &input_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );

    // prepare metadata cell_dep
    let metadata = Metadata::new_builder().epoch_len(axon_u32(100)).build();
    let metadata_list = MetadataList::new_builder().push(metadata).build();
    let meta_data = axon_metadata_data(
        &metadata_type_script.clone().calc_script_hash(),
        &stake_smt_type_script.calc_script_hash(),
        &checkpoint_type_script.calc_script_hash(),
        &stake_smt_type_script.calc_script_hash(),
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
                        .type_(Some(stake_smt_type_script.clone()).pack())
                        .build(),
                    input_stake_smt_data.as_bytes(),
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
        // stake smt cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(stake_smt_type_script.clone()).pack())
            .build(),
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(always_success_lock_script.clone())
            .type_(Some(metadata_type_script.clone()).pack())
            .build(),
    ];

    let output_stake_infos = BTreeSet::new();
    let output_stake_smt_data = axon_stake_smt_cell_data(
        &output_stake_infos,
        &metadata_type_script.calc_script_hash(),
        inauguration_epoch,
    );
    let outputs_data = vec![
        output_stake_smt_data.as_bytes(), // stake smt cell
        meta_data.as_bytes(),
    ];

    let stake_smt_witness = StakeSmtWitness::new_builder().mode(1.into()).build();
    let stake_smt_witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(stake_smt_witness.as_bytes())).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witnesses(vec![stake_smt_witness.as_bytes().pack()])
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
fn test_eth_sig_success() {
    let hex_privkey = [0xcd; 32];
    let priv_key = Secp256k1RecoverablePrivateKey::try_from(hex_privkey.as_slice()).unwrap();
    let pubkey = priv_key.pub_key();
    let pubkey = pubkey.to_uncompressed_bytes().to_vec();

    let msg = [1u8; 32];
    // println!("tx hash: {:?}", msg.clone().as_bytes().to_vec());
    let signature = Secp256k1Recoverable::sign_message(&msg, &priv_key.to_bytes())
        .unwrap()
        .to_bytes()
        .to_vec();
    println!(
        "eth_addr msg: {:?}, signature:{:?}, len: {:?}",
        msg,
        signature,
        signature.len()
    );

    let result = Secp256k1Recoverable::verify_signature(&msg, &signature, &pubkey);
    println!(
        "secp256k1 signature, signature: {:?}, msg: {:?}, pubkey: {:?}",
        signature,
        msg.as_slice().to_vec(),
        pubkey.to_vec()
    );
    assert!(result.is_ok());
    match result {
        Ok(_) => println!("Verify secp256k1 signature success!"),
        Err(err) => println!("Verify secp256k1 signature failed! {}", err),
    }
}

#[test]
fn test_lock_info_sort_success() {
    let lock_info0 = LockInfo {
        addr: [0u8; 20],
        amount: 200,
    };
    let lock_info1 = LockInfo {
        addr: [1u8; 20],
        amount: 100,
    };
    let lock_info2 = LockInfo {
        addr: [2u8; 20],
        amount: 300,
    };

    let mut lock_infos = BTreeSet::new();
    lock_infos.insert(lock_info0);
    lock_infos.insert(lock_info1);
    lock_infos.insert(lock_info2);

    let iter = lock_infos.iter();
    let mut top_3quorum = iter.take(2);
    let mut new_stake_infos_set = BTreeSet::new();
    while let Some(elem) = top_3quorum.next() {
        new_stake_infos_set.insert((*elem).clone());
    }

    for lock_info in &new_stake_infos_set {
        println!("LockInfo: {:?}", lock_info);
    }
}
