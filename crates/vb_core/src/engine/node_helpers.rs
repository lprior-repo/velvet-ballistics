#![forbid(unsafe_code)]
//! Node execution helper functions.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use crate::workflow::CompiledWorkflow;
use crate::EngineSignal;

#[inline]
pub(super) fn set_const(
    plan: &CompiledWorkflow,
    run: &mut crate::frame::RunFrame,
    node: &crate::workflow::CompiledNode,
    value: ConstIdx,
) -> Result<EngineSignal, EngineError> {
    let constant = plan
        .constant(value)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: value })?;
    let slot_value = constant.to_slot_value()?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, slot_value)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
pub(super) fn copy_slot(
    run: &mut crate::frame::RunFrame,
    node: &crate::workflow::CompiledNode,
    source: SlotIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(source)?;
    let taint = run.read_taint(source)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, value, taint)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
pub(super) fn jump_to_next(
    run: &mut crate::frame::RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let next = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, next)
}

#[inline]
pub(super) fn jump_to(
    run: &mut crate::frame::RunFrame,
    target: StepIdx,
) -> Result<EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(EngineSignal::Continue)
}

#[inline]
pub(super) fn finish_run(
    run: &mut crate::frame::RunFrame,
    result: SlotIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(result)?;
    let taint = run.read_taint(result)?;
    run.increment_executed()?;
    Ok(EngineSignal::Finished(value, taint))
}


