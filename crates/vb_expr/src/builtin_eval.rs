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
