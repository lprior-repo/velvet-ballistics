#![forbid(unsafe_code)]
//! ForEach iteration primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{FanoutLimit, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use super::helpers::{
    build_iterator_state, decode_iterator_state, empty_list, expect_list, jump_to, jump_to_body,
    jump_to_next, require_output,
};

/// Executes ForEachStart: validates input list, binds first item, and writes
/// the cursor-based iterator state to the output slot.
#[allow(clippy::too_many_arguments)]
pub fn for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: impl Into<FanoutLimit>,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let limit = limit.into();
    let list_id = expect_list(*run.read_slot(input)?)?;
    let input_taint = run.read_taint(input)?;
    let items = store.list(list_id)?;
    let item_count = items.len();
    let limit_count = limit.as_usize();
    if item_count > limit_count {
        return Err(EngineError::IterationLimitExceeded {
            resource: "for_each_limit",
        });
    }
    let iter_output = require_output(output, run.pc())?;
    if item_count == 0 {
        run.write_slot_with_taint(
            iter_output,
            SlotValue::List(store.insert_list(empty_list())?),
            input_taint,
        )?;
        return jump_to(run, done);
    }
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each item_count checked nonzero",
        })?;
    run.write_slot_with_taint(item_slot, first, input_taint)?;
    let state = build_iterator_state(list_id, 1)?;
    let state_id = store.insert_list(state)?;
    run.write_slot_with_taint(iter_output, SlotValue::List(state_id), input_taint)?;
    jump_to(run, body)
}

/// Executes ForEachNext: advances the cursor, binds the next item, or exits.
pub fn for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let state_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let iter_taint = run.read_taint(iterator_slot)?;
    let state_items = store.list(state_id)?;
    if state_items.is_empty() {
        return jump_to(run, done);
    }
    let (source_id, cursor) = decode_iterator_state(state_items)?;
    let source = store.list(source_id)?;
    if cursor >= source.len() {
        return jump_to(run, done);
    }
    let item_output = require_output(output, run.pc())?;
    let item = source
        .get(cursor)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each cursor bounds-checked above",
        })?;
    run.write_slot_with_taint(item_output, item, iter_taint)?;
    let next_cursor = cursor
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each cursor overflow",
        })?;
    let next_state = build_iterator_state(source_id, next_cursor)?;
    let next_state_id = store.insert_list(next_state)?;
    run.write_slot_with_taint(iterator_slot, SlotValue::List(next_state_id), iter_taint)?;
    jump_to_body(run, body)
}

/// Executes ForEachJoin: materializes ordered loop results to the output slot.
pub fn for_each_join(
    run: &mut RunFrame,
    materialized: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let output_slot = require_output(output, step)?;
    let value = *run.read_slot(materialized)?;
    let taint = run.read_taint(materialized)?;
    expect_list(value)?;
    run.write_slot_with_taint(output_slot, value, taint)?;
    jump_to_next(run, next, step)
}

#[cfg(test)]
#[path = "../for_each_tests.rs"]
mod tests;
