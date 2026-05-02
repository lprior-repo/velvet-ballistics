//! Gate 7: Expression stack depth bounded
//!
//! Validates that every expression program's declared max_stack fits within the
//! protocol hard limit and that the declared value matches the actual computed
//! stack depth.

use crate::{ValidationError, ValidationResult};

pub use vb_core::workflow::{ExprOp, ExprProgram};
pub use vb_core::ids::SlotIdx;
pub use vb_core::workflow::WorkflowParts;

/// Maximum expression stack depth allowed by the v1 protocol.
const MAX_EXPR_STACK_DEPTH: u8 = 64;

/// Validates that every expression program's declared max_stack fits within the
/// protocol hard limit and that the declared value matches the actual computed
/// stack depth.
///
/// Gate 7 (boundedness): no expression program may exceed the expression stack
/// bound, and the declared `max_stack` metadata must agree with a fresh
/// recomputation from the opcode stream.
pub fn validate_gate_07_expression_stack_depth(parts: &WorkflowParts) -> ValidationResult<()> {
    let contract_stack = parts.resource_contract.max_expr_stack;
    if contract_stack > MAX_EXPR_STACK_DEPTH {
        return Err(ValidationError::ExpressionStackExceeded {
            declared: usize::from(contract_stack),
            limit: usize::from(MAX_EXPR_STACK_DEPTH),
        });
    }
    for (expr_index, expr) in parts.expressions.iter().enumerate() {
        if expr.max_stack > contract_stack {
            return Err(ValidationError::ExpressionStackExceeded {
                declared: usize::from(expr.max_stack),
                limit: usize::from(contract_stack),
            });
        }
        let computed = compute_stack_depth(&expr.ops)?;
        if computed != expr.max_stack {
            return Err(ValidationError::ExpressionStackMismatch {
                expr_index,
                declared: usize::from(expr.max_stack),
                computed: usize::from(computed),
            });
        }
    }
    Ok(())
}

/// Computes the maximum stack depth for a postfix expression opcode stream.
///
/// Models the stack effects exactly as the core engine does:
/// - LoadSlot/LoadConst/LoadAccessor: pop 0, push 1
/// - Not/Exists/Length/Empty/Sum/Count/Unique: pop 1, push 1
/// - AppendIf: pop 3, push 1
/// - All others (binary): pop 2, push 1
pub fn compute_stack_depth(ops: &[ExprOp]) -> ValidationResult<u8> {
    let mut depth: u8 = 0;
    let mut max_depth: u8 = 0;
    for op in ops {
        let _effect = stack_effect(op);
        let pop_amount = pop_count(op);
        depth = depth.checked_sub(pop_amount).ok_or(
            ValidationError::ExpressionStackExceeded {
                declared: 0,
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            },
        )?;
        let push_amount = push_count(op);
        depth = depth
            .checked_add(push_amount)
            .ok_or(ValidationError::ExpressionStackExceeded {
                declared: usize::from(depth) + usize::from(push_amount),
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            })?;
        if depth > max_depth {
            max_depth = depth;
        }
    }
    Ok(max_depth)
}

/// Returns how many values an opcode pops from the stack.
pub fn pop_count(op: &ExprOp) -> u8 {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => 0,
        ExprOp::Not
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => 1,
        ExprOp::AppendIf => 3,
        _ => 2,
    }
}

/// Returns how many values an opcode pushes onto the stack.
pub fn push_count(_op: &ExprOp) -> u8 {
    // All opcodes push exactly 1 result.
    1
}

/// Returns the net stack effect of a single expression opcode.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn stack_effect(_op: &ExprOp) -> i8 {
    let pop = pop_count(_op);
    let push = push_count(_op);
    (push as i8).saturating_sub(pop as i8)
}
