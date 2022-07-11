use super::*;
use bit_vec::BitVec;
use ckb_system_scripts::BUNDLED_CELL;
use ckb_testtool::ckb_crypto::secp::Generator;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType, TransactionBuilder},
    packed::*,
    prelude::*,
};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;
use rlp::RlpStream;

const MAX_CYCLES: u64 = 100_000_000;

#[test]
fn test_selection_success() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("selection");
    let out_point = context.deploy_cell(contract_bin);
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare lock_args
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::new())
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();
    let omni_lock_hash = always_success_lock_script.calc_script_hash();
    let selection_args = axon::SelectionLockArgs::new_builder()
        .omni_lock_hash(axon_byte32(&omni_lock_hash))
        .checkpoint_lock_hash(axon_byte32(&Byte32::default()))
        .build();

    // prepare scripts
    let lock_script = context
        .build_script(&out_point, selection_args.as_bytes())
        .expect("selection script");
    let lock_script_dep = CellDep::new_builder().out_point(out_point).build();

    // prepare inputs and outputs
    let inputs = vec![
        // omni cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(500.pack())
                        .lock(always_success_lock_script.clone())
                        .build(),
                    Bytes::new(),
                ),
            )
            .build(),
        // selection cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(500.pack())
                        .lock(lock_script.clone())
                        .build(),
                    Bytes::new(),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // omni cell
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(always_success_lock_script)
            .build(),
        // selection cell
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    // prepare outputs_data
    let outputs_data = vec![Bytes::new(), Bytes::new()];

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
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
fn test_checkpoint_success() {
    // init context
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("checkpoint");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let withdrawal_bin: Bytes = Loader::default().load_binary("withdrawal");
    let withdrawal_out_point = context.deploy_cell(withdrawal_bin);
    let withdrawal_dep = CellDep::new_builder()
        .out_point(withdrawal_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let checkpoint_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .expect("checkpoint script");
    let stake_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![3]))
        .expect("checkpoint script");
    let sudt_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("at script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    // prepare withdrawal script
    let withdrawal_args = axon::WithdrawalLockArgs::new_builder()
        .admin_identity(axon_identity(&vec![0u8; 20]))
        .checkpoint_cell_type_hash(axon_byte32(&checkpoint_type_script.calc_script_hash()))
        .node_identity(axon_identity_opt(&vec![0u8; 20]))
        .build();
    let withdrawal_lock_script = context
        .build_script(&withdrawal_out_point, withdrawal_args.as_bytes())
        .expect("withdrawal script");

    // prepare checkpoint script
    let checkpoint_args = axon::CheckpointLockArgs::new_builder()
        .admin_identity(axon_identity(&vec![0u8; 20]))
        .type_id_hash(axon_byte32(&checkpoint_type_script.calc_script_hash()))
        .build();
    let checkpoint_data = axon_checkpoint_data(
        &sudt_type_script.calc_script_hash(),
        &stake_type_script.calc_script_hash(),
        &withdrawal_lock_script.code_hash(),
    );
    let checkpoint_lock_script = context
        .build_script(&contract_out_point, checkpoint_args.as_bytes())
        .expect("checkpoint script");

    // prepare stake script celldep
    let bls_keypairs = vec![0; 8]
        .iter()
        .map(|_| random_bls_keypair())
        .collect::<Vec<_>>();
    let stake_infos = vec![1u64; 8]
        .into_iter()
        .enumerate()
        .map(|(i, era)| {
            let mut bls_pubkey = [0u8; 48];
            bls_pubkey.copy_from_slice(&bls_keypairs[i].1);
            axon_stake_info(&vec![i as u8; 20], &bls_pubkey, (i + 1) as u128, era)
        })
        .collect::<Vec<_>>();
    let stake_data = axon_stake_data(
        stake_infos.len() as u8,
        &checkpoint_type_script.calc_script_hash(),
        &sudt_type_script.calc_script_hash(),
        &stake_infos,
    );
    let stake_cell_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(stake_type_script).pack())
                    .build(),
                stake_data.as_bytes(),
            ),
        )
        .build();

    // prepare tx inputs and outputs
    let inputs = vec![
        // checkpoint cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(checkpoint_lock_script.clone())
                        .type_(Some(checkpoint_type_script.clone()).pack())
                        .build(),
                    checkpoint_data.as_bytes(),
                ),
            )
            .build(),
        // withdrawal cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .lock(withdrawal_lock_script.clone())
                        .type_(Some(sudt_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_withdrawal_data(3000, 2)),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // checkpoint cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(checkpoint_lock_script)
            .type_(Some(checkpoint_type_script).pack())
            .build(),
        // withdrawal cell
        CellOutput::new_builder()
            .lock(withdrawal_lock_script)
            .type_(Some(sudt_type_script).pack())
            .build(),
    ];

    // prepare proposal rlp
    let proposal = {
        let mut proposal = RlpStream::new_list(14);
        proposal.append_empty_data();
        proposal.append(&vec![0u8; 20]); // proposer_address
        vec![0; 9].iter().for_each(|_| {
            proposal.append_empty_data();
        });
        proposal.append(&vec![0u8; 32]); // last_checkpoint_block_hash
        proposal.append_empty_data();
        proposal.append_empty_data();
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

    // prepare outputs_data
    let output_checkpoint_data = checkpoint_data
        .as_builder()
        .era(axon_byte8(2))
        .period(axon_byte8(2))
        .block_hash(axon_byte32(&proposal_hash.to_fixed_bytes().pack()))
        .build();
    let outputs_data = vec![
        output_checkpoint_data.as_bytes(),
        Bytes::from(axon_withdrawal_data(3000, 3)),
    ];

    // prepare witness
    let witness_lock = axon::CheckpointLockWitnessLock::new_builder()
        .proposal(axon_bytes(&proposal))
        .proof(axon_bytes(&proof))
        .build();
    let witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(witness_lock.as_bytes())).pack())
        .input_type(Some(Bytes::from(vec![1])).pack())
        .build();

    // prepare signed tx
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(witness.as_bytes().pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(stake_cell_dep)
        .cell_dep(withdrawal_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_withdrawal_success() {
    // init context
    let mut context = Context::default();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    let contract_bin: Bytes = Loader::default().load_binary("withdrawal");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let type_id_type_script = context
        .build_script(&always_success_out_point, Bytes::new())
        .expect("type_id script");
    let at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("at script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    // prepare checkpoint_args and checkpoint_data
    let keypair = Generator::random_keypair();
    let withdrawal_args = axon::WithdrawalLockArgs::new_builder()
        .admin_identity(axon_identity(&keypair.1.serialize()))
        .checkpoint_cell_type_hash(axon_byte32(&type_id_type_script.calc_script_hash()))
        .node_identity(axon_identity_opt(&keypair.1.serialize()))
        .build();
    let withdrawal_data = axon_withdrawal_data(0, 1);

    // prepare checkpoint lock_script
    let withdrawal_lock_script = context
        .build_script(&contract_out_point, withdrawal_args.as_bytes())
        .expect("withdrawal script");

    // prepare tx inputs and outputs
    let inputs = vec![
        // withdrawal cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(withdrawal_lock_script.clone())
                        .type_(Some(at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(withdrawal_data.clone()),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // withdrawal cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdrawal_lock_script)
            .type_(Some(at_type_script).pack())
            .build(),
    ];

    // prepare outputs_data
    let outputs_data = vec![Bytes::from(withdrawal_data)];

    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(
        &type_id_type_script.calc_script_hash(),
        &[0u8; 32].pack(),
        &[0u8; 32].pack(),
    );
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script)
                    .type_(Some(type_id_type_script).pack())
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
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(secp256k1_data_dep)
        .cell_dep(checkpoint_script_dep)
        .build();
    let tx = context.complete_tx(tx);
    let tx = sign_tx(tx, &keypair.0, 1);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_stake_success() {
    // init context
    let mut context = Context::default();
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
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .expect("checkpoint script");
    let stake_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![3]))
        .expect("stake script");
    let stake_at_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![4]))
        .expect("sudt script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    // prepare stake_args and stake_data
    let stake_args = axon::StakeLockArgs::new_builder()
        .admin_identity(axon_identity(&vec![0u8; 20]))
        .type_id_hash(axon_byte32(&stake_type_script.calc_script_hash()))
        .node_identity(axon_identity_none())
        .build();
    let keypair = Generator::random_keypair();
    let at_stake_args = axon::StakeLockArgs::new_builder()
        .admin_identity(axon_identity(&vec![0u8; 20]))
        .type_id_hash(axon_byte32(&stake_type_script.calc_script_hash()))
        .node_identity(axon_identity_opt(&keypair.1.serialize()))
        .build();
    let mut stake_infos = vec![3u64; 10]
        .into_iter()
        .enumerate()
        .map(|(i, era)| axon_stake_info(&vec![i as u8; 20], &[i as u8; 48], (i + 1) as u128, era))
        .collect::<Vec<_>>();
    let input_stake_data = axon_stake_data(
        20,
        &checkpoint_type_script.calc_script_hash(),
        &stake_at_type_script.calc_script_hash(),
        &stake_infos,
    );

    // prepare stake lock_script
    let stake_lock_script = context
        .build_script(&contract_out_point, stake_args.as_bytes())
        .expect("stake script");
    let at_stake_lock_script = context
        .build_script(&contract_out_point, at_stake_args.as_bytes())
        .expect("at stake script");

    // prepare withdraw lock_script
    let withdrawal_args = axon::WithdrawalLockArgs::new_builder()
        .admin_identity(axon_identity(&vec![0u8; 20]))
        .checkpoint_cell_type_hash(axon_byte32(&checkpoint_type_script.calc_script_hash()))
        .node_identity(axon_identity_opt(&keypair.1.serialize()))
        .build();
    let withdrawal_lock_script = Script::new_builder()
        .code_hash([0u8; 32].pack())
        .hash_type(ScriptHashType::Type.into())
        .args(withdrawal_args.as_slice().pack())
        .build();

    // prepare tx inputs and outputs
    let inputs = vec![
        // stake AT cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(at_stake_lock_script.clone())
                        .type_(Some(stake_at_type_script.clone()).pack())
                        .build(),
                    Bytes::from(axon_at_data(152, 1).to_vec()),
                ),
            )
            .build(),
        // stake cell
        CellInput::new_builder()
            .previous_output(
                context.create_cell(
                    CellOutput::new_builder()
                        .capacity(1000.pack())
                        .lock(stake_lock_script.clone())
                        .type_(Some(stake_type_script.clone()).pack())
                        .build(),
                    input_stake_data.as_bytes(),
                ),
            )
            .build(),
    ];
    let outputs = vec![
        // stake at cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(at_stake_lock_script)
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
        // stake cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(stake_lock_script)
            .type_(Some(stake_type_script.clone()).pack())
            .build(),
        // withdrawal cell
        CellOutput::new_builder()
            .capacity(1000.pack())
            .lock(withdrawal_lock_script.clone())
            .type_(Some(stake_at_type_script.clone()).pack())
            .build(),
    ];

    // prepare outputs_data
    stake_infos.push(axon_stake_info(&keypair.1.serialize(), &[10; 48], 102, 3));
    let output_stake_data = axon_stake_data(
        20,
        &checkpoint_type_script.calc_script_hash(),
        &stake_at_type_script.calc_script_hash(),
        &stake_infos,
    );
    let outputs_data = vec![
        Bytes::from(axon_at_data(102, 1).to_vec()),
        output_stake_data.as_bytes(),
        Bytes::from(axon_withdrawal_data(50, 2)),
    ];

    // prepare checkpoint cell_dep
    let checkpoint_data = axon_checkpoint_data(
        &stake_at_type_script.calc_script_hash(),
        &[0u8; 32].pack(),
        &withdrawal_lock_script.code_hash(),
    );
    let checkpoint_script_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script)
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
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(secp256k1_data_dep)
        .cell_dep(checkpoint_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // sign tx for stake at cell (companion mode)
    let tx = sign_tx(tx, &keypair.0, 1);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_metadata_update_success() {
    let mut context = Context::default();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    let contract_bin: Bytes = Loader::default().load_binary("crosschain-metadata");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    // prepare metadata lock script
    let keypair = Generator::random_keypair();
    let metadata_script = context
        .build_script(
            &contract_out_point,
            blake160(&keypair.1.serialize()).to_vec().into(),
        )
        .expect("crosschain-metadata script");

    // prepare metadata cell data
    let metadata = crosschain::Metadata::new_builder()
        .chain_id(cs_uint16(5))
        .ckb_fee_ratio(cs_uint32(100))
        .stake_typehash(cs_hash(&Byte32::default()))
        .token_config(cs_token_config(&vec![]))
        .build();
    println!("metadata = {}", hex::encode(metadata.as_slice()));
    println!("metadata = {}", String::from("3d0000001400000015000000190000001d0000000504000000640000000000000000000000000000000000000000000000000000000000000000000000"));

    // prepare ckb transaction input/output
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(metadata_script.clone())
                    .build(),
                metadata.as_bytes(),
            ),
        )
        .build();
    let output = CellOutput::new_builder()
        .capacity(1000.pack())
        .lock(metadata_script)
        .build();

    // tx build and sign
    let tx = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(metadata.as_bytes().pack())
        .cell_dep(contract_dep)
        .cell_dep(secp256k1_data_dep)
        .build();
    let tx = context.complete_tx(tx);
    let tx = sign_tx(tx, &keypair.0, 1);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_crosschain_request_success() {
    let mut context = Context::default();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();
    let sudt_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .expect("sudt script");
    let metadata_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![3]))
        .expect("metadata script");
    let lock_lock_script = context
        .build_script(
            &always_success_out_point,
            metadata_type_script.calc_script_hash().as_bytes(),
        )
        .expect("lock script");
    let contract_bin: Bytes = Loader::default().load_binary("crosschain-request");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    // prepare metadata celldep
    let sudt_config = ([0u8; 20], sudt_type_script.calc_script_hash(), 100);
    let metadata = crosschain::Metadata::new_builder()
        .chain_id(cs_uint16(5))
        .ckb_fee_ratio(cs_uint32(100))
        .stake_typehash(cs_hash(&Byte32::default()))
        .token_config(cs_token_config(&vec![sudt_config]))
        .build();
    let metadata_celldep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(metadata_type_script.clone()).pack())
                    .build(),
                metadata.as_bytes(),
            ),
        )
        .build();

    // prepare corsschain request script
    let transfer_args = crosschain::Transfer::new_builder()
        .axon_address(cs_address(&[0u8; 20]))
        .ckb_amount(cs_uint64(450))
        .sudt_amount(cs_uint128(270))
        .erc20_address(cs_address(&[0u8; 20]))
        .build();
    let request_script = context
        .build_script(&contract_out_point, transfer_args.as_bytes())
        .expect("crosschain-request script");

    // prepare ckb transaction input/output
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(600.pack())
                    .lock(always_success_lock_script.clone())
                    .build(),
                Bytes::new(),
            ),
        )
        .build();
    let outputs = vec![
        // crosschain-lock cell
        CellOutput::new_builder()
            .capacity(500.pack())
            .lock(lock_lock_script)
            .type_(Some(sudt_type_script).pack())
            .build(),
        // corsschain-request cell
        CellOutput::new_builder()
            .capacity(100.pack())
            .lock(always_success_lock_script)
            .type_(Some(request_script).pack())
            .build(),
    ];

    // build tx data
    let outputs_data = vec![
        Bytes::from(300u128.to_le_bytes().to_vec()).pack(),
        Bytes::new().pack(),
    ];

    // build tx
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data)
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(metadata_celldep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_crosschain_lock_success() {
    let mut context = Context::default();
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_lock_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![1]))
        .expect("always_success script");
    let always_success_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();
    let metadata_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![2]))
        .expect("metadata script");
    let stake_type_script = context
        .build_script(&always_success_out_point, Bytes::from(vec![3]))
        .expect("stake script");
    let contract_bin: Bytes = Loader::default().load_binary("crosschain-lock");
    let contract_out_point = context.deploy_cell(contract_bin);
    let contract_dep = CellDep::new_builder()
        .out_point(contract_out_point.clone())
        .build();

    // prepare metadata celldep
    let metadata = crosschain::Metadata::new_builder()
        .chain_id(cs_uint16(5))
        .ckb_fee_ratio(cs_uint32(100))
        .stake_typehash(cs_hash(&stake_type_script.calc_script_hash()))
        .token_config(cs_token_config(&vec![]))
        .build();
    let metadata_celldep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(metadata_type_script.clone()).pack())
                    .build(),
                metadata.as_bytes(),
            ),
        )
        .build();

    // prepare stake script celldep
    let bls_keypairs = vec![0; 8]
        .iter()
        .map(|_| random_bls_keypair())
        .collect::<Vec<_>>();
    let stake_infos = vec![1u64; 8]
        .into_iter()
        .enumerate()
        .map(|(i, era)| {
            let mut bls_pubkey = [0u8; 48];
            bls_pubkey.copy_from_slice(&bls_keypairs[i].1);
            axon_stake_info(&vec![i as u8; 20], &bls_pubkey, (i + 1) as u128, era)
        })
        .collect::<Vec<_>>();
    let stake_data = axon_stake_data(
        stake_infos.len() as u8,
        &Byte32::default(),
        &Byte32::default(),
        &stake_infos,
    );
    let stake_cell_dep = CellDep::new_builder()
        .out_point(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000.pack())
                    .lock(always_success_lock_script.clone())
                    .type_(Some(stake_type_script).pack())
                    .build(),
                stake_data.as_bytes(),
            ),
        )
        .build();

    // prepare crosschain lock script
    let acs_lock_script = context
        .build_script(
            &contract_out_point,
            metadata_type_script.calc_script_hash().as_bytes(),
        )
        .expect("crosschain-lock script");

    // prepare tx input/output
    let input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(600.pack())
                    .lock(acs_lock_script)
                    .build(),
                Bytes::new(),
            ),
        )
        .build();
    let output = CellOutput::new_builder()
        .capacity(600.pack())
        .lock(always_success_lock_script)
        .build();

    // build tx
    let tx = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(Bytes::new().pack())
        .cell_dep(contract_dep)
        .cell_dep(always_success_script_dep)
        .cell_dep(metadata_celldep)
        .cell_dep(stake_cell_dep)
        .build();
    let tx = context.complete_tx(tx);

    // generate blst aggregate signature
    let digest: [u8; 32] = tx.hash().unpack();
    let signature = generate_bls_signature(&digest, &bls_keypairs);
    let bls_pubkeys = bls_keypairs
        .iter()
        .map(|(_, pubkey)| {
            let mut value = [0u8; 48];
            value.copy_from_slice(pubkey);
            value
        })
        .collect::<Vec<_>>();
    let witness = {
        let witness = crosschain::Witness::new_builder()
            .signature(cs_signature(&signature))
            .bls_pubkeys(cs_blspubkey_list(&bls_pubkeys))
            .build();
        WitnessArgs::new_builder()
            .lock(Some(witness.as_bytes()).pack())
            .build()
            .as_bytes()
            .pack()
    };
    let tx = tx
        .as_advanced_builder()
        .set_witnesses(vec![witness])
        .build();

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}
