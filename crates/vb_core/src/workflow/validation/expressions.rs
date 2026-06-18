#![forbid(unsafe_code)]
//! Expression bytecode program validation.
//!
//! Validates that each expression program has consistent stack metadata and
//! that accessor references are within bounds.

use crate::ids::AccessorIdx;
use crate::workflow::{ExprOp, ExprProgram, WorkflowError};

/// Validates a single expression program by reconstructing it through
/// [`ExprProgram::try_from_parts`] and checking accessor references.
fn validate_expression(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
    validate_expression_accessors(expression, accessor_count)
}

/// Checks that all [`ExprOp::LoadAccessor`] references fall within bounds.
fn validate_expression_accessors(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for op in expression.ops.as_ref() {
        if let ExprOp::LoadAccessor(accessor) = op {
            validate_accessor(*accessor, accessor_count)?;
        }
    }
    Ok(())
}

/// Validates that an [`AccessorIdx`] falls within the declared accessor count.
fn validate_accessor(accessor: AccessorIdx, accessor_count: usize) -> Result<(), WorkflowError> {
    if accessor.as_usize() < accessor_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            crate::errors::CoreError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            },
        ))
    }
}

/// Validates all expressions and their accessor references.
pub(crate) fn validate_expressions(
    expressions: &[ExprProgram],
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for expression in expressions {
        validate_expression(expression, accessor_count)?;
    }
    Ok(())
}
