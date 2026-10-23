#![forbid(unsafe_code)]
//! Reduce accumulation primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, ListId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value::Taint;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use super::helpers::{
    build_iterator_state, decode_iterator_state, expect_list, jump_to, jump_to_next, require_output,
};

fn get_initial_value(plan: &CompiledWorkflow, initial: ConstIdx) -> Result<SlotValue, EngineError> {
    plan.constant(initial)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: initial })?
        .to_slot_value()
}

fn get_first_item(items: &[SlotValue]) -> Result<SlotValue, EngineError> {
    items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce items checked nonempty",
        })
}

fn write_iterator_state(
    run: &mut RunFrame,
    store: &mut ValueStore,
    slot: SlotIdx,
    list_id: ListId,
    cursor: usize,
    taint: Taint,
) -> Result<(), EngineError> {
    let state = build_iterator_state(list_id, cursor)?;
    let state_id = store.insert_list(state)?;
    run.write_slot_with_taint(slot, SlotValue::List(state_id), taint)
}

fn get_item_at(source: &[SlotValue], cursor: usize) -> Result<SlotValue, EngineError> {
    source
        .get(cursor)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce cursor bounds-checked above",
        })
}

fn advance_cursor(cursor: usize) -> Result<usize, EngineError> {
    cursor
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce cursor overflow",
        })
}

/// Executes ReduceStart: initializes accumulator from constant pool,
/// reads input list, binds first item, writes remaining tail to
/// iterator slot.
#[allow(clippy::too_many_arguments)]
pub fn reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let init_value = get_initial_value(plan, initial)?;
    run.write_slot(accumulator, init_value)?;
    let list_id = expect_list(*run.read_slot(input)?)?;
    let input_taint = run.read_taint(input)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let iter_output = require_output(output, run.pc())?;
    let first = get_first_item(items)?;
    run.write_slot_with_taint(input, first, input_taint)?;
    write_iterator_state(run, store, iter_output, list_id, 1, input_taint)?;
    jump_to(run, body)
}

/// Executes ReduceNext: advances iterator, binds next item to
/// iterator slot, or exits when all items consumed.
fn reduce_next_with_policy<P: super::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    _accumulator: SlotIdx,
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
    let item = get_item_at(source, cursor)?;
    run.write_slot_with_taint(item_output, item, iter_taint)?;
    let next_cursor = advance_cursor(cursor)?;
    write_iterator_state(
        run,
        store,
        iterator_slot,
        source_id,
        next_cursor,
        iter_taint,
    )?;
    P::jump(run, body)
}

/// Legacy entry-only state reset; this API does not validate a complete region.
pub fn reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    actual_accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    reduce_next_with_policy::<super::body_entry::LegacyEntry>(
        run,
        store,
        iterator_slot,
        actual_accumulator,
        body,
        done,
        output,
    )
}

/// Internal deferred-reset variant for owned driver preflight.
pub(crate) fn reduce_next_deferred(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    actual_accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    reduce_next_with_policy::<super::body_entry::DeferredOwned>(
        run,
        store,
        iterator_slot,
        actual_accumulator,
        body,
        done,
        output,
    )
}

/// Executes ReduceFinish: writes the final accumulator to output.
pub fn reduce_finish(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let value = *run.read_slot(accumulator)?;
    let taint = run.read_taint(accumulator)?;
    let out = require_output(output, step)?;
    run.write_slot_with_taint(out, value, taint)?;
    jump_to_next(run, next, step)
}

#[cfg(test)]
#[path = "reduce/tests.rs"]
mod tests;
