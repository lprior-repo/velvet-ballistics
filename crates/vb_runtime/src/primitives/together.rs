//! Together parallel-branch primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use super::helpers::{expect_list, jump_to, jump_to_next, require_output};

/// Executes TogetherStart: forks into the first branch.
///
/// In a deterministic synchronous runtime, together branches execute
/// sequentially in declaration order. TogetherStart creates an empty
/// accumulator list in the output slot, then jumps to the first
/// TogetherBranch node.
pub fn together_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branches: &[StepIdx],
    _join: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let count = u16::try_from(branches.len())
        .map_err(|_| EngineError::TogetherBranchLimitExceeded { max: u16::MAX })?;
    if count == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "together_start requires at least one branch",
        });
    }
    let iter_output = require_output(output, run.pc())?;
    let state = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot(iter_output, SlotValue::List(state))?;
    let first_branch =
        branches
            .first()
            .copied()
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "together branches checked nonzero",
            })?;
    jump_to(run, first_branch)
}

/// Executes TogetherBranch: runs one branch body and appends its result.
///
/// For branches after the first, the output slot holds the previous
/// branch body result (a non-list value written by the branch body).
/// This function reads that result from the output slot, reads the
/// accumulator list from the dedicated accumulator slot, appends the
/// result, writes the updated accumulator list back, then jumps to the
/// branch entry.
///
/// For the first branch, no previous result exists so only the jump
/// occurs.
pub fn together_branch(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch: u16,
    entry: StepIdx,
    _join: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    if branch > 0 {
        let branch_output = require_output(output, run.pc())?;
        let previous_result = *run.read_slot(branch_output)?;
        append_to_accumulator(run, store, accumulator, previous_result)?;
    }
    jump_to(run, entry)
}

/// Executes TogetherJoin: waits for all branches and merges results.
///
/// In the synchronous model, all branches have already completed in
/// order. TogetherJoin reads the last branch result from the output
/// slot, appends it to the accumulator list, and writes the final
/// merged list to the output slot.
pub fn together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    _branch_count: u16,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let out = require_output(output, step)?;
    // Read the accumulator list built by together_branch invocations.
    let acc_value = *run.read_slot(accumulator)?;
    let final_list = match acc_value {
        SlotValue::List(id) => {
            // Append the last branch body result if it's not already a list.
            let last_result = *run.read_slot(out)?;
            match last_result {
                SlotValue::List(_) | SlotValue::Null => SlotValue::List(id),
                other => {
                    append_to_accumulator(run, store, accumulator, other)?;
                    *run.read_slot(accumulator)?
                }
            }
        }
        _ => {
            // No accumulator list; use whatever is in the output slot.
            acc_value
        }
    };
    run.write_slot(out, final_list)?;
    jump_to_next(run, next, step)
}

/// Reads the current accumulator list from the accumulator slot,
/// appends the given value, and writes the updated list back.
///
/// The accumulator slot must hold a `SlotValue::List`. A new list is
/// allocated in the store containing all existing elements plus the
/// new value, and the updated list handle is written back to the
/// accumulator slot.
fn append_to_accumulator(
    run: &mut RunFrame,
    store: &mut ValueStore,
    accumulator: SlotIdx,
    value: SlotValue,
) -> Result<(), EngineError> {
    let current = *run.read_slot(accumulator)?;
    let list_id = expect_list(current)?;
    let existing = store.list(list_id)?;
    let mut items = existing.to_vec();
    items.push(value);
    let updated = store.insert_list(items.into_boxed_slice())?;
    run.write_slot(accumulator, SlotValue::List(updated))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::list_in_slot;
    use vb_core::value_store::ValueStore;

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 8)
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
            0,
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
            1,
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
        run.write_slot(output, SlotValue::I64(10))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling together_join with next=None
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
                assert_eq!(other, SlotValue::I64(0));
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
        run.write_slot(accumulator, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        run.write_slot(output, SlotValue::I64(99))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling together_join
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
}
