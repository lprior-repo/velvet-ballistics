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

mod phase23;

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
    assert_eq!(run.write_slot(output, SlotValue::I64(20)), Ok(()));

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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
    assert_eq!(run.parallel_in_flight(), 0);
    assert_eq!(store.total_arena_count(), 0);
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
            assert_eq!(other, SlotValue::I64(0));
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
    assert_eq!(run.write_slot(output, SlotValue::I64(20)), Ok(()));
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
            assert_eq!(other, SlotValue::I64(0));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
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
    assert_eq!(run.write_slot(output, SlotValue::I64(10)), Ok(()));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
    assert_eq!(run.parallel_in_flight(), 1);
    assert_eq!(run.read_slot(output).copied(), Ok(SlotValue::I64(10)));
}

#[test]
fn together_join_invalid_next_does_not_mutate_output_or_parallel_count() {
    let mut run = fresh_frame_with(2, 2);
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(7)]);
    assert_eq!(run.write_slot(output, SlotValue::I64(8)), Ok(()));
    assert!(run.add_parallel_in_flight(1).is_ok());

    let result = together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(StepIdx::new(3)),
        StepIdx::ZERO,
    );

    assert_eq!(
        result,
        Err(EngineError::InvalidProgramCounter {
            step: StepIdx::new(3)
        })
    );
    assert_eq!(run.parallel_in_flight(), 1);
    assert_eq!(run.read_slot(output).copied(), Ok(SlotValue::I64(8)));
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
    assert_eq!(run.write_slot(output, SlotValue::I64(42)), Ok(()));
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
            assert_eq!(other, SlotValue::I64(0));
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
    assert_eq!(run.write_slot(output, SlotValue::I64(99)), Ok(()));
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
            assert_eq!(other, SlotValue::I64(0));
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
    assert_eq!(run.write_slot(accumulator, SlotValue::I64(42)), Ok(()));
    assert_eq!(run.write_slot(output, SlotValue::I64(10)), Ok(()));
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
fn together_join_appends_non_null_non_list_last_result() {
    // Given a frame with accumulator list and I64(20) in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    assert_eq!(run.write_slot(output, SlotValue::I64(20)), Ok(()));
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
            assert_eq!(other, SlotValue::I64(0));
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
    assert_eq!(run.write_slot(accumulator, SlotValue::I64(42)), Ok(()));
    assert_eq!(run.write_slot(output, SlotValue::I64(99)), Ok(()));
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

#[test]
fn together_join_with_distinct_null_result_appends_null() {
    // Given a frame with accumulator list and Null in output
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
    assert_eq!(run.write_slot(output, SlotValue::Null), Ok(()));
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
    // The distinct result slot contains a legitimate Null branch value.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(1), Some(&SlotValue::Null));
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn together_join_with_distinct_list_result_appends_nested_list() {
    // A distinct result slot holds the final branch list.
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
    assert_eq!(run.write_slot(output, SlotValue::List(list_id)), Ok(()));
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
    // The final branch list is appended as a nested value.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items, &[SlotValue::I64(10), SlotValue::List(list_id)]);
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn together_join_legacy_aliased_accumulator_does_not_append_itself() -> Result<(), EngineError> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let slot = SlotIdx::new(0);
    let value = SlotValue::F64(vb_core::FiniteF64::new(2.5)?);
    let id = store.insert_list_with_taint(
        Box::from([SlotValue::I64(7), value]),
        Box::from([Taint::Secret, Taint::DerivedFromSecret]),
    )?;
    run.write_slot_with_taint(slot, SlotValue::List(id), Taint::Secret)?;
    run.add_parallel_in_flight(1)?;
    together_join(
        &mut run,
        &mut store,
        1,
        slot,
        Some(slot),
        Some(StepIdx::new(5)),
        StepIdx::ZERO,
    )?;
    assert_eq!(*run.read_slot(slot)?, SlotValue::List(id));
    assert_eq!(store.list(id)?, &[SlotValue::I64(7), value]);
    assert_eq!(
        store.list_item_with_taint(id, 0)?,
        (SlotValue::I64(7), Taint::Secret)
    );
    assert_eq!(
        store.list_item_with_taint(id, 1)?,
        (value, Taint::DerivedFromSecret)
    );
    assert_eq!(run.read_taint(slot)?, Taint::Secret);
    assert_eq!(run.parallel_in_flight(), 0);
    Ok(())
}

#[test]
fn together_join_distinct_slots_with_same_list_handle_still_append() -> Result<(), EngineError> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let id = store.insert_list_with_taint(
        Box::from([SlotValue::I64(7)]),
        Box::from([Taint::DerivedFromSecret]),
    )?;
    run.write_slot_with_taint(accumulator, SlotValue::List(id), Taint::DerivedFromSecret)?;
    run.write_slot_with_taint(output, SlotValue::List(id), Taint::Secret)?;
    run.add_parallel_in_flight(1)?;
    together_join(
        &mut run,
        &mut store,
        1,
        accumulator,
        Some(output),
        Some(StepIdx::new(5)),
        StepIdx::ZERO,
    )?;
    let SlotValue::List(result) = *run.read_slot(output)? else {
        return Err(EngineError::TypeMismatch {
            expected: "list",
            found: "other",
        });
    };
    assert_eq!(
        store.list(result)?,
        &[SlotValue::I64(7), SlotValue::List(id)]
    );
    assert_eq!(
        store.list_item_with_taint(result, 0)?,
        (SlotValue::I64(7), Taint::DerivedFromSecret)
    );
    assert_eq!(
        store.list_item_with_taint(result, 1)?,
        (SlotValue::List(id), Taint::Secret)
    );
    assert_eq!(run.read_taint(output)?, Taint::Secret);
    assert_eq!(run.parallel_in_flight(), 0);
    Ok(())
}
