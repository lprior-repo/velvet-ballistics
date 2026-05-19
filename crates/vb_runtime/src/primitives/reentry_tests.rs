//! Body-re-entry unit tests for loop primitives.
//!
//! These tests verify that loop primitives correctly handle re-entry
//! into body steps after a previous iteration has completed.
//!
//! Bug: When a loop body step completes (Succeeded) and control returns
//! to the loop primitive, the step is still in Succeeded state.
//! The missing Succeeded→Pending transition prevents proper re-entry.

use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use crate::primitives::collect::{collect_next, collect_page, collect_start, CollectStates};
use crate::primitives::for_each::{for_each_next, for_each_start};
use crate::primitives::reduce::{reduce_next, reduce_start};
use crate::primitives::repeat::{repeat_attempt, repeat_check, repeat_start};
use crate::test_harness::list_in_slot;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(6, 10)
}

/// Verifies that for_each_next correctly handles re-entry after body completion.
/// The bug: when the body step is in Succeeded state, for_each_next cannot
/// re-enter because Succeeded→Pending transition is missing.
#[test]
fn vb_y4pa_001_for_each_two_item_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Two-item list: [10, 20]
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(10), SlotValue::I64(20)]);

    // Start: binds first item (10), tail [20] in iterator_slot
    let start_result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100u32,
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);

    // Body step starts executing
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    // Body completes
    run.mark_succeeded(body_step).unwrap();

    // Now for_each_next is called for second item
    // BUG: Without Succeeded→Pending transition, this may fail
    let next_result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );

    // The second iteration should process item 20
    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);

    // item_slot should have 20 (the second item)
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
        SlotValue::I64(20)
    );
}

/// Verifies that reduce_next correctly handles re-entry after body completion.
#[test]
fn vb_y4pa_002_reduce_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(3); // output slot holds the tail list
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Two-item list: [5, 6]
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(5), SlotValue::I64(6)]);

    // Start reduce - writes tail to output (slot 3 = iterator_slot)
    let plan = minimal_workflow();
    let start_result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        accumulator,
        vb_core::ids::ConstIdx::new(0),
        body,
        done,
        Some(iterator_slot), // tail goes here
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    // Body completes
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next re-entry - reads from iterator_slot (slot 3)
    let next_result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)), // output for the item
    );

    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

/// Verifies that collect_next correctly handles re-entry after page body completion.
#[test]
fn vb_y4pa_003_collect_next_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // 4 items with page_size=2: first page [10, 20], second page [30, 40]
    list_in_slot(&mut run, &mut store, source, vec![
        SlotValue::I64(10),
        SlotValue::I64(20),
        SlotValue::I64(30),
        SlotValue::I64(40),
    ]);

    // Start collect
    let start_result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2, // page_size=2
        body,
        done,
        Some(collector_slot),
        None,
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    // First page body completes
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // collect_next re-entry for second page
    let next_result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector_slot,
        body,
        done,
    );

    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

/// Verifies that collect_page correctly handles re-entry after body completion.
#[test]
fn vb_y4pa_004_collect_page_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(10), SlotValue::I64(20)]);

    let _ = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector_slot),
        None,
    );

    // First page body completes
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // collect_page re-entry
    let page_result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector_slot,
        body,
        done,
    );

    assert_eq!(page_result, Ok(vb_core::EngineSignal::Continue));
}

/// Verifies that repeat_attempt correctly handles re-entry after body completion.
#[test]
fn vb_y4pa_005_repeat_attempt_reentry() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // repeat_start initializes: max=3, current=0
    let start_result = repeat_start(&mut run, 3, body, done, Some(attempt_slot));
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    // Body completes (first attempt done)
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // repeat_attempt re-entry - should read current attempt and jump to body
    let attempt_result = repeat_attempt(&mut run, attempt_slot, body, done);

    assert_eq!(attempt_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

/// Verifies that repeat_check correctly handles re-entry after body completion.
#[test]
fn vb_y4pa_006_repeat_check_reentry() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let done = StepIdx::new(2);
    let next_body = StepIdx::new(1);

    // Set up: max_attempts=3, current_attempt=1 (two more attempts remain)
    let packed: i64 = (3_i64 << 32) | 1_i64;
    run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

    // Body completes
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // repeat_check re-entry - should increment to 2 and route to body
    let check_result = repeat_check(
        &mut run,
        attempt_slot,
        done,
        Some(next_body),
        StepIdx::ZERO,
    );

    assert_eq!(check_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_body);
}

/// Minimal workflow for reduce tests.
fn minimal_workflow() -> vb_core::workflow::CompiledWorkflow {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

    let parts = WorkflowParts {
        name: Box::from("reentry_test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([7; 32]),
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
        constants: vec![ConstValue::I64(0)].into_boxed_slice(),
        slot_count: 10,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

// =====================================================================
// TC-005 to TC-014: Additional reentry unit tests
// =====================================================================

/// TC-005: for_each_three_item_reentry
/// 3-item list [A, B, C]: body runs for each item without state machine error.
#[test]
fn tc005_for_each_three_item_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );

    // Start: binds first item (10)
    let start_result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100u32,
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
        SlotValue::I64(10)
    );

    // Body runs for item 10 → Succeeded
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // for_each_next for item 20
    let next1 = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );
    assert_eq!(next1, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
        SlotValue::I64(20)
    );

    // Body runs for item 20 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // for_each_next for item 30
    let next2 = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );
    assert_eq!(next2, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
        SlotValue::I64(30)
    );

    // Body runs for item 30 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // for_each_next with empty tail → done
    let next_done = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );
    assert_eq!(next_done, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

/// TC-006: for_each_empty_list_does_not_reenter
/// Empty list: for_each_start jumps to done, for_each_next not called.
#[test]
fn tc006_for_each_empty_list_does_not_reenter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(&mut run, &mut store, input, vec![]);

    // for_each_start with empty list jumps directly to done
    let start_result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100u32,
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);

    // Body step should still be Pending (never executed)
    assert_eq!(run.step_state(body).unwrap(), vb_core::frame::StepState::Pending);
}

/// TC-007: reduce_three_item_accumulator
/// 3-item list [1, 2, 3] with initial accumulator 0.
#[test]
fn tc007_reduce_three_item_accumulator() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(3);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    let plan = minimal_workflow_with_const(0i64);
    let start_result = reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        accumulator,
        vb_core::ids::ConstIdx::new(0),
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    // Body completes for item 1
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next binds item 2
    let next1 = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    );
    assert_eq!(next1, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);

    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next binds item 3
    let next2 = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    );
    assert_eq!(next2, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);

    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next with empty tail → done
    let done_result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    );
    assert_eq!(done_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

/// TC-008: reduce_body_succeeded_resets_on_reentry
/// Body step is Succeeded from previous iteration, reduce_next re-entry succeeds.
#[test]
fn tc008_reduce_body_succeeded_resets_on_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(3);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Two-item list: reduce_start binds first, tail in iterator
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(5), SlotValue::I64(6)]);

    let plan = minimal_workflow_with_const(0i64);
    reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        accumulator,
        vb_core::ids::ConstIdx::new(0),
        body,
        done,
        Some(iterator_slot),
    )
    .ok()
    .unwrap();

    // Simulate body step completing (Succeeded)
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // Verify body is Succeeded
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // reduce_next re-entry: iterator has [6], should return Continue and PC at body
    let next_result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    );
    // Verify Continue is returned and PC is at body
    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

/// TC-009: collect_four_page_reentry
/// 8 items, page_size=2 (4 pages total): pages processed correctly.
#[test]
fn tc009_collect_four_page_reentry() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // 8 items: [1, 2, 3, 4, 5, 6, 7, 8], page_size=2
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3), SlotValue::I64(4),
            SlotValue::I64(5), SlotValue::I64(6), SlotValue::I64(7), SlotValue::I64(8),
        ],
    );

    // Start collect
    let start_result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector_slot),
        None,
    );
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    let body_step = StepIdx::new(1);

    // Process 3 pages (body Succeeded each time), then re-entry for page 4
    for _page in 0..3 {
        run.mark_running(body_step).unwrap();
        run.mark_succeeded(body_step).unwrap();

        let next = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector_slot,
            body,
            done,
        );
        assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    // 4th page body completes
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // Final collect_next → done
    let final_next = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector_slot,
        body,
        done,
    );
    assert_eq!(final_next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

/// TC-010: collect_page_body_succeeded_resets
/// Body step Succeeded after first page, jump_to_body resets for re-entry.
#[test]
fn tc010_collect_page_body_succeeded_resets() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30), SlotValue::I64(40)],
    );

    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector_slot),
        None,
    )
    .ok()
    .unwrap();

    // First page body completes → Succeeded
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // collect_page re-entry via jump_to_body
    let page_result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector_slot,
        body,
        done,
    );
    assert_eq!(page_result, Ok(vb_core::EngineSignal::Continue));
    // Body step should now be Pending
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Pending);
}

/// TC-011: repeat_max_attempts_exhausted
/// max_attempts=3: 3 attempts complete, repeat_check routes to done.
#[test]
fn tc011_repeat_max_attempts_exhausted() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let done = StepIdx::new(2);
    let next_body = StepIdx::new(1);

    // max=3, current=2 (after increment will be 3 which equals max)
    let packed: i64 = (3_i64 << 32) | 2_i64;
    run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // repeat_check: current=2, next_attempt=3, 3 >= 3 → done
    let check_result = repeat_check(
        &mut run,
        attempt_slot,
        done,
        Some(next_body),
        StepIdx::ZERO,
    );
    assert_eq!(check_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);

    // Attempt counter updated to 3
    let updated = match *run.read_slot(attempt_slot).unwrap() {
        SlotValue::I64(v) => v,
        _ => panic!("expected I64"),
    };
    let (_, current) = decode_packed(updated);
    assert_eq!(current, 3);
}

/// TC-012: repeat_body_state_resets_on_each_attempt
/// repeat_attempt called 3 times sequentially, each re-entry succeeds.
#[test]
fn tc012_repeat_body_state_resets_on_each_attempt() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // max=3, current=0
    let start_result = repeat_start(&mut run, 3, body, done, Some(attempt_slot));
    assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

    let body_step = StepIdx::new(1);

    for _attempt in 0..3 {
        // Body completes (Succeeded)
        run.mark_running(body_step).unwrap();
        run.mark_succeeded(body_step).unwrap();

        // repeat_attempt re-entry: should return Continue and set PC to body
        let attempt_result = repeat_attempt(&mut run, attempt_slot, body, done);
        assert_eq!(attempt_result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }
}

/// TC-013: for_each_next_jumps_to_done_when_iterator_empty
/// Empty iterator (but body step is Succeeded from prior run).
#[test]
fn tc013_for_each_next_jumps_to_done_when_iterator_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let iterator_slot = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Empty iterator
    list_in_slot(&mut run, &mut store, iterator_slot, vec![]);

    // Body step is Succeeded from previous iteration
    let body_step = StepIdx::new(1);
    run.mark_succeeded(body_step).unwrap();

    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );

    // Should jump to done (empty iterator takes precedence over body state)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

/// TC-014: reduce_next_jumps_to_done_when_remaining_empty
/// Empty remaining list: reduce_next jumps to done.
#[test]
fn tc014_reduce_next_jumps_to_done_when_remaining_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let iterator_slot = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Empty remaining list
    list_in_slot(&mut run, &mut store, iterator_slot, vec![]);

    let result = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(2)),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

// =====================================================================
// BDD Given/When/Then Scenarios: GWT-RE-1 to GWT-RE-6
// =====================================================================

/// GWT-RE-1: for_each body re-entry after Succeeded
/// Given: for_each over [Item1, Item2], body ran Item1 → Succeeded
/// When: for_each_next called for Item2
/// Then: jump_to_body transitions Succeeded → Pending, body processes Item2.
#[test]
fn gwt_re1_for_each_body_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1), SlotValue::I64(2)]);

    // for_each_start binds Item1 (1)
    for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100u32,
        body,
        done,
        Some(iterator_slot),
    )
    .ok()
    .unwrap();

    // Body runs Item1 → Succeeded
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // for_each_next for Item2 (re-entry)
    let next = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );

    // THEN: Continue returned, PC at body, Item2 bound, body step Pending
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
        SlotValue::I64(2)
    );
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Pending);
}

/// GWT-RE-2: reduce body re-entry after Succeeded
/// Given: reduce over [A, B, C], body ran A→Succeeded, B→Succeeded
/// When: reduce_next for C
/// Then: jump_to_body transitions Succeeded → Pending, body runs C.
#[test]
fn gwt_re2_reduce_body_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(3);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );

    let plan = minimal_workflow_with_const(0i64);
    reduce_start(
        &plan, &mut run, &mut store, input, accumulator,
        vb_core::ids::ConstIdx::new(0), body, done, Some(iterator_slot),
    )
    .ok()
    .unwrap();

    let body_step = StepIdx::new(1);

    // Body ran for A=10 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next binds B=20
    reduce_next(
        &mut run, &mut store, iterator_slot, accumulator,
        body, done, Some(SlotIdx::new(4)),
    )
    .ok()
    .unwrap();

    // Body ran for B=20 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // reduce_next for C=30 (re-entry)
    let next = reduce_next(
        &mut run, &mut store, iterator_slot, accumulator,
        body, done, Some(SlotIdx::new(4)),
    );

    // THEN: Continue, PC at body, C bound, body step Pending
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Pending);
}

/// GWT-RE-3: collect_page re-entry after page body Succeeded
/// Given: collect with page_size=2 over [A,B,C,D], page1 body → Succeeded
/// When: collect_page re-entry for page2
/// Then: jump_to_body transitions Succeeded → Pending.
#[test]
fn gwt_re3_collect_page_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3), SlotValue::I64(4)],
    );

    collect_start(
        &mut run, &mut store, &mut states, source,
        100, 2, body, done, Some(collector_slot), None,
    )
    .ok()
    .unwrap();

    let body_step = StepIdx::new(1);

    // Page1 body completes → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // collect_page re-entry
    let page = collect_page(
        &mut run, &mut store, &mut states,
        collector_slot, body, done,
    );

    // THEN: Continue, body step Pending
    assert_eq!(page, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Pending);
}

/// GWT-RE-4: repeat_attempt re-entry after attempt Succeeded
/// Given: repeat with max_attempts=3, attempt 1 → Succeeded
/// When: repeat_attempt for attempt 2
/// Then: jump_to_body transitions Succeeded → Pending, body runs attempt 2.
#[test]
fn gwt_re4_repeat_attempt_reentry_after_succeeded() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    repeat_start(&mut run, 3, body, done, Some(attempt_slot))
        .ok()
        .unwrap();

    let body_step = StepIdx::new(1);

    // Attempt 1 runs → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // repeat_attempt for attempt 2 (re-entry)
    let attempt = repeat_attempt(&mut run, attempt_slot, body, done);

    // THEN: Continue, PC at body, body step Pending
    assert_eq!(attempt, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Pending);
}

/// GWT-RE-5: repeat_check loops back to body after attempt Succeeded
/// Given: repeat with max_attempts=3, attempt 2 → Succeeded
/// When: repeat_check routes back to body
/// Then: jump_to_body transitions Succeeded → Pending.
#[test]
fn gwt_re5_repeat_check_loops_back_after_succeeded() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let done = StepIdx::new(2);
    let next_body = StepIdx::new(1);

    // max=3, current=1 (after increment will be 2, which is < max=3, so loops back)
    let packed: i64 = (3_i64 << 32) | 1_i64;
    run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

    let body_step = StepIdx::new(1);

    // Attempt 1 runs → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(run.step_state(body_step).unwrap(), vb_core::frame::StepState::Succeeded);

    // repeat_check: next_attempt=2, 2 < 3 is true, so loops back to body_entry
    let check = repeat_check(
        &mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO,
    );

    // THEN: Continue, PC at body_entry
    assert_eq!(check, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_body);
}

/// GWT-RE-6: Succeeded→Running transition rejected by state machine (negative)
/// Given: body step in Succeeded state
/// When: Plain jump_to (not jump_to_body) is called, and engine tries Succeeded→Running
/// Then: validate_transition returns Err("invalid_state_transition").
#[test]
fn gwt_re6_succeeded_to_running_rejected() {
    use vb_core::frame::StepState;

    // Succeeded → Running is NOT a valid transition
    let is_valid = vb_core::frame::is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Running,
    );
    assert!(
        !is_valid,
        "Succeeded→Running must be invalid per VALID_TRANSITIONS"
    );

    // The only valid transitions from Succeeded are:
    // - Succeeded → Succeeded (idempotent)
    // - Succeeded → Pending (for loop re-entry via jump_to_body)
    let can_go_to_pending = vb_core::frame::is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Pending,
    );
    assert!(
        can_go_to_pending,
        "Succeeded→Pending must be valid (this is why jump_to_body exists)"
    );
}

// =====================================================================
// Helper functions
// =====================================================================

fn minimal_workflow_with_const(cv: i64) -> vb_core::workflow::CompiledWorkflow {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    let parts = WorkflowParts {
        name: Box::from("reentry_test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([7; 32]),
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
        constants: vec![ConstValue::I64(cv)].into_boxed_slice(),
        slot_count: 10,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

fn decode_packed(packed: i64) -> (u16, u16) {
    let max_attempts = (packed >> 32) as u16;
    let current_attempt = (packed & 0xFFFF) as u16;
    (max_attempts, current_attempt)
}