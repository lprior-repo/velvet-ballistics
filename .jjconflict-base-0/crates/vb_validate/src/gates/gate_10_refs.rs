#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ConstIdx};
use vb_core::workflow::{ExprOp, WorkflowParts};

pub(crate) fn validate_expression_references(
    parts: &WorkflowParts,
    const_count: usize,
    accessor_count: usize,
) -> ValidationResult<()> {
    parts
        .expressions
        .iter()
        .enumerate()
        .try_for_each(|(expr_index, expr)| {
            expr.ops.iter().try_for_each(|op| match op {
                ExprOp::LoadConst(value) => {
                    validate_load_const_reference(expr_index, *value, const_count)
                }
                ExprOp::LoadAccessor(accessor) => {
                    validate_load_accessor_reference(expr_index, *accessor, accessor_count)
                }
                _ => Ok(()),
            })
        })
}

fn validate_load_const_reference(
    expr_index: usize,
    value: ConstIdx,
    const_count: usize,
) -> ValidationResult<()> {
    let const_usize = value.as_usize();
    if const_usize >= const_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadConst const index {const_usize} out of range (const_count {const_count})"
            ),
        });
    }
    Ok(())
}

fn validate_load_accessor_reference(
    expr_index: usize,
    accessor: AccessorIdx,
    accessor_count: usize,
) -> ValidationResult<()> {
    let accessor_usize = accessor.as_usize();
    if accessor_usize >= accessor_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadAccessor accessor index {accessor_usize} out of range (accessor_count {accessor_count})"
            ),
        });
    }
    Ok(())
}
