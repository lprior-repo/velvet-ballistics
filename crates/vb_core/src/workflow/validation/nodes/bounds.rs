#![forbid(unsafe_code)]
//! Primitive index bounds validators for step, slot, expression, and constant
//! indices used throughout the workflow graph.

use crate::errors::CoreError;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};

use crate::workflow::WorkflowError;

/// Validates that a [`SlotIdx`] falls within the declared slot table bound.
pub(crate) fn validate_slot(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::SlotOutOfBounds { slot })
    }
}

/// Validates that a [`StepIdx`] falls within the declared node count.
pub(crate) fn validate_step(step: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if step.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::StepOutOfBounds { step })
    }
}

/// Validates that an [`ExprIdx`] falls within the declared expression count.
pub(crate) fn validate_expr(expr: ExprIdx, expression_count: usize) -> Result<(), WorkflowError> {
    if expr.as_usize() < expression_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds {
            expr,
        }))
    }
}

/// Validates that a [`ConstIdx`] falls within the declared constant-pool count.
pub(crate) fn validate_const(constant: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if constant.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::ConstOutOfBounds { constant })
    }
}

/// Validates that an optional slot is either absent or within bounds.
pub(crate) fn validate_optional_slot(
    slot: Option<SlotIdx>,
    slot_count: u16,
) -> Result<(), WorkflowError> {
    slot.map_or(Ok(()), |value| validate_slot(value, slot_count))
}

/// Validates that an optional step is either absent or within bounds.
pub(crate) fn validate_optional_step(
    step: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    step.map_or(Ok(()), |target| validate_step(target, node_count))
}
