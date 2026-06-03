#![forbid(unsafe_code)]
//! Reduce accumulation primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use super::helpers::{
    expect_list, jump_to, jump_to_body, jump_to_next, require_output, tail_items,
};

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
    let init_value = plan
        .constant(initial)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: initial })?
        .to_slot_value()?;
    run.write_slot(accumulator, init_value)?;
    let list_id = expect_list(*run.read_slot(input)?)?;
    let input_taint = run.read_taint(input)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let iter_output = require_output(output, run.pc())?;
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce items checked nonempty",
        })?;
    run.write_slot_with_taint(input, first, input_taint)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot_with_taint(iter_output, SlotValue::List(tail_id), input_taint)?;
    jump_to(run, body)
}

/// Executes ReduceNext: advances iterator, binds next item to
/// iterator slot, or exits when all items consumed.
pub fn reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    _accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let remaining_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let iter_taint = run.read_taint(iterator_slot)?;
    let remaining = store.list(remaining_id)?;
    if remaining.is_empty() {
        return jump_to(run, done);
    }
    let item_output = require_output(output, run.pc())?;
    let first = remaining
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce next items checked nonempty",
        })?;
    run.write_slot_with_taint(item_output, first, iter_taint)?;
    let tail = tail_items(remaining)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot_with_taint(iterator_slot, SlotValue::List(tail_id), iter_taint)?;
    jump_to_body(run, body)
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
