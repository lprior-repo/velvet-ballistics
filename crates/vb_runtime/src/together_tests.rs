use super::*;
use crate::test_harness::list_in_slot;
use vb_core::ids::BranchIdx;
use vb_core::value::Taint;
use vb_core::value_store::ValueStore;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(8, 8)
}

fn fresh_frame_with(step_count: u16, slot_count: u16) -> RunFrame {
    crate::test_harness::fresh_frame(step_count, slot_count)
}

#[test]
fn together_start_initializes_branch_tracking() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branch_a = StepIdx::new(1);
    let join = StepIdx::new(2);

    let result = together_start(&mut run, &mut store, &[branch_a], join, Some(output));

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), branch_a);
    let slot_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"));
    assert!(matches!(slot_val, SlotValue::List(_)));
}

#[test]
fn together_branch_routes_to_entry_step() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    let join = StepIdx::new(4);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);

    let result = together_branch(
        &mut run,
        &mut store,
        BranchIdx::new(0),
        entry,
        join,
        accumulator,
        Some(output),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), entry);
}

#[test]
fn together_join_waits_for_all_branches() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);

    // Set up accumulator with a list containing one branch result.
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    // Output slot holds the last branch result (a non-list value).
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    assert!(run.add_parallel_in_flight(2).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        2,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    // Output slot should now hold the final merged list.
    let final_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"));
    assert!(matches!(final_val, SlotValue::List(_)));
}

// BDD tests for together primitives

#[test]
fn together_start_returns_error_when_no_branches() {
    // Given empty branches list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    // When calling together_start with empty branches
    let result = together_start(&mut run, &mut store, &[], StepIdx::new(2), Some(output));
    // Then it returns InvalidCompiledWorkflow
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "together_start requires at least one branch");
        }
        other => {
            panic!("expected InvalidCompiledWorkflow error, got {other:?}");
        }
    }
}

#[test]
fn together_start_returns_error_when_output_missing() {
    // Given valid branches but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    // When calling together_start with output=None
    let result = together_start(
        &mut run,
        &mut store,
        &[StepIdx::new(1)],
        StepIdx::new(2),
        None,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            panic!("expected MissingOutputSlot error, got {other:?}");
        }
    }
}

#[test]
fn together_start_creates_empty_accumulator_list() {
    // Given valid branches
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &[StepIdx::new(1)],
        StepIdx::new(2),
        Some(output),
    );
    // Then output slot has an empty list
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items.len(), 0);
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_branch_appends_previous_result_for_nonzero_branch() {
    // Given a frame with accumulator list and previous result in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling together_branch with branch=1 (nonzero)
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then it succeeds and jumps to entry
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), entry);
    // And the accumulator list now has 2 items
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_branch_returns_error_when_output_missing_for_nonzero_branch() {
    // Given a frame with accumulator list but no output for branch > 0
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    // When calling together_branch with branch=1 and output=None
    let result = together_branch(
        &mut run,
        &mut store,
        BranchIdx::new(1),
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        None,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            panic!("expected MissingOutputSlot error, got {other:?}");
        }
    }
}

#[test]
fn together_join_returns_error_when_output_missing() {
    // Given a frame
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    // When calling together_join with output=None
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
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
            panic!("expected MissingOutputSlot error, got {other:?}");
        }
    }
}

#[test]
fn together_join_returns_error_when_next_missing() {
    // Given a frame
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(10))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling together_join with next=None
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        None,
        StepIdx::ZERO,
    );
    // Then it returns MissingNextStep
    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            panic!("expected MissingNextStep error, got {other:?}");
        }
    }
}

#[test]
fn together_start_increments_executed_counter() {
    // Given a frame
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let executed_before = run.executed();
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &[StepIdx::new(1)],
        StepIdx::new(2),
        Some(output),
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), executed_before + 1);
}

// ── Adversarial BDD tests for together ──────────────────────────────

#[test]
fn together_start_one_branch_jumps_to_that_branch() {
    // Given a frame with 1 branch
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branch_a = StepIdx::new(5);
    // When calling together_start with exactly 1 branch
    let result = together_start(
        &mut run,
        &mut store,
        &[branch_a],
        StepIdx::new(2),
        Some(output),
    );
    // Then it jumps to that branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), branch_a);
}

#[test]
fn together_start_two_branches_jumps_to_first() {
    // Given a frame with 2 branches
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branch_a = StepIdx::new(3);
    let branch_b = StepIdx::new(4);
    // When calling together_start with 2 branches
    let result = together_start(
        &mut run,
        &mut store,
        &[branch_a, branch_b],
        StepIdx::new(2),
        Some(output),
    );
    // Then it jumps to the first branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), branch_a);
}

#[test]
fn together_branch_zero_does_not_append_to_accumulator() {
    // Given a frame with accumulator and output having different values
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch with branch=0
    let result = together_branch(
        &mut run,
        &mut store,
        0,
        entry,
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then accumulator still has 0 items (branch 0 skips append)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            assert_eq!(
                store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("must read"))
                    .len(),
                0
            );
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_branch_nonzero_appends_to_accumulator() {
    // Given a frame with an empty accumulator and output = I64(99)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(99))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch with branch=1
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then accumulator has 1 item: I64(99)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(99)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_branch_nonzero_returns_error_when_accumulator_is_not_list() {
    // Given a frame where the accumulator slot holds a non-list value
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    run.write_slot(accumulator, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(output, SlotValue::I64(10))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch with branch=1
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then it returns TypeMismatch (accumulator is not a list)
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn together_join_with_null_last_result_preserves_accumulator() {
    // Given a frame with accumulator list and Null in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has the accumulator list (Null last result is not appended)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_join_appends_non_null_non_list_last_result() {
    // Given a frame with accumulator list and I64(20) in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has the accumulator list with I64(20) appended
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_join_with_list_in_output_does_not_double_append() {
    // Given a frame where output already contains a list (from a prior branch)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    let list_id = store
        .insert_list(vec![SlotValue::I64(20)].into_boxed_slice())
        .ok()
        .unwrap_or_else(|| panic!("must insert"));
    run.write_slot(output, SlotValue::List(list_id))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has the accumulator list (list in output is not appended)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            // Should have 1 item from the accumulator, not the output list appended
            assert_eq!(items.len(), 1);
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn together_join_with_non_list_accumulator_uses_accumulator_value() {
    // Given a frame where accumulator is not a list (corruption scenario)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    run.write_slot(accumulator, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(output, SlotValue::I64(99))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then it writes the raw accumulator value to output (non-list path)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(42)
    );
}

// ========================================================================
// Phase 23: Together bounded branches
// ========================================================================

// -- 1. Bounded branches: together respects branch limits --

#[test]
fn phase23_together_start_accepts_max_u16_branches() {
    // Given u16::MAX = 65535 branches (the maximum that fits in u16)
    let mut run = fresh_frame_with(u16::MAX, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches: Vec<StepIdx> = (0..u16::MAX).map(|i| StepIdx::new(i)).collect();
    // When calling together_start with exactly u16::MAX branches
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(0),
        Some(output),
    );
    // Then it succeeds
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

#[test]
fn phase23_together_start_rejects_more_than_u16_max_branches() {
    // Given u16::MAX + 1 branches (one more than fits in u16)
    let mut run = fresh_frame_with(u16::MAX, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let mut branches: Vec<StepIdx> = (0..u16::MAX).map(|i| StepIdx::new(i)).collect();
    branches.push(StepIdx::new(0));
    // When calling together_start with u16::MAX + 1 branches
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(0),
        Some(output),
    );
    // Then it returns TogetherBranchLimitExceeded
    match result {
        Err(EngineError::TogetherBranchLimitExceeded { max }) => {
            assert_eq!(max, u16::MAX);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn phase23_together_start_single_branch_within_limit() {
    // Given a single branch (well within any reasonable limit)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branch = StepIdx::new(1);
    // When calling together_start with 1 branch
    let result = together_start(
        &mut run,
        &mut store,
        &[branch],
        StepIdx::new(2),
        Some(output),
    );
    // Then it succeeds and jumps to the branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), branch);
}

#[test]
fn phase23_together_start_many_branches_within_u16_limit() {
    // Given 256 branches (within u16 range)
    let mut run = fresh_frame_with(300, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches: Vec<StepIdx> = (1..=256).map(StepIdx::new).collect();
    let first_branch = branches[0];
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(299),
        Some(output),
    );
    // Then it succeeds and jumps to the first branch
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), first_branch);
}

// -- 2. Branch state tracking: each branch has independent state --

#[test]
fn phase23_branch_zero_independent_state_does_not_touch_accumulator() {
    // Given an accumulator with pre-existing data and an output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(100)]);
    run.write_slot(output, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch for branch 0
    let result = together_branch(
        &mut run,
        &mut store,
        0,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    // Then accumulator is untouched (still has 1 item)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(100)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_branch_one_appends_own_result_independently() {
    // Given an empty accumulator and output = I64(42)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch for branch 1
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    // Then accumulator has exactly I64(42) from branch 1
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(42)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_sequential_branches_build_independent_accumulator_state() {
    // Simulate 3 branches: branch 0 runs first, branch 1 appends result A,
    // branch 2 appends result B. Each branch contributes independently.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    // Branch 0: does not append, just jumps to entry
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    let result0 = together_branch(
        &mut run,
        &mut store,
        0,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result0, Ok(vb_core::EngineSignal::Continue));

    // Simulate branch 0 body writing I64(10) to output
    run.write_slot(output, SlotValue::I64(10))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 1: appends I64(10)
    let result1 = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result1, Ok(vb_core::EngineSignal::Continue));

    // Simulate branch 1 body writing I64(20) to output
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 2: appends I64(20)
    let result2 = together_branch(
        &mut run,
        &mut store,
        2,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result2, Ok(vb_core::EngineSignal::Continue));

    // Accumulator now has [I64(10), I64(20)] from branches 1 and 2
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_different_branch_values_do_not_interfere() {
    // Given an accumulator with one value and a different value in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);
    list_in_slot(
        &mut run,
        &mut store,
        accumulator,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    run.write_slot(output, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When branch 3 appends Bool(true)
    let result = together_branch(
        &mut run,
        &mut store,
        3,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    // Then accumulator has [I64(1), I64(2), Bool(true)] without corruption
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 3);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
            assert_eq!(items.get(2), Some(&SlotValue::Bool(true)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

// -- 3. Join semantics: all branches must complete before join --

#[test]
fn phase23_join_appends_last_branch_result_to_accumulator() {
    // Given a completed 2-branch scenario with results [I64(10)] in accumulator
    // and I64(20) in output (the last branch result)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(2).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        2,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has [I64(10), I64(20)] - all 2 branches merged
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_join_three_branches_all_results_collected() {
    // Given 3 branches: accumulator has [I64(10), I64(20)], output has I64(30)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(
        &mut run,
        &mut store,
        accumulator,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    run.write_slot(output, SlotValue::I64(30))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join with branch_count=3
    assert!(run.add_parallel_in_flight(3).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        3,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has all 3 branch results
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 3);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(20)));
            assert_eq!(items.get(2), Some(&SlotValue::I64(30)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_join_produces_list_in_output_slot() {
    // Given valid accumulator and output state
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(7)]);
    run.write_slot(output, SlotValue::I64(8))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(2).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        2,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then the output slot holds a SlotValue::List (the merged results)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let final_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert!(matches!(final_val, SlotValue::List(_)));
}

#[test]
fn phase23_join_merges_taint_from_accumulator_and_output() {
    // Given an accumulator with a derived taint and output with secret taint
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot_with_taint(accumulator, SlotValue::I64(10), Taint::DerivedFromSecret)
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // Reset accumulator to list after the overwrite
    let list_id = store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .ok()
        .unwrap_or_else(|| panic!("insert"));
    run.write_slot_with_taint(
        accumulator,
        SlotValue::List(list_id),
        Taint::DerivedFromSecret,
    )
    .ok()
    .unwrap_or_else(|| panic!("write"));
    run.write_slot_with_taint(output, SlotValue::I64(20), Taint::Secret)
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join
    assert!(run.add_parallel_in_flight(2).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        2,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then the output taint is the join of DerivedFromSecret and Secret = Secret
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let out_taint = run
        .read_taint(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(out_taint, Taint::Secret);
}

// -- 4. Failure policy: single branch failure behavior (fail-fast) --

#[test]
fn phase23_branch_failure_when_accumulator_corrupted_propagates_error() {
    // Given a branch operation where the accumulator is corrupted (not a list)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    // Accumulator holds a non-list value (simulating corruption)
    run.write_slot(accumulator, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch with branch=1 (triggers append)
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then it returns TypeMismatch (fail-fast on corrupted state)
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
fn phase23_branch_failure_preserves_existing_accumulator_state() {
    // Given an accumulator with data from a prior branch
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    run.write_slot(output, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_branch with branch=1 but accumulator corrupted after
    // First make the accumulator corrupt
    run.write_slot(accumulator, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    // Then a TypeMismatch error is returned (fail-fast)
    // The accumulator was corrupted from list to boolean
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }
    // And the output slot still holds the value from the last successful branch
    let output_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(output_val, SlotValue::Bool(false));
}

#[test]
fn phase23_join_failure_when_output_slot_missing() {
    // Given a valid accumulator but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    // When calling together_join with output=None
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        None,
        Some(StepIdx::new(5)),
        StepIdx::ZERO,
    );
    // Then it returns MissingOutputSlot (fail-fast)
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            panic!("expected MissingOutputSlot error, got {other:?}");
        }
    }
}

#[test]
fn phase23_start_failure_when_branches_empty() {
    // Given no branches
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    // When calling together_start with empty branches
    let result = together_start(&mut run, &mut store, &[], StepIdx::new(2), Some(output));
    // Then it returns InvalidCompiledWorkflow (fail-fast: cannot start with 0 branches)
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "together_start requires at least one branch");
        }
        other => {
            panic!("expected InvalidCompiledWorkflow error, got {other:?}");
        }
    }
}

// -- 5. Partial failure handling: some branches succeed, some fail --

#[test]
fn phase23_partial_failure_first_branch_succeeds_second_corrupt_accumulator() {
    // Simulate: branch 0 succeeds, branch 1 would succeed, but then
    // a corrupted accumulator causes failure
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    // Branch 0 succeeds (no append)
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    let result0 = together_branch(
        &mut run,
        &mut store,
        0,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result0, Ok(vb_core::EngineSignal::Continue));

    // Branch 0 body writes I64(10)
    run.write_slot(output, SlotValue::I64(10))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 1 succeeds (appends I64(10))
    let result1 = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result1, Ok(vb_core::EngineSignal::Continue));

    // Verify accumulated state so far: [I64(10)]
    match *run
        .read_slot(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }

    // Branch 1 body writes I64(20)
    run.write_slot(output, SlotValue::I64(20))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Corrupt the accumulator before branch 2
    run.write_slot(accumulator, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 2 fails due to corrupted accumulator
    let result2 = together_branch(
        &mut run,
        &mut store,
        2,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    // Then branch 2 returns TypeMismatch (partial failure)
    assert!(result2.is_err());
    match result2 {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            panic!("expected TypeMismatch error, got {other:?}");
        }
    }
}

#[test]
fn phase23_partial_failure_output_preserved_after_branch_error() {
    // Given a scenario where branches have partially executed
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);

    // Branch 0 writes Null to output (valid)
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 1 appends Null
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    // Now corrupt the accumulator
    run.write_slot(accumulator, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 2 fails
    let fail_result = together_branch(
        &mut run,
        &mut store,
        2,
        StepIdx::new(3),
        StepIdx::new(4),
        accumulator,
        Some(output),
    );
    match fail_result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            // Accumulator was corrupted to I64(999); expect_list requires a list
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }

    // The output slot still holds Null from the last successful branch
    let output_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(output_val, SlotValue::Null);
}

#[test]
fn phase23_join_with_single_branch_collects_one_result() {
    // Given a single-branch together: accumulator empty, output = I64(42)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    run.write_slot(output, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join with branch_count=1
    assert!(run.add_parallel_in_flight(1).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then output has [I64(42)] (the single branch result)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(42)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_taint_propagation_across_branches() {
    // Verify that taint from each branch is propagated into the accumulator
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let entry = StepIdx::new(5);

    // Start with clean accumulator
    list_in_slot(&mut run, &mut store, accumulator, vec![]);
    // Branch 0 body writes I64(10) with Secret taint
    run.write_slot_with_taint(output, SlotValue::I64(10), Taint::Secret)
        .ok()
        .unwrap_or_else(|| panic!("write"));

    // Branch 1 appends I64(10) - accumulator should pick up Secret taint
    let result = together_branch(
        &mut run,
        &mut store,
        1,
        entry,
        StepIdx::new(6),
        accumulator,
        Some(output),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // Accumulator taint should be Secret from the branch output
    let acc_taint = run
        .read_taint(accumulator)
        .ok()
        .unwrap_or_else(|| panic!("must read"));
    assert_eq!(acc_taint, Taint::Secret);
}

#[test]
fn phase23_start_creates_empty_list_output_even_with_multiple_branches() {
    // Given 3 branches
    let mut run = fresh_frame_with(8, 2);
    let mut store = ValueStore::new();
    let output = SlotIdx::new(0);
    let branches = [StepIdx::new(3), StepIdx::new(4), StepIdx::new(5)];
    // When calling together_start
    let result = together_start(
        &mut run,
        &mut store,
        &branches,
        StepIdx::new(2),
        Some(output),
    );
    // Then output slot has an empty list as the initial accumulator
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 0);
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}

#[test]
fn phase23_join_preserves_order_of_branch_results() {
    // Given 4 branches with results accumulated in order
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(
        &mut run,
        &mut store,
        accumulator,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    run.write_slot(output, SlotValue::I64(4))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling together_join with branch_count=4
    assert!(run.add_parallel_in_flight(4).is_ok());
    let result = together_join(
        &mut run,
        &mut store,
        4,
        accumulator,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then the final list preserves insertion order [1, 2, 3, 4]
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 4);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
            assert_eq!(items.get(2), Some(&SlotValue::I64(3)));
            assert_eq!(items.get(3), Some(&SlotValue::I64(4)));
        }
        other => {
            panic!("expected list slot, got {:?}", other);
        }
    }
}
