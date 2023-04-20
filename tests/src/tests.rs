use super::*;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use helper::*;
use molecule::prelude::*;

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
