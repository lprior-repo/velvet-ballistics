use crate::frame::RunFrame;
use crate::ids::{BranchIdx, SlotIdx, StepIdx};
use crate::value::{SlotValue, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::CompiledNode;

use super::super::{ReplayError, engine_to_replay_err};
use super::iter::{expect_list_value, preflight_writes, replay_next, replay_output};
use super::nodes::replay_jump;
use super::types::ReplayAction;

/// Replays `TogetherStart`: creates the accumulator list and forks into the
/// first branch. Mirrors `vb_runtime::primitives::together::together_start`.
pub(super) fn replay_together_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    branches: &[StepIdx],
) -> Result<ReplayAction, ReplayError> {
    let count = u16::try_from(branches.len()).map_err(|_| ReplayError::Internal {
        reason: "together branch count exceeds u16 during replay",
    })?;
    if count == 0 {
        return Err(ReplayError::Internal {
            reason: "together_start requires at least one branch during replay",
        });
    }
    let current = run.parallel_in_flight();
    let max = run.max_parallel_in_flight();
    let requested = current.checked_add(count).ok_or(ReplayError::Internal {
        reason: "together parallel counter overflow during replay",
    })?;
    if requested > max {
        return Err(ReplayError::Internal {
            reason: "together parallel limit exceeded during replay",
        });
    }
    let output = replay_output(node, "TogetherStart node missing output slot")?;
    let first = branches.first().copied().ok_or(ReplayError::Internal {
        reason: "together branches checked nonempty during replay",
    })?;
    preflight_writes(run, &[output], first)?;
    let state = store
        .insert_list(Box::from([]))
        .map_err(|_| ReplayError::Internal {
            reason: "insert together accumulator failed during replay",
        })?;
    run.write_slot(output, SlotValue::List(state))
        .map_err(engine_to_replay_err)?;
    run.add_parallel_in_flight(count)
        .map_err(engine_to_replay_err)?;
    replay_jump(run, first)
}

/// Replays `TogetherBranch`: appends the previous branch result (except for
/// the first branch) then jumps to the branch entry.
/// Mirrors `vb_runtime::primitives::together::together_branch`.
pub(super) fn replay_together_branch(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    branch: u16,
    entry: StepIdx,
    accumulator: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    if !BranchIdx::new(branch).is_first() {
        let branch_output = replay_output(node, "TogetherBranch node missing output slot")?;
        let previous = *run.read_slot(branch_output).map_err(engine_to_replay_err)?;
        append_to_accumulator(run, store, accumulator, previous, branch_output)?;
    }
    replay_jump(run, entry)
}

/// Replays `TogetherJoin`: merges branch results and releases the parallel
/// counter. Mirrors `vb_runtime::primitives::together::together_join`.
pub(super) fn replay_together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    branch_count: u16,
    accumulator: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let output = replay_output(node, "TogetherJoin node missing output slot")?;
    let target = replay_next(node, "TogetherJoin node missing next step")?;
    if run.parallel_in_flight() < branch_count {
        return Err(ReplayError::Internal {
            reason: "together parallel counter underflow during replay",
        });
    }
    let acc_value = *run.read_slot(accumulator).map_err(engine_to_replay_err)?;
    let final_list = if matches!(acc_value, SlotValue::List(_)) && output != accumulator {
        let last = *run.read_slot(output).map_err(engine_to_replay_err)?;
        append_to_accumulator(run, store, accumulator, last, output)?;
        *run.read_slot(accumulator).map_err(engine_to_replay_err)?
    } else {
        acc_value
    };
    let acc_taint = run.read_taint(accumulator).map_err(engine_to_replay_err)?;
    let out_taint = run.read_taint(output).map_err(engine_to_replay_err)?;
    run.write_slot_with_taint(output, final_list, join_taint(acc_taint, out_taint))
        .map_err(engine_to_replay_err)?;
    run.sub_parallel_in_flight(branch_count)
        .map_err(|_| ReplayError::Internal {
            reason: "together parallel counter underflow during replay",
        })?;
    replay_jump(run, target)
}

fn append_to_accumulator(
    run: &mut RunFrame,
    store: &mut ValueStore,
    accumulator: SlotIdx,
    value: SlotValue,
    branch_output: SlotIdx,
) -> Result<(), ReplayError> {
    let current = *run.read_slot(accumulator).map_err(engine_to_replay_err)?;
    let acc_taint = run.read_taint(accumulator).map_err(engine_to_replay_err)?;
    let branch_taint = run
        .read_taint(branch_output)
        .map_err(engine_to_replay_err)?;
    let list_id = expect_list_value(current)?;
    let existing = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "together accumulator list missing during replay",
    })?;
    let mut items = existing.to_vec();
    let mut taints = (0..existing.len())
        .map(|index| {
            let index = u32::try_from(index).map_err(|_| ReplayError::Internal {
                reason: "together item index exceeds u32 during replay",
            })?;
            store
                .list_item_with_taint(list_id, index)
                .map(|(_, taint)| taint)
                .map_err(engine_to_replay_err)
        })
        .collect::<Result<Vec<_>, _>>()?;
    items.push(value);
    taints.push(branch_taint);
    let updated = store
        .insert_list_with_taint(items.into_boxed_slice(), taints.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert together accumulator failed during replay",
        })?;
    run.write_slot_with_taint(
        accumulator,
        SlotValue::List(updated),
        join_taint(acc_taint, branch_taint),
    )
    .map_err(engine_to_replay_err)
}
