#![forbid(unsafe_code)]
//! Expression bytecode program validation.
//!
//! Validates that each expression program has consistent stack metadata and
//! that accessor, slot, and constant references are within bounds.

use crate::ids::{AccessorIdx, ConstIdx, SlotIdx};
use crate::workflow::{ExprOp, ExprProgram, WorkflowError};

/// Validates a single expression program by reconstructing it through
/// [`ExprProgram::try_from_parts`] and checking accessor, slot, and constant
/// references against the declared counts.
fn validate_expression(
    expression: &ExprProgram,
    slot_count: u16,
    const_count: usize,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
    validate_expression_ops(expression, slot_count, const_count, accessor_count)
}

/// Checks that all slot, constant, and accessor references in the expression
/// fall within the declared bounds. Without this check, an untrusted workflow
/// can be admitted with a `LoadSlot` or `LoadConst` that points past the end of
/// its pool — the bytecode will pass stack validation but reference a
/// non-existent runtime slot or constant.
fn validate_expression_ops(
    expression: &ExprProgram,
    slot_count: u16,
    const_count: usize,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for op in expression.ops.as_ref() {
        match *op {
            ExprOp::LoadSlot(slot) => validate_slot_ref(slot, slot_count)?,
            ExprOp::LoadConst(idx) => validate_const_ref(idx, const_count)?,
            ExprOp::LoadAccessor(accessor) => validate_accessor(accessor, accessor_count)?,
            _ => {}
        }
    }
    Ok(())
}

/// Validates that a [`SlotIdx`] falls within the declared slot count.
fn validate_slot_ref(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            crate::errors::CoreError::InvalidCompiledWorkflow {
                reason: "expression LoadSlot out of bounds",
            },
        ))
    }
}

/// Validates that a [`ConstIdx`] falls within the declared constant count.
fn validate_const_ref(idx: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if idx.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            crate::errors::CoreError::InvalidCompiledWorkflow {
                reason: "expression LoadConst out of bounds",
            },
        ))
    }
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

/// Validates all expressions against the declared slot, constant, and accessor
/// pools.
pub(crate) fn validate_expressions(
    expressions: &[ExprProgram],
    slot_count: u16,
    const_count: usize,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for expression in expressions {
        validate_expression(expression, slot_count, const_count, accessor_count)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::ExprProgram;

    fn program(ops: Vec<ExprOp>) -> ExprProgram {
        let ops: Box<[ExprOp]> = ops.into_boxed_slice();
        let max_stack = ops
            .iter()
            .map(|op| match op {
                ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => 1,
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        ExprProgram {
            ops,
            max_stack: u8::try_from(max_stack).expect("test stack fits"),
            constants: Box::new([]),
        }
    }

    #[test]
    fn rejects_load_slot_out_of_bounds() {
        let expression = program(vec![ExprOp::LoadSlot(SlotIdx::new(5))]);
        let err = validate_expressions(std::slice::from_ref(&expression), 2, 0, 0)
            .expect_err("LoadSlot past slot_count must fail");
        match err {
            WorkflowError::Expression(crate::errors::CoreError::InvalidCompiledWorkflow {
                reason,
            }) => assert_eq!(reason, "expression LoadSlot out of bounds"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn accepts_load_slot_in_bounds() {
        let expression = program(vec![ExprOp::LoadSlot(SlotIdx::new(1))]);
        validate_expressions(std::slice::from_ref(&expression), 2, 0, 0)
            .expect("LoadSlot within slot_count must succeed");
    }

    #[test]
    fn rejects_load_const_out_of_bounds() {
        let expression = program(vec![ExprOp::LoadConst(ConstIdx::new(3))]);
        let err = validate_expressions(std::slice::from_ref(&expression), 1, 2, 0)
            .expect_err("LoadConst past const_count must fail");
        match err {
            WorkflowError::Expression(crate::errors::CoreError::InvalidCompiledWorkflow {
                reason,
            }) => assert_eq!(reason, "expression LoadConst out of bounds"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn accepts_load_const_in_bounds() {
        let expression = program(vec![ExprOp::LoadConst(ConstIdx::new(1))]);
        validate_expressions(std::slice::from_ref(&expression), 1, 2, 0)
            .expect("LoadConst within const_count must succeed");
    }
}
