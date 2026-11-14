#![forbid(unsafe_code)]
//! Together parallel-branch primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{BranchCount, BranchIdx, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, join_taint};
use vb_core::value_store::ValueStore;
use vb_core::{ListId, Taint};

use super::helpers::{expect_list, jump_to, require_next_step, require_output};

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
    let (count, iter_output, first_branch) = validate_together_start(run, branches, output)?;
    let state = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot(iter_output, SlotValue::List(state))?;
    run.add_parallel_in_flight(count)?;
    jump_to(run, first_branch)
}

fn validate_together_start(
    run: &RunFrame,
    branches: &[StepIdx],
    output: Option<SlotIdx>,
) -> Result<(u16, SlotIdx, StepIdx), EngineError> {
    let count = validate_parallel_capacity(run, branches)?;
    let (iter_output, first_branch) = validate_together_targets(run, branches, output)?;
    Ok((count, iter_output, first_branch))
}

fn validate_parallel_capacity(run: &RunFrame, branches: &[StepIdx]) -> Result<u16, EngineError> {
    let count = u16::try_from(branches.len())
        .map_err(|_| EngineError::TogetherBranchLimitExceeded { max: u16::MAX })?;
    if count == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "together_start requires at least one branch",
        });
    }
    let requested = run.parallel_in_flight().checked_add(count).ok_or(
        EngineError::InternalInvariantViolation {
            reason: "parallel_in_flight overflow".into(),
        },
    )?;
    let max = run.max_parallel_in_flight();
    if requested > max {
        return Err(EngineError::ParallelLimitExceeded { limit: max });
    }
    Ok(count)
}

fn validate_together_targets(
    run: &RunFrame,
    branches: &[StepIdx],
    output: Option<SlotIdx>,
) -> Result<(SlotIdx, StepIdx), EngineError> {
    let iter_output = require_output(output, run.pc())?;
    if iter_output.get() >= run.slot_count() {
        return Err(EngineError::SlotOutOfBounds { slot: iter_output });
    }
    let first_branch =
        branches
            .first()
            .copied()
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "together branches checked nonzero".into(),
            })?;
    if first_branch.get() >= run.step_count() {
        return Err(EngineError::InvalidProgramCounter { step: first_branch });
    }
    if run.executed() == u64::MAX {
        return Err(EngineError::StepCounterOverflow);
    }
    Ok((iter_output, first_branch))
}

/// Executes TogetherBranch: runs one branch body and appends its result.
///
/// For branches after the first, the output slot holds the previous
/// branch body result written by the branch body.
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
    let out = require_output(output, step)?;
    let target = require_next_step(run, next, step)?;
    let count = branch_count.get();
    if run.parallel_in_flight() < count {
        return Err(EngineError::InternalInvariantViolation {
            reason: "parallel_in_flight underflow".into(),
        });
    }
    let (final_list, combined_taint) = collect_join_result(run, store, accumulator, out)?;
    run.write_slot_with_taint(out, final_list, combined_taint)?;
    run.sub_parallel_in_flight(count)?;
    jump_to(run, target)
}

fn collect_join_result(
    run: &mut RunFrame,
    store: &mut ValueStore,
    accumulator: SlotIdx,
    output: SlotIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    let acc_value = *run.read_slot(accumulator)?;
    let final_list = if matches!(acc_value, SlotValue::List(_)) && output != accumulator {
        let last_result = *run.read_slot(output)?;
        append_to_accumulator(run, store, accumulator, last_result, output)?;
        *run.read_slot(accumulator)?
    } else {
        acc_value
    };
    let acc_taint = run.read_taint(accumulator)?;
    let out_taint = run.read_taint(output)?;
    Ok((final_list, join_taint(acc_taint, out_taint)))
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
    let updated = append_list_value(store, list_id, value, branch_taint)?;
    run.write_slot_with_taint(accumulator, SlotValue::List(updated), combined_taint)?;
    Ok(())
}

fn append_list_value(
    store: &mut ValueStore,
    list_id: ListId,
    value: SlotValue,
    branch_taint: Taint,
) -> Result<ListId, EngineError> {
    let existing = store.list(list_id)?;
    let mut items = existing.to_vec();
    let mut taints = (0..existing.len())
        .map(|index| {
            let index = u32::try_from(index).map_err(|_| EngineError::AllocationFailed)?;
            store
                .list_item_with_taint(list_id, index)
                .map(|(_, taint)| taint)
        })
        .collect::<Result<Vec<_>, _>>()?;
    items.push(value);
    taints.push(branch_taint);
    store.insert_list_with_taint(items.into_boxed_slice(), taints.into_boxed_slice())
}

#[cfg(test)]
#[path = "../together_tests/mod.rs"]
mod tests;
