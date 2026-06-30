use super::*;
use crate::test_harness::{iterator_state_in_slot, list_in_slot};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

fn minimal_workflow_with_constant(cv: ConstValue) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: Box::from("reduce_test"),
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

#[test]
fn reduce_start_writes_initial_accumulator() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(100));
    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let output = SlotIdx::new(2);
    let initial = ConstIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        accumulator,
        initial,
        body,
        done,
        Some(output),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(accumulator)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(100)
    );
}

#[test]
fn reduce_next_applies_item_to_accumulator() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let output = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    // RP-016: iterator slot holds a (source_id, cursor=0) state list.
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(5), SlotValue::I64(6)],
        0,
    );

    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(output),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(5)
    );
}

#[test]
fn reduce_finish_writes_final_accumulator_to_output() {
    let mut run = fresh_frame();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(accumulator, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    let result = reduce_finish(
        &mut run,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(42)
    );
}

// BDD tests for reduce primitives

#[test]
fn reduce_start_returns_error_when_input_is_not_list() {
    // Given a frame with a non-list in input slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then it returns TypeMismatch
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_start_returns_error_when_output_missing() {
    // Given a frame with a list but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1)],
    );
    // When calling reduce_start with output=None
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        None,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_start_jumps_to_done_when_list_empty() {
    // Given a frame with an empty list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(100));
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![]);
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        done,
        Some(SlotIdx::new(2)),
    );
    // Then it jumps to done
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn reduce_start_returns_error_when_constant_missing() {
    // Given a frame with a list but invalid constant index
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1)],
    );
    // When calling reduce_start with ConstIdx out of bounds
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(99),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then it returns ConstOutOfBounds
    match result {
        Err(EngineError::ConstOutOfBounds { index }) => {
            assert_eq!(index, ConstIdx::new(99));
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_next_binds_first_remaining_item() {
    // Given a frame with a real iterator state (source_id, cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output = SlotIdx::new(2);
    let body = StepIdx::new(1);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(42), SlotValue::I64(99)],
        0,
    );
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        body,
        StepIdx::new(2),
        Some(output),
    );
    // Then output has first remaining item from the source list
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(42)
    );
}

#[test]
fn reduce_next_jumps_to_done_when_remaining_empty() {
    // Given a frame with empty remaining items
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![]);
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        done,
        Some(SlotIdx::new(2)),
    );
    // Then it jumps to done
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn reduce_finish_returns_error_when_output_missing() {
    // Given a frame with accumulator
    let mut run = fresh_frame();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling reduce_finish with output=None
    let result = reduce_finish(
        &mut run,
        SlotIdx::new(0),
        None,
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_finish_returns_error_when_next_missing() {
    // Given a frame
    let mut run = fresh_frame();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling reduce_finish with next=None
    let result = reduce_finish(
        &mut run,
        SlotIdx::new(0),
        Some(SlotIdx::new(1)),
        None,
        StepIdx::ZERO,
    );
    // Then it returns MissingNextStep
    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_start_writes_initial_accumulator_from_constant() {
    // Given a frame with a list and constant value 100
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(100));
    let accumulator = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        accumulator,
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then accumulator has the initial constant value
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(accumulator)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(100)
    );
}

#[test]
fn reduce_next_returns_error_when_output_missing() {
    // Given a frame with a real iterator state (source_id, cursor=0) and no output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1)],
        0,
    );
    // When calling reduce_next with output=None
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        StepIdx::new(2),
        None,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_start_increments_executed_counter() {
    // Given a frame with a list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1)],
    );
    let before = run.executed();
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn reduce_next_increments_executed_counter() {
    // Given a frame with a real iterator state (source_id, cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1)],
        0,
    );
    let before = run.executed();
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn reduce_finish_increments_executed_counter() {
    // Given a frame with accumulator
    let mut run = fresh_frame();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let before = run.executed();
    // When calling reduce_finish
    let result = reduce_finish(
        &mut run,
        SlotIdx::new(0),
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(3)),
        StepIdx::ZERO,
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn reduce_next_returns_error_when_not_list() {
    // Given a frame with non-list in iterator slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    run.write_slot(iterator_slot, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then it returns TypeMismatch
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn reduce_start_writes_cursor_state_to_iterator_slot() {
    // Given a frame with a 3-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    let output = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
    );
    // Then the output slot has a 2-element (source_id, cursor=1) state,
    // not a materialized 2-item tail.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(state.len(), 2, "cursor state must be 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn reduce_next_writes_cursor_state_to_iterator_slot() {
    // Given a frame with a 3-item source encoded as iterator state (cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output = SlotIdx::new(2);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        0,
    );
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
    );
    // Then the iterator slot has a 2-element cursor state advanced to 1
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(state.len(), 2, "cursor state must be 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

// ── Adversarial BDD tests for reduce ────────────────────────────────

#[test]
fn reduce_start_empty_list_with_initial_value_jumps_to_done() {
    // Given a frame with empty input list and initial value 100
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(100));
    let accumulator = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![]);
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        accumulator,
        ConstIdx::new(0),
        StepIdx::new(1),
        done,
        Some(SlotIdx::new(2)),
    );
    // Then it jumps to done with accumulator = 100
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_eq!(
        *run.read_slot(accumulator)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(100)
    );
}

#[test]
fn reduce_start_single_item_list_writes_cursor_state_and_jumps_to_body() {
    // Given a frame with single-item input list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    let body = StepIdx::new(1);
    let output = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(42)],
    );
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        body,
        StepIdx::new(2),
        Some(output),
    );
    // Then it jumps to body and output has a 2-element cursor state with
    // cursor=1 (= source len), signaling "next call goes to done".
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must be 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn reduce_start_first_item_in_input_cursor_state_in_output() {
    // Given a frame with a 2-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::I64(0));
    let input = SlotIdx::new(0);
    let output = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
    );
    // Then input slot has the first item and output slot has a 2-element
    // (source_id, cursor=1) state list, NOT a literal 1-item tail.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(input)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(10)
    );
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must be 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn reduce_start_null_initial_constant_succeeds() {
    // Given a frame with a Null constant and a list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::Null);
    let accumulator = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1)],
    );
    // When calling reduce_start with Null initial
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        accumulator,
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then accumulator has Null
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(accumulator)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::Null
    );
}

#[test]
fn reduce_start_bool_initial_constant_succeeds() {
    // Given a frame with a Bool constant
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let plan = minimal_workflow_with_constant(ConstValue::Bool(true));
    let accumulator = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        SlotIdx::new(0),
        vec![SlotValue::I64(1)],
    );
    // When calling reduce_start
    let result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        SlotIdx::new(0),
        accumulator,
        ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then accumulator has Bool(true) -- no type checking on initial vs items
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(accumulator)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::Bool(true)
    );
}

#[test]
fn reduce_next_single_remaining_item_produces_exhausted_cursor() {
    // Given a frame with a 1-item source encoded as iterator state (cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output = SlotIdx::new(2);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(42)],
        0,
    );
    // When calling reduce_next
    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        SlotIdx::new(1),
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
    );
    // Then output has the item and the iterator state advances to cursor=1
    // (= source len). The next call must jump to done.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(42)
    );
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must be 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn reduce_finish_copies_accumulator_to_output_verbatim() {
    // Given a frame with a non-I64 accumulator (Bool)
    let mut run = fresh_frame();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(accumulator, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling reduce_finish
    let result = reduce_finish(
        &mut run,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has the exact accumulator value (no type coercion)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::Bool(false)
    );
}
