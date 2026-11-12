use crate::frame::RunFrame;
use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledWorkflow};

use super::super::{ReplayError, engine_to_replay_err};
use super::iter::{
    advance_cursor, build_iterator_state, decode_iterator_state, expect_list_value,
    preflight_writes, replay_output,
};
use super::nodes::{advance_to_next, replay_jump};
use super::types::ReplayAction;

/// Replays `ReduceStart`: initializes the accumulator from the constant pool,
/// binds the first item, and writes the remaining tail as iterator state.
/// Mirrors `vb_runtime::primitives::reduce::reduce_start`.
#[allow(clippy::too_many_arguments)]
pub(super) fn replay_reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let list_id = expect_list_value(*run.read_slot(input).map_err(engine_to_replay_err)?)?;
    let input_taint = run.read_taint(input).map_err(engine_to_replay_err)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "reduce source list missing during replay",
    })?;
    let limit = usize::try_from(limit).map_err(|_| ReplayError::Internal {
        reason: "reduce limit overflow during replay",
    })?;
    if items.len() > limit {
        return Err(ReplayError::Internal {
            reason: "reduce item count exceeds limit during replay",
        });
    }
    let constant = plan
        .constant(initial)
        .copied()
        .ok_or(ReplayError::Internal {
            reason: "reduce initial constant out of bounds during replay",
        })?;
    let init_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "reduce initial constant to slot value failed during replay",
        })?;
    if items.is_empty() {
        preflight_writes(run, &[accumulator], done)?;
        run.write_slot(accumulator, init_value)
            .map_err(engine_to_replay_err)?;
        return replay_jump(run, done);
    }
    let iter_output = replay_output(node, "ReduceStart node missing output slot")?;
    let first = items.first().copied().ok_or(ReplayError::Internal {
        reason: "reduce items checked nonempty during replay",
    })?;
    let item_taint = crate::value::join_taint(
        input_taint,
        store
            .list_item_with_taint(list_id, 0)
            .map_err(engine_to_replay_err)?
            .1,
    );
    preflight_writes(run, &[accumulator, input, iter_output], body)?;
    let state = build_iterator_state(list_id, 1)?;
    let state_id = store
        .insert_list(state)
        .map_err(|_| ReplayError::Internal {
            reason: "insert reduce iterator state failed during replay",
        })?;
    run.write_slot(accumulator, init_value)
        .map_err(engine_to_replay_err)?;
    run.write_slot_with_taint(input, first, item_taint)
        .map_err(engine_to_replay_err)?;
    run.write_slot_with_taint(iter_output, SlotValue::List(state_id), input_taint)
        .map_err(engine_to_replay_err)?;
    replay_jump(run, body)
}

/// Replays `ReduceNext`: advances the iterator, binds the next item, or
/// exits. Mirrors `reduce_next_with_policy`; the legacy body-entry re-arm is
/// a no-op in replay because replay never marks step states.
pub(super) fn replay_reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let state_id = expect_list_value(*run.read_slot(iterator_slot).map_err(engine_to_replay_err)?)?;
    let iter_taint = run
        .read_taint(iterator_slot)
        .map_err(engine_to_replay_err)?;
    let state_items = store.list(state_id).map_err(|_| ReplayError::Internal {
        reason: "reduce iterator state missing during replay",
    })?;
    if state_items.is_empty() {
        return replay_jump(run, done);
    }
    let (source_id, cursor) = decode_iterator_state(state_items)?;
    let source = store.list(source_id).map_err(|_| ReplayError::Internal {
        reason: "reduce source list missing during replay",
    })?;
    if cursor >= source.len() {
        return replay_jump(run, done);
    }
    let item_output = replay_output(node, "ReduceNext node missing output slot")?;
    let item = source.get(cursor).copied().ok_or(ReplayError::Internal {
        reason: "reduce cursor bounds-checked above during replay",
    })?;
    let item_taint = crate::value::join_taint(
        iter_taint,
        store
            .list_item_with_taint(
                source_id,
                u32::try_from(cursor).map_err(|_| ReplayError::Internal {
                    reason: "iterator cursor exceeds u32",
                })?,
            )
            .map_err(engine_to_replay_err)?
            .1,
    );
    preflight_writes(run, &[item_output, iterator_slot], body)?;
    let next_cursor = advance_cursor(cursor)?;
    let next_state = build_iterator_state(source_id, next_cursor)?;
    let next_state_id = store
        .insert_list(next_state)
        .map_err(|_| ReplayError::Internal {
            reason: "insert reduce iterator state failed during replay",
        })?;
    run.write_slot_with_taint(item_output, item, item_taint)
        .map_err(engine_to_replay_err)?;
    run.write_slot_with_taint(iterator_slot, SlotValue::List(next_state_id), iter_taint)
        .map_err(engine_to_replay_err)?;
    replay_jump(run, body)
}

/// Replays `ReduceFinish`: writes the final accumulator to the output.
/// Mirrors `vb_runtime::primitives::reduce::reduce_finish`.
pub(super) fn replay_reduce_finish(
    run: &mut RunFrame,
    node: &CompiledNode,
    accumulator: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(accumulator).map_err(engine_to_replay_err)?;
    let taint = run.read_taint(accumulator).map_err(engine_to_replay_err)?;
    let output = replay_output(node, "ReduceFinish node missing output slot")?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    advance_to_next(run, node).map(ReplayAction::Continue)
}
