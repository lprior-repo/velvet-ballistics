use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::CompiledNode;

use super::super::{ReplayError, engine_to_replay_err};
use super::iter::{
    advance_cursor, build_iterator_state, decode_iterator_state, expect_list_value, replay_output,
};
use super::nodes::{advance_to_next, replay_jump};
use super::types::ReplayAction;

/// Replays `ForEachStart`: validates the input list against the fanout limit,
/// binds the first item, and writes cursor-based iterator state.
/// Mirrors `vb_runtime::primitives::for_each::for_each_start`.
#[allow(clippy::too_many_arguments)]
pub(super) fn replay_for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let list_id = expect_list_value(*run.read_slot(input).map_err(engine_to_replay_err)?)?;
    let input_taint = run.read_taint(input).map_err(engine_to_replay_err)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "for_each source list missing during replay",
    })?;
    let item_count = items.len();
    let item_limit = usize::try_from(limit).map_err(|_| ReplayError::Internal {
        reason: "for_each limit overflow during replay",
    })?;
    if item_count > item_limit {
        return Err(ReplayError::Internal {
            reason: "for_each item count exceeds limit during replay",
        });
    }
    let iter_output = replay_output(node, "ForEachStart node missing output slot")?;
    if item_count == 0 {
        let empty = store
            .insert_list(Box::from([]))
            .map_err(|_| ReplayError::Internal {
                reason: "insert for_each empty list failed during replay",
            })?;
        run.write_slot_with_taint(iter_output, SlotValue::List(empty), input_taint)
            .map_err(engine_to_replay_err)?;
        return replay_jump(run, done);
    }
    let first = items.first().copied().ok_or(ReplayError::Internal {
        reason: "for_each items checked nonempty during replay",
    })?;
    run.write_slot_with_taint(item_slot, first, input_taint)
        .map_err(engine_to_replay_err)?;
    let state = build_iterator_state(list_id, 1)?;
    let state_id = store
        .insert_list(state)
        .map_err(|_| ReplayError::Internal {
            reason: "insert for_each iterator state failed during replay",
        })?;
    run.write_slot_with_taint(iter_output, SlotValue::List(state_id), input_taint)
        .map_err(engine_to_replay_err)?;
    replay_jump(run, body)
}

/// Replays `ForEachNext`: advances the cursor, binds the next item, or exits.
/// Mirrors `for_each_next_with_policy`; the legacy body-entry re-arm is a
/// no-op in replay because replay never marks step states.
pub(super) fn replay_for_each_next(
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
        reason: "for_each iterator state missing during replay",
    })?;
    if state_items.is_empty() {
        return replay_jump(run, done);
    }
    let (source_id, cursor) = decode_iterator_state(state_items)?;
    let source = store.list(source_id).map_err(|_| ReplayError::Internal {
        reason: "for_each source list missing during replay",
    })?;
    if cursor >= source.len() {
        return replay_jump(run, done);
    }
    let item_output = replay_output(node, "ForEachNext node missing output slot")?;
    let item = source.get(cursor).copied().ok_or(ReplayError::Internal {
        reason: "for_each cursor bounds-checked above during replay",
    })?;
    run.write_slot_with_taint(item_output, item, iter_taint)
        .map_err(engine_to_replay_err)?;
    let next_cursor = advance_cursor(cursor)?;
    let next_state = build_iterator_state(source_id, next_cursor)?;
    let next_state_id = store
        .insert_list(next_state)
        .map_err(|_| ReplayError::Internal {
            reason: "insert for_each iterator state failed during replay",
        })?;
    run.write_slot_with_taint(iterator_slot, SlotValue::List(next_state_id), iter_taint)
        .map_err(engine_to_replay_err)?;
    replay_jump(run, body)
}

/// Replays `ForEachJoin`: materializes ordered loop results to the output.
/// Mirrors `vb_runtime::primitives::for_each::for_each_join`.
pub(super) fn replay_for_each_join(
    run: &mut RunFrame,
    node: &CompiledNode,
    materialized: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(materialized).map_err(engine_to_replay_err)?;
    let taint = run.read_taint(materialized).map_err(engine_to_replay_err)?;
    expect_list_value(value)?;
    let output = replay_output(node, "ForEachJoin node missing output slot")?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    advance_to_next(run, node).map(ReplayAction::Continue)
}
