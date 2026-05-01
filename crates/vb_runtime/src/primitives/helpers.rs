//! Shared helper functions for primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

pub(crate) fn expect_list(value: SlotValue) -> Result<ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

pub(crate) fn empty_list() -> Box<[SlotValue]> {
    Vec::<SlotValue>::new().into_boxed_slice()
}

pub(crate) fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    if items.len() <= 1 {
        return Ok(empty_list());
    }
    let tail_len = items
        .len()
        .checked_sub(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "tail_items length checked nonempty",
        })?;
    let mut tail = Vec::with_capacity(tail_len);
    let mut index = 1usize;
    while index < items.len() {
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "tail_items index checked",
            })?;
        tail.push(value);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "tail_items index overflow",
            })?;
    }
    Ok(tail.into_boxed_slice())
}

pub(crate) fn jump_to(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

pub(crate) fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}

pub(crate) fn require_output(
    output: Option<SlotIdx>,
    step: StepIdx,
) -> Result<SlotIdx, EngineError> {
    output.ok_or(EngineError::MissingOutputSlot { step })
}
