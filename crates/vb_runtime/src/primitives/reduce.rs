//! Reduce accumulation primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

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
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let iter_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce items checked nonempty",
        })?;
    run.write_slot(iter_output, first)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iter_output, SlotValue::List(tail_id))?;
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
    let remaining = store.list(remaining_id)?;
    if remaining.is_empty() {
        return jump_to(run, done);
    }
    let item_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let first = remaining
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce next items checked nonempty",
        })?;
    run.write_slot(item_output, first)?;
    let tail = tail_items(remaining)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iterator_slot, SlotValue::List(tail_id))?;
    jump_to(run, body)
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
    let out = output.ok_or(EngineError::MissingOutputSlot { step })?;
    run.write_slot(out, value)?;
    jump_to_next(run, next, step)
}

fn expect_list(value: SlotValue) -> Result<vb_core::ids::ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    if items.len() <= 1 {
        return Ok(Vec::<SlotValue>::new().into_boxed_slice());
    }
    Ok(items
        .get(1..)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "tail_items start index checked",
        })?
        .to_vec()
        .into_boxed_slice())
}

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target);
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}
