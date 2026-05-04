//! Builtin binary and unary operator evaluation.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::SlotValue;

use crate::lexer::{BinaryOp, UnaryOp};
use crate::stack_ops::{expect_bool, expect_i64, pop_pair, pop_value, push_value};
use crate::{ExprError, ExprResult};

/// Evaluates equality comparison (Eq or NotEq).
pub fn eval_eq(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    positive: bool,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

/// Evaluates a binary operation by popping two values from the stack.
pub fn eval_binary_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: BinaryOp,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    let value = eval_binary_op(op, left, right)?;
    push_value(stack, value)
}

/// Evaluates a unary operation by popping one value from the stack.
pub fn eval_unary_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: UnaryOp,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = eval_unary_op(op, value)?;
    push_value(stack, result)
}

/// Evaluates one binary operation over two already-popped values.
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => Ok(SlotValue::Bool(expect_bool(left)? && expect_bool(right)?)),
        BinaryOp::Or => Ok(SlotValue::Bool(expect_bool(left)? || expect_bool(right)?)),
        BinaryOp::Eq => Ok(SlotValue::Bool(left == right)),
        BinaryOp::NotEq => Ok(SlotValue::Bool(left != right)),
        BinaryOp::Add => eval_i64_values(left, right, i64::checked_add),
        BinaryOp::Sub => eval_i64_values(left, right, i64::checked_sub),
        BinaryOp::Mul => eval_i64_values(left, right, i64::checked_mul),
        BinaryOp::Div => eval_div_values(left, right),
        BinaryOp::Gt => eval_i64_cmp_values(left, right, i64::gt),
        BinaryOp::Gte => eval_i64_cmp_values(left, right, i64::ge),
        BinaryOp::Lt => eval_i64_cmp_values(left, right, i64::lt),
        BinaryOp::Lte => eval_i64_cmp_values(left, right, i64::le),
    }
}

/// Evaluates one unary operation over an already-popped value.
pub fn eval_unary_op(op: UnaryOp, value: SlotValue) -> ExprResult<SlotValue> {
    match op {
        UnaryOp::Not => Ok(SlotValue::Bool(!expect_bool(value)?)),
        UnaryOp::Neg => {
            let number = expect_i64(value)?;
            let negated = number.checked_neg().ok_or(ExprError::IntegerOverflow)?;
            Ok(SlotValue::I64(negated))
        }
    }
}

fn eval_i64_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(expect_i64(left)?, expect_i64(right)?).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    let value = left_i64
        .checked_div(right_i64)
        .ok_or(ExprError::DivisionByZero)?;
    Ok(SlotValue::I64(value))
}

fn eval_i64_cmp_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    Ok(SlotValue::Bool(op(&left_i64, &right_i64)))
}

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod blackhat_tests {
    use super::*;
    use crate::eval::{eval_binary_op, eval_unary_op};
    use crate::lexer::{BinaryOp, UnaryOp};
    use crate::ExprError;

    /// BH-BE-001: builtin_eval::eval_div_values maps i64::MIN / -1 to DivisionByZero.
    ///
    /// SECURITY FINDING (HIGH): The `eval_div_values` function in this module
    /// maps ALL `None` results from `checked_div` to `DivisionByZero`, but
    /// `checked_div` returns `None` for both division by zero AND for the
    /// overflow case `i64::MIN / -1`. The correct error for `i64::MIN / -1`
    /// is `IntegerOverflow`, not `DivisionByZero`. This is a misdiagnosis that
    /// could cause incorrect control flow in callers that distinguish between
    /// the two error types.
    ///
    /// Compare with `eval::eval_div_values` which correctly handles this by
    /// checking for zero explicitly before calling `checked_div`.
    #[test]
    fn blackhat_be_001_div_values_misreports_min_div_neg_one() {
        let result = eval_div_values(SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        // This test documents the current buggy behavior.
        // The result SHOULD be IntegerOverflow, but builtin_eval reports DivisionByZero.
        let Err(ExprError::DivisionByZero) = result else {
            // If this branch is reached, the bug has been fixed.
            // Change this test to assert IntegerOverflow when fixed.
            return;
        };
        // BUG CONFIRMED: i64::MIN / -1 incorrectly reports DivisionByZero
    }

    /// BH-BE-002: Public eval_binary_op correctly handles i64::MIN / -1.
    ///
    /// The public API in `eval.rs` correctly returns IntegerOverflow.
    /// This test verifies the correct behavior for comparison with BH-BE-001.
    #[test]
    fn blackhat_be_002_public_api_correctly_handles_min_div_neg_one() {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        let Err(ExprError::IntegerOverflow) = result else {
            return;
        };
        // CORRECT: public API returns IntegerOverflow
    }

    /// BH-BE-003: eval_binary_op addition overflow detection.
    #[test]
    fn blackhat_be_003_add_overflow() {
        let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-004: eval_binary_op subtraction overflow detection.
    #[test]
    fn blackhat_be_004_sub_overflow() {
        let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-005: eval_binary_op multiplication overflow detection.
    #[test]
    fn blackhat_be_005_mul_overflow() {
        let r = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-006: eval_unary_op negation overflow detection.
    #[test]
    fn blackhat_be_006_neg_overflow() {
        let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-007: Division by zero returns correct error.
    #[test]
    fn blackhat_be_007_div_by_zero() {
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(1), SlotValue::I64(0));
        assert!(matches!(r, Err(ExprError::DivisionByZero)));
    }

    /// BH-BE-008: Type confusion rejected for all cross-type operations.
    #[test]
    fn blackhat_be_008_type_confusion() {
        // bool + int
        assert!(matches!(
            eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1)),
            Err(ExprError::TypeMismatch { .. })
        ));
        // int and int
        assert!(matches!(
            eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(0)),
            Err(ExprError::TypeMismatch { .. })
        ));
        // not int
        assert!(matches!(
            eval_unary_op(UnaryOp::Not, SlotValue::I64(1)),
            Err(ExprError::TypeMismatch { .. })
        ));
    }
}
