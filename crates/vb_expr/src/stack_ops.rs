#![forbid(unsafe_code)]
//! Stack operation primitives for bounded stack-based expression evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use vb_core::SlotValue;

use crate::{ExprError, ExprResult};

/// Pushes a value onto the evaluation stack.
pub fn push_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> ExprResult<()> {
    stack.try_push(value).map_err(|_| ExprError::StackOverflow {
        max: MAX_EXPRESSION_STACK,
    })
}

/// Pops a single value from the evaluation stack.
pub fn pop_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<SlotValue> {
    stack.pop().ok_or(ExprError::StackUnderflow)
}

/// Pops a pair of values from the evaluation stack (right, then left).
pub fn pop_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue)> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

/// Extracts a boolean from a SlotValue, returning TypeMismatch on failure.
pub fn expect_bool(value: SlotValue) -> ExprResult<bool> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        other => Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: other.type_name().into(),
        }),
    }
}

/// Extracts an i64 from a SlotValue, returning TypeMismatch on failure.
pub fn expect_i64(value: SlotValue) -> ExprResult<i64> {
    match value {
        SlotValue::I64(n) => Ok(n),
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}
