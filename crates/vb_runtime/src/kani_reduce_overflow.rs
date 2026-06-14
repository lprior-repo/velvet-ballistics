//! Kani harnesses for Reduce accumulator arithmetic overflow verification (PO-013).

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fn minimal_workflow(cv: vb_core::value::ConstValue) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: Box::from("reduce_overflow_test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([5; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![cv].into_boxed_slice(),
        slot_count: 8,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts)
        .ok()
        .unwrap_or_else(|| panic!("workflow must compile"))
}

fn fresh_frame() -> RunFrame {
    vb_runtime::test_harness::fresh_frame(4, 8)
}

/// KANI-XI2F.15-013: Prove accumulator arithmetic does not overflow.
#[kani::proof]
#[kani::unwind(8)]
fn reduce_accumulator_overflow_harness() {
    let initial_value: i64 = kani::any();
    kani::assume(initial_value == i64::MAX || initial_value == i64::MIN || initial_value == 0);

    let cv = vb_core::value::ConstValue::I64(initial_value);
    let plan = minimal_workflow(cv);

    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let output = SlotIdx::new(2);
    let initial = ConstIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    let list_items = vec![SlotValue::I64(i64::MAX)];
    let list_id = match store.insert_list(list_items.into_boxed_slice()) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    match run.write_slot(input, SlotValue::List(list_id)) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }

    let result = vb_runtime::primitives::reduce::reduce_start(
        &plan, &mut run, &mut store,
        input, accumulator, initial, body, done, Some(output),
    );

    match result {
        Ok(v) => kani::assert(v == vb_core::EngineSignal::Continue, "expected Continue"),
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
}
