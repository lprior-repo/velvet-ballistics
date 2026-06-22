#![forbid(unsafe_code)]
//! Together parallel-branch primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{BranchCount, BranchIdx, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, join_taint};
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
    let current = run.parallel_in_flight();
    let max = run.max_parallel_in_flight();
    // RP-003: use `checked_add` so a `current + count` overflow cannot
    // pass the guard as if the resulting sum fit within `max`. The
    // pre-fix `saturating_add` clamped the sum to `u16::MAX`, which
    // made the `> max` comparison trivially false when `max ==
    // u16::MAX` and the caller then surfaced as
    // `InternalInvariantViolation` from `add_parallel_in_flight`'s
    // own `checked_add`. Treat arithmetic overflow as the same
    // limit-exceeded failure mode as an over-limit increment.
    let next = current
        .checked_add(count)
        .ok_or(EngineError::ParallelLimitExceeded { limit: max })?;
    if next > max {
        return Err(EngineError::ParallelLimitExceeded { limit: max });
    }
    run.add_parallel_in_flight(count)?;
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
    branch: impl Into<BranchIdx>,
    entry: StepIdx,
    _join: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let branch = branch.into();
    if !branch.is_first() {
        let branch_output = require_output(output, run.pc())?;
        let previous_result = *run.read_slot(branch_output)?;
        append_to_accumulator(run, store, accumulator, previous_result, branch_output)?;
    }
    jump_to(run, entry)
}

/// Executes TogetherJoin: waits for all branches and merges results.
pub fn together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch_count: impl Into<BranchCount>,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let branch_count = branch_count.into();
    run.sub_parallel_in_flight(branch_count.get())?;
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
                    append_to_accumulator(run, store, accumulator, other, out)?;
                    *run.read_slot(accumulator)?
                }
            }
        }
        _ => {
            // No accumulator list; use whatever is in the output slot.
            acc_value
        }
    };
    let acc_taint = run.read_taint(accumulator)?;
    let out_taint = run.read_taint(out)?;
    let combined_taint = join_taint(acc_taint, out_taint);
    run.write_slot_with_taint(out, final_list, combined_taint)?;
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
    branch_output: SlotIdx,
) -> Result<(), EngineError> {
    let current = *run.read_slot(accumulator)?;
    let acc_taint = run.read_taint(accumulator)?;
    let branch_taint = run.read_taint(branch_output)?;
    let combined_taint = join_taint(acc_taint, branch_taint);
    let list_id = expect_list(current)?;
    let existing = store.list(list_id)?;
    let mut items = existing.to_vec();
    items.push(value);
    let updated = store.insert_list(items.into_boxed_slice())?;
    run.write_slot_with_taint(accumulator, SlotValue::List(updated), combined_taint)?;
    Ok(())
}

#[cfg(test)]
#[path = "../together_tests.rs"]
mod tests;
