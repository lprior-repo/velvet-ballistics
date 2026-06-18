#![forbid(unsafe_code)]
//! Gate 7: Expression stack depth bounded

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{ExprOp, WorkflowParts};

/// Maximum byte length for a compiled action capability requirement name.
pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

const MAX_EXPR_STACK_DEPTH: u8 = 64;

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

pub fn compute_stack_depth(ops: &[ExprOp]) -> ValidationResult<u8> {
    let mut depth: u8 = 0;
    let mut max_depth: u8 = 0;
    for op in ops {
        let _effect = stack_effect(op);
        let pop_amount = pop_count(op);
        depth = depth
            .checked_sub(pop_amount)
            .ok_or(ValidationError::ExpressionStackExceeded {
                declared: 0,
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            })?;
        let push_amount = push_count(op);
        depth = depth
            .checked_add(push_amount)
            .ok_or(ValidationError::ExpressionStackExceeded {
                declared: usize::from(depth).saturating_add(usize::from(push_amount)),
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            })?;
        if depth > max_depth {
            max_depth = depth;
        }
    }
    Ok(max_depth)
}

fn pop_count(op: &ExprOp) -> u8 {
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

fn push_count(_op: &ExprOp) -> u8 {
    1
}

#[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
pub fn stack_effect(_op: &ExprOp) -> i8 {
    let pop: i16 = i16::from(pop_count(_op));
    let push: i16 = i16::from(push_count(_op));
    let net = push.saturating_sub(pop);
    match i8::try_from(net) {
        Ok(value) => value,
        Err(_) => 0,
    }
}
