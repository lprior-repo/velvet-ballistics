#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{ExprOp, WorkflowParts};

// ---------------------------------------------------------------------------
// Gate 7: Expression stack depth bounded
// ---------------------------------------------------------------------------

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
        // Apply pop first (subtract), then push (add).
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

/// Returns how many values an opcode pops from the stack.
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

/// Returns how many values an opcode pushes onto the stack.
fn push_count(_op: &ExprOp) -> u8 {
    // All opcodes push exactly 1 result.
    1
}

/// Returns the net stack effect of a single expression opcode.
///
/// Uses `i16::from()` for safe widening conversion (infallible), then
/// computes the net effect. Since push is always 1 and pop is 0..=3, the
/// result is always in i8 range (-2..=1).
///
/// Explicit match is used instead of `unwrap_or` / `unwrap_or_default` to
/// comply with the "no unwrap" engineering rule, even though these methods
/// are safe (they return a default on `Err`, they don't panic).
#[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
fn stack_effect(_op: &ExprOp) -> i8 {
    let pop: i16 = i16::from(pop_count(_op));
    let push: i16 = i16::from(push_count(_op));
    let net = push.saturating_sub(pop);
    // Convert back to i8: net is in [-2, 1], always fits.
    match i8::try_from(net) {
        Ok(value) => value,
        Err(_) => 0, // unreachable: net is always in i8 range [-2, 1]
    }
}
