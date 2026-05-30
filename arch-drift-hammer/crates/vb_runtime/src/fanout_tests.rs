//! Behavior tests for fanout / parallel execution using together primitives.
//!
//! The `together_*` family (together_start, together_branch, together_join)
//! is the canonical fanout mechanism in vb_runtime. Branches execute
//! sequentially in declaration order within the deterministic synchronous
//! runtime, but the primitives model true parallel fork/join semantics:
//! independent branch state, accumulator-based result collection, and
//! taint-aware output merging.

use super::*;
use crate::test_harness::list_in_slot;
use vb_core::errors::EngineError;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

fn fresh_frame(step_count: u16, slot_count: u16) -> RunFrame {
    crate::test_harness::fresh_frame(step_count, slot_count)
}

// =========================================================================
// 1. Fanout with empty task list
// =========================================================================

#[test]
fn fanout_empty_branches_returns_invalid_compiled_workflow() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);

    let result = together_start(&mut run, &mut store, &[], StepIdx::new(2), Some(output));

    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "together_start requires at least one branch");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_empty_branches_does_not_modify_output_slot() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    run.write_slot(output, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let _ = together_start(&mut run, &mut store, &[], StepIdx::new(2), Some(output));

    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read")),
        SlotValue::I64(999)
    );
}

// =========================================================================
// 2. Fanout with single task
// =========================================================================

#[test]
fn fanout_single_branch_full_lifecycle_start_branch_join() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);
    let next = StepIdx::new(5);

    let result = together_start(&mut run, &mut store, &[entry], join, Some(accumulator));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), entry);

    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let result = together_branch(
        &mut run, &mut store,
        0, entry, join, accumulator, Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), entry);

    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run, &mut store,
        1, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next);

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), 1);
    assert_eq!(items.get(0), Some(&SlotValue::I64(42)));
}

#[test]
fn fanout_single_branch_null_result_skips_append() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);
    let next = StepIdx::new(5);

    let result = together_start(&mut run, &mut store, &[entry], join, Some(accumulator));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(output, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let result = together_branch(
        &mut run, &mut store,
        0, entry, join, accumulator, Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run, &mut store,
        1, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), 0);
}

// =========================================================================
// 3. Fanout with many parallel tasks
// =========================================================================

#[test]
fn fanout_many_branches_all_results_collected_in_order() {
    let mut run = fresh_frame(16, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(10);
    let join = StepIdx::new(11);
    let next = StepIdx::new(12);

    let branches: Vec<StepIdx> = vec![entry; 5];
    let result = together_start(&mut run, &mut store, &branches, join, Some(accumulator));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let values: [SlotValue; 5] = [
        SlotValue::I64(10),
        SlotValue::I64(20),
        SlotValue::I64(30),
        SlotValue::I64(40),
        SlotValue::I64(50),
    ];

    for (i, &val) in values.iter().enumerate() {
        let branch = u16::try_from(i).ok().unwrap_or(0);
        run.write_slot(output, val)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        let result = together_branch(
            &mut run, &mut store,
            branch, entry, join, accumulator, Some(output),
        );
        assert_eq!(
            result,
            Ok(vb_core::EngineSignal::Continue),
            "branch {branch} must succeed"
        );
    }

    assert!(run.add_parallel_in_flight(5).is_ok());
    let result = together_join(
        &mut run, &mut store,
        5, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), 5);
    for (i, expected) in values.iter().enumerate() {
        assert_eq!(items.get(i), Some(expected));
    }
}

#[test]
fn fanout_hundred_branches_collects_all_results() {
    let branch_count: u16 = 100;
    let mut run = fresh_frame(branch_count + 4, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(branch_count + 1);
    let join = StepIdx::new(branch_count + 2);
    let next = StepIdx::new(branch_count + 3);

    let branches: Vec<StepIdx> = (0..branch_count).map(|_| entry).collect();
    let result = together_start(&mut run, &mut store, &branches, join, Some(accumulator));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    for i in 0..branch_count {
        let value = SlotValue::I64(i64::from(i));
        run.write_slot(output, value)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        let result = together_branch(
            &mut run, &mut store,
            i, entry, join, accumulator, Some(output),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    }

    assert!(run.add_parallel_in_flight(branch_count).is_ok());
    let result = together_join(
        &mut run, &mut store,
        branch_count,
        accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), branch_count as usize);
    for (i, item) in items.iter().enumerate() {
        assert_eq!(item, &SlotValue::I64(i64::try_from(i).ok().unwrap_or(0)));
    }
}

// =========================================================================
// 4. Fanout join: all completed
// =========================================================================

#[test]
fn fanout_join_all_completed_merges_accumulated_results() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(5);

    list_in_slot(
        &mut run, &mut store, accumulator,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    run.write_slot(output, SlotValue::I64(3))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(3).is_ok());
    let result = together_join(
        &mut run, &mut store,
        3, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next);

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), 3);
}

#[test]
fn fanout_join_all_completed_decrements_parallel_in_flight() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(5);

    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(7))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(3).is_ok());
    assert_eq!(run.parallel_in_flight(), 3);

    let result = together_join(
        &mut run, &mut store,
        3, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.parallel_in_flight(), 0);
}

// =========================================================================
// 5. Fanout join: any completed (race / early join)
// =========================================================================

#[test]
fn fanout_join_with_fewer_branches_than_in_flight_leaves_remaining() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(5);

    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(5).is_ok());
    let result = together_join(
        &mut run, &mut store,
        2, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.parallel_in_flight(), 3);
}

#[test]
fn fanout_join_with_more_branches_than_in_flight_underflows_error() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(5);

    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run, &mut store,
        3, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    match result {
        Err(EngineError::InternalInvariantViolation { reason }) => {
            assert_eq!(reason, "parallel_in_flight underflow");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_single_branch_join_immediately_after_start_no_intermediate_branches() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);
    let next = StepIdx::new(5);

    let result = together_start(&mut run, &mut store, &[entry], join, Some(accumulator));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(output, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run, &mut store,
        1, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

// =========================================================================
// 6. Fanout error propagation from child
// =========================================================================

#[test]
fn fanout_branch_error_propagates_type_mismatch_when_accumulator_corrupted() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);

    run.write_slot(accumulator, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let result = together_branch(
        &mut run, &mut store,
        1, StepIdx::new(3), StepIdx::new(4), accumulator, Some(output),
    );

    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_error_propagation_preserves_output_slot_after_branch_failure() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);

    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let result = together_branch(
        &mut run, &mut store,
        1, StepIdx::new(3), StepIdx::new(4), accumulator, Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(accumulator, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let result = together_branch(
        &mut run, &mut store,
        2, StepIdx::new(3), StepIdx::new(4), accumulator, Some(output),
    );
    assert!(result.is_err());

    let out_val = *run.read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read"));
    assert_eq!(out_val, SlotValue::I64(20));
}

#[test]
fn fanout_join_error_propagates_missing_output_slot() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);

    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    assert!(run.add_parallel_in_flight(1).is_ok());

    let result = together_join(
        &mut run, &mut store,
        1, accumulator, None, Some(StepIdx::new(5)), StepIdx::ZERO,
    );

    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_join_error_propagates_missing_next_step() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);

    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(7))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    assert!(run.add_parallel_in_flight(1).is_ok());

    let result = together_join(
        &mut run, &mut store,
        1, accumulator, Some(output), None, StepIdx::new(3),
    );

    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::new(3));
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

// =========================================================================
// 7. Fanout cancellation mid-execution
// =========================================================================

#[test]
fn fanout_cancel_after_start_decrements_in_flight_to_zero() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);

    let result = together_start(
        &mut run, &mut store, &[entry, entry, entry], join, Some(accumulator),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    assert!(run.sub_parallel_in_flight(3).is_ok());
    assert_eq!(run.parallel_in_flight(), 0);
}

#[test]
fn fanout_cancel_after_partial_execution_leaves_incomplete_branches() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);

    let result = together_start(
        &mut run, &mut store, &[entry, entry, entry, entry], join, Some(accumulator),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(output, SlotValue::I64(100))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let result = together_branch(
        &mut run, &mut store,
        0, entry, join, accumulator, Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(output, SlotValue::I64(200))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let result = together_branch(
        &mut run, &mut store,
        1, entry, join, accumulator, Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    assert!(run.sub_parallel_in_flight(2).is_ok());
    assert_eq!(run.parallel_in_flight(), 2);
}

#[test]
fn fanout_cancel_remaining_branches_then_join_partial_results() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);
    let next = StepIdx::new(5);

    let result = together_start(
        &mut run, &mut store, &[entry, entry, entry], join, Some(accumulator),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(output, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store, 0, entry, join, accumulator, Some(output),
    );

    run.write_slot(output, SlotValue::I64(2))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store, 1, entry, join, accumulator, Some(output),
    );

    assert!(run.sub_parallel_in_flight(1).is_ok());

    run.write_slot(output, SlotValue::I64(3))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run, &mut store,
        1, accumulator, Some(output), Some(next), StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let SlotValue::List(list_id) =
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read"))
    else {
        panic!("expected list");
    };
    let items = store.list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(items.len(), 1);
}

// =========================================================================
// 8. Fanout resource limits
// =========================================================================

#[test]
fn fanout_resource_limit_parallel_exceeded_when_current_plus_count_gt_max() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);

    run.set_max_parallel_in_flight(2);

    assert!(run.add_parallel_in_flight(2).is_ok());

    let result = together_start(
        &mut run, &mut store,
        &[StepIdx::new(1), StepIdx::new(2)],
        StepIdx::new(3),
        Some(output),
    );

    match result {
        Err(EngineError::ParallelLimitExceeded { limit }) => {
            assert_eq!(limit, 2);
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_resource_limit_parallel_respected_when_within_limit() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);

    run.set_max_parallel_in_flight(10);

    let result = together_start(
        &mut run, &mut store,
        &[StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)],
        StepIdx::new(4),
        Some(output),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.parallel_in_flight(), 3);
}

#[test]
fn fanout_resource_limit_zero_max_parallel_rejects_single_branch() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);

    run.set_max_parallel_in_flight(0);

    let result = together_start(
        &mut run, &mut store,
        &[StepIdx::new(1)],
        StepIdx::new(2),
        Some(output),
    );

    match result {
        Err(EngineError::ParallelLimitExceeded { limit }) => {
            assert_eq!(limit, 0);
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn fanout_resource_limit_exactly_at_max_succeeds() {
    let mut run = fresh_frame(8, 8);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);

    run.set_max_parallel_in_flight(3);

    let result = together_start(
        &mut run, &mut store,
        &[StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)],
        StepIdx::new(4),
        Some(output),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.parallel_in_flight(), 3);
}

// =========================================================================
// 9. Nested fanout (fanout within fanout)
// =========================================================================

#[test]
fn nested_fanout_outer_and_inner_independent_collections() {
    let mut run = fresh_frame(16, 8);
    let mut store = ValueStore::new();

    let outer_acc = SlotIdx::new(0);
    let outer_out = SlotIdx::new(1);
    let inner_acc = SlotIdx::new(2);
    let inner_out = SlotIdx::new(3);

    let outer_entry = StepIdx::new(10);
    let inner_entry = StepIdx::new(11);
    let outer_join = StepIdx::new(12);
    let inner_join = StepIdx::new(13);
    let outer_next = StepIdx::new(14);
    let inner_next = StepIdx::new(15);

    let outer_result = together_start(
        &mut run, &mut store,
        &[outer_entry, outer_entry],
        outer_join,
        Some(outer_acc),
    );
    assert_eq!(outer_result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(outer_out, SlotValue::I64(100))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let branch0_result = together_branch(
        &mut run, &mut store,
        0, outer_entry, outer_join, outer_acc, Some(outer_out),
    );
    assert_eq!(branch0_result, Ok(vb_core::EngineSignal::Continue));

    let inner_result = together_start(
        &mut run, &mut store,
        &[inner_entry, inner_entry, inner_entry],
        inner_join,
        Some(inner_acc),
    );
    assert_eq!(inner_result, Ok(vb_core::EngineSignal::Continue));

    let inner_values = [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)];
    for (i, &val) in inner_values.iter().enumerate() {
        run.write_slot(inner_out, val)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        let result = together_branch(
            &mut run, &mut store,
            u16::try_from(i).ok().unwrap_or(0),
            inner_entry, inner_join, inner_acc, Some(inner_out),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    }

    assert!(run.add_parallel_in_flight(3).is_ok());
    let inner_join_result = together_join(
        &mut run, &mut store,
        3, inner_acc, Some(inner_out), Some(inner_next), StepIdx::ZERO,
    );
    assert_eq!(inner_join_result, Ok(vb_core::EngineSignal::Continue));

    let inner_list_id = match *run.read_slot(inner_out)
        .ok()
        .unwrap_or_else(|| panic!("read"))
    {
        SlotValue::List(id) => id,
        other => panic!("expected list, got {other:?}"),
    };
    let inner_items = store.list(inner_list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(inner_items.len(), 3);

    run.write_slot(outer_out, SlotValue::I64(200))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let branch1_result = together_branch(
        &mut run, &mut store,
        1, outer_entry, outer_join, outer_acc, Some(outer_out),
    );
    assert_eq!(branch1_result, Ok(vb_core::EngineSignal::Continue));

    assert!(run.add_parallel_in_flight(2).is_ok());
    let outer_join_result = together_join(
        &mut run, &mut store,
        2, outer_acc, Some(outer_out), Some(outer_next), StepIdx::ZERO,
    );
    assert_eq!(outer_join_result, Ok(vb_core::EngineSignal::Continue));

    let outer_list_id = match *run.read_slot(outer_out)
        .ok()
        .unwrap_or_else(|| panic!("read"))
    {
        SlotValue::List(id) => id,
        other => panic!("expected list, got {other:?}"),
    };
    let outer_items = store.list(outer_list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(outer_items.len(), 2);
}

#[test]
fn nested_fanout_inner_failure_does_not_corrupt_outer_state() {
    let mut run = fresh_frame(16, 8);
    let mut store = ValueStore::new();

    let outer_acc = SlotIdx::new(0);
    let outer_out = SlotIdx::new(1);
    let inner_acc = SlotIdx::new(2);
    let inner_out = SlotIdx::new(3);

    let outer_entry = StepIdx::new(10);
    let inner_entry = StepIdx::new(11);
    let outer_join = StepIdx::new(12);
    let inner_join = StepIdx::new(13);

    let outer_result = together_start(
        &mut run, &mut store,
        &[outer_entry, outer_entry],
        outer_join,
        Some(outer_acc),
    );
    assert_eq!(outer_result, Ok(vb_core::EngineSignal::Continue));

    run.write_slot(outer_out, SlotValue::I64(100))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store,
        0, outer_entry, outer_join, outer_acc, Some(outer_out),
    );

    let _ = together_start(
        &mut run, &mut store,
        &[inner_entry],
        inner_join,
        Some(inner_acc),
    );

    run.write_slot(inner_acc, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(inner_out, SlotValue::I64(5))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    let inner_branch_result = together_branch(
        &mut run, &mut store,
        1, inner_entry, inner_join, inner_acc, Some(inner_out),
    );
    assert!(inner_branch_result.is_err());

    let outer_acc_val = *run.read_slot(outer_acc)
        .ok()
        .unwrap_or_else(|| panic!("read"));
    let SlotValue::List(outer_list_id) = outer_acc_val else {
        panic!("expected list");
    };
    let outer_items = store.list(outer_list_id)
        .ok()
        .unwrap_or_else(|| panic!("list"));
    assert_eq!(outer_items.len(), 1);
    assert_eq!(outer_items.get(0), Some(&SlotValue::I64(100)));
}

#[test]
fn nested_fanout_three_levels_deep_all_succeed() {
    let mut run = fresh_frame(24, 8);
    let mut store = ValueStore::new();

    let l1_acc = SlotIdx::new(0);
    let l1_out = SlotIdx::new(1);
    let l2_acc = SlotIdx::new(2);
    let l2_out = SlotIdx::new(3);
    let l3_acc = SlotIdx::new(4);
    let l3_out = SlotIdx::new(5);

    let l1_entry = StepIdx::new(16);
    let l2_entry = StepIdx::new(17);
    let l3_entry = StepIdx::new(18);
    let l1_join = StepIdx::new(19);
    let l2_join = StepIdx::new(20);
    let l3_join = StepIdx::new(21);
    let l3_next = StepIdx::new(22);
    let _l1_next = StepIdx::new(23);

    let _ = together_start(
        &mut run, &mut store,
        &[l1_entry], l1_join, Some(l1_acc),
    );

    run.write_slot(l1_out, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store,
        0, l1_entry, l1_join, l1_acc, Some(l1_out),
    );

    let _ = together_start(
        &mut run, &mut store,
        &[l2_entry], l2_join, Some(l2_acc),
    );

    run.write_slot(l2_out, SlotValue::I64(2))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store,
        0, l2_entry, l2_join, l2_acc, Some(l2_out),
    );

    let _ = together_start(
        &mut run, &mut store,
        &[l3_entry], l3_join, Some(l3_acc),
    );

    run.write_slot(l3_out, SlotValue::I64(3))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let _ = together_branch(
        &mut run, &mut store,
        0, l3_entry, l3_join, l3_acc, Some(l3_out),
    );

    assert!(run.add_parallel_in_flight(1).is_ok());
    let r3 = together_join(
        &mut run, &mut store,
        1, l3_acc, Some(l3_out), Some(l3_next), StepIdx::ZERO,
    );
    assert_eq!(r3, Ok(vb_core::EngineSignal::Continue));

    // Level 3 output is a list of 1 item
    match *run.read_slot(l3_out).ok().unwrap_or_else(|| panic!("read")) {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("list"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(3)));
        }
        other => panic!("expected list, got {other:?}"),
    }

    // Level 1 accumulator still intact
    match *run.read_slot(l1_acc).ok().unwrap_or_else(|| panic!("read")) {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("list"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
        }
        other => panic!("expected list, got {other:?}"),
    }
}

// =========================================================================
// 10. Proptest: fanout result count matches input
// =========================================================================

#[cfg(test)]
mod proptest {
    use proptest::prelude::*;
    use crate::primitives::together::*;
    use crate::test_harness::fresh_frame;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::value::SlotValue;
    use vb_core::value_store::ValueStore;

    proptest! {
        #[test]
        fn prop_fanout_result_count_equals_branch_count(
            branch_count in 1u16..=64,
        ) {
            let step_count = branch_count + 4;
            let mut run = fresh_frame(step_count, 8);
            let mut store = ValueStore::new();
            let accumulator = SlotIdx::new(0);
            let output = SlotIdx::new(1);
            let entry = StepIdx::new(branch_count + 1);
            let join = StepIdx::new(branch_count + 2);
            let next = StepIdx::new(branch_count + 3);

            let branches: Vec<StepIdx> = (0..branch_count).map(|_| entry).collect();
            let start_result = together_start(
                &mut run, &mut store, &branches, join, Some(accumulator),
            );
            prop_assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

            for i in 0..branch_count {
                let value = SlotValue::I64(i64::from(i));
                run.write_slot(output, value)
                    .ok()
                    .unwrap_or_else(|| panic!("write"));
                let branch_result = together_branch(
                    &mut run, &mut store,
                    i, entry, join, accumulator, Some(output),
                );
                prop_assert_eq!(branch_result, Ok(vb_core::EngineSignal::Continue));
            }

            let add_result = run.add_parallel_in_flight(branch_count);
            prop_assert!(add_result.is_ok());

            let join_result = together_join(
                &mut run, &mut store,
                branch_count, accumulator, Some(output), Some(next), StepIdx::ZERO,
            );
            prop_assert_eq!(join_result, Ok(vb_core::EngineSignal::Continue));

            let final_value = *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read"));
            let SlotValue::List(list_id) = final_value else {
                panic!("expected list");
            };
            let items = store.list(list_id)
                .ok()
                .unwrap_or_else(|| panic!("list"));
            prop_assert_eq!(
                items.len(),
                branch_count as usize,
                "result list length must equal branch count"
            );
        }

        #[test]
        fn prop_fanout_accumulator_items_non_decreasing(
            steps in prop::collection::vec(1u16..=4, 1..12)
        ) {
            let mut run = fresh_frame(16, 8);
            let mut store = ValueStore::new();
            let accumulator = SlotIdx::new(0);
            let output = SlotIdx::new(1);
            let entry = StepIdx::new(10);
            let join = StepIdx::new(11);

            let total_branches: u16 = steps.iter().sum();
            let branches: Vec<StepIdx> = (0..steps.len()).map(|_| entry).collect();
            let start_result = together_start(
                &mut run, &mut store, &branches, join, Some(accumulator),
            );
            prop_assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

            let mut acc_count = 0usize;
            for (branch_idx, _step_count) in steps.iter().enumerate() {
                let branch = u16::try_from(branch_idx).ok().unwrap_or(0);
                let value = SlotValue::I64(i64::try_from(branch).ok().unwrap_or(0));
                run.write_slot(output, value)
                    .ok()
                    .unwrap_or_else(|| panic!("write"));
                let result = together_branch(
                    &mut run, &mut store,
                    branch, entry, join, accumulator, Some(output),
                );
                prop_assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

                if branch > 0 {
                    acc_count = acc_count.saturating_add(1);
                }

                let SlotValue::List(list_id) =
                    *run.read_slot(accumulator)
                        .ok()
                        .unwrap_or_else(|| panic!("read"))
                else {
                    panic!("expected list");
                };
                let items = store.list(list_id)
                    .ok()
                    .unwrap_or_else(|| panic!("list"));
                prop_assert!(items.len() >= acc_count);
            }
        }

        #[test]
        fn prop_fanout_with_interleaved_values_collects_all(
            values in prop::collection::vec(any::<i64>(), 1..16)
        ) {
            let branch_count = u16::try_from(values.len()).ok().unwrap_or(0);
            if branch_count == 0 {
                return Ok::<(), proptest::test_runner::TestCaseError>(());
            }

            let mut run = fresh_frame(branch_count + 4, 8);
            let mut store = ValueStore::new();
            let accumulator = SlotIdx::new(0);
            let output = SlotIdx::new(1);
            let entry = StepIdx::new(branch_count + 1);
            let join = StepIdx::new(branch_count + 2);
            let next = StepIdx::new(branch_count + 3);

            let branches: Vec<StepIdx> = (0..branch_count).map(|_| entry).collect();
            let start_result = together_start(
                &mut run, &mut store, &branches, join, Some(accumulator),
            );
            prop_assert_eq!(start_result, Ok(vb_core::EngineSignal::Continue));

            for (i, &val) in values.iter().enumerate() {
                run.write_slot(output, SlotValue::I64(val))
                    .ok()
                    .unwrap_or_else(|| panic!("write"));
                let result = together_branch(
                    &mut run, &mut store,
                    u16::try_from(i).ok().unwrap_or(0),
                    entry, join, accumulator, Some(output),
                );
                prop_assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
            }

            let add_result = run.add_parallel_in_flight(branch_count);
            prop_assert!(add_result.is_ok());

            let join_result = together_join(
                &mut run, &mut store,
                branch_count, accumulator, Some(output), Some(next), StepIdx::ZERO,
            );
            prop_assert_eq!(join_result, Ok(vb_core::EngineSignal::Continue));

            let SlotValue::List(list_id) =
                *run.read_slot(output)
                    .ok()
                    .unwrap_or_else(|| panic!("read"))
            else {
                panic!("expected list");
            };
            let items = store.list(list_id)
                .ok()
                .unwrap_or_else(|| panic!("list"));
            prop_assert_eq!(items.len(), values.len());
            for (i, &val) in values.iter().enumerate() {
                prop_assert_eq!(items.get(i), Some(&SlotValue::I64(val)));
            }
        }
    }
}
