//! ForEach iteration primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{FanoutLimit, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use super::helpers::{empty_list, expect_list, jump_to, jump_to_next, require_output, tail_items};

/// Executes ForEachStart: validates input list, binds first item, sets up
/// iterator state in the output slot as the remaining tail list.
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
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot_with_taint(iter_output, SlotValue::List(tail_id), input_taint)?;
    jump_to(run, body)
}

/// Executes ForEachNext: advances iterator, binds next item or exits.
pub fn for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let list_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let iter_taint = run.read_taint(iterator_slot)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let item_output = require_output(output, run.pc())?;
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each next items checked nonempty",
        })?;
    run.write_slot_with_taint(item_output, first, iter_taint)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot_with_taint(iterator_slot, SlotValue::List(tail_id), iter_taint)?;
    jump_to(run, body)
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
