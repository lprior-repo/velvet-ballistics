//! Accumulator overflow tests for Reduce primitive (PO-014).

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

fn list_in_slot(run: &mut RunFrame, store: &mut ValueStore, slot: SlotIdx, items: Vec<SlotValue>) {
    let list_id = store.insert_list(items.into_boxed_slice()).unwrap();
    run.write_slot(slot, SlotValue::List(list_id)).unwrap();
}

#[test]
fn accumulator_overflow_max_initial() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow(vb_core::value::ConstValue::I64(i64::MAX));
    list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![SlotValue::I64(1)]);

    let result = vb_runtime::primitives::reduce::reduce_start(
        &plan, &mut run, &mut store,
        SlotIdx::new(0), SlotIdx::new(1),
        ConstIdx::new(0), StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(2)),
    );

    // Accumulator overflow test: ensure the operation doesn't panic on i64::MAX initial
    match result {
        Ok(signal) => assert_eq!(
            signal,
            vb_core::EngineSignal::Continue,
            "overflow initial must signal Continue (not panic)"
        ),
        Err(e) => assert!(
            !e.is_empty(),
            "error must have non-empty description when overflow occurs"
        ),
    }
}

#[test]
fn accumulator_overflow_zero_initial() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow(vb_core::value::ConstValue::I64(0));
    list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![SlotValue::I64(1), SlotValue::I64(2)]);

    let result = vb_runtime::primitives::reduce::reduce_start(
        &plan, &mut run, &mut store,
        SlotIdx::new(0), SlotIdx::new(1),
        ConstIdx::new(0), StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(2)),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(*run.read_slot(SlotIdx::new(1)).unwrap(), SlotValue::I64(0));
}

#[test]
fn accumulator_overflow_reduce_next_max() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow(vb_core::value::ConstValue::I64(0));
    run.write_slot(SlotIdx::new(1), SlotValue::I64(i64::MAX)).unwrap();
    list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![SlotValue::I64(1)]);

    let result = vb_runtime::primitives::reduce::reduce_next(
        &mut run, &mut store,
        SlotIdx::new(0), SlotIdx::new(1),
        StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(2)),
    );

    // reduce_next with i64::MAX accumulator must not panic
    match result {
        Ok(signal) => assert_eq!(
            signal,
            vb_core::EngineSignal::Continue,
            "reduce_next with i64::MAX accumulator must signal Continue"
        ),
        Err(e) => assert!(
            !e.is_empty(),
            "reduce_next error must have non-empty description"
        ),
    }
}

#[test]
fn accumulator_overflow_reduce_finish_large() {
    let mut run = fresh_frame();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(i64::MAX)).unwrap();

    let result = vb_runtime::primitives::reduce::reduce_finish(
        &mut run, SlotIdx::new(0), Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)), StepIdx::ZERO,
    );

    // reduce_finish with i64::MAX accumulator must not panic
    match result {
        Ok(signal) => assert_eq!(
            signal,
            vb_core::EngineSignal::Continue,
            "reduce_finish with i64::MAX accumulator must signal Continue"
        ),
        Err(e) => assert!(
            !e.is_empty(),
            "reduce_finish error must have non-empty description when overflow occurs"
        ),
    }
}
