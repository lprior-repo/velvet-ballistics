#![forbid(unsafe_code)]
//! Scalar expression operator evaluation.

use core::cmp::Ordering;

use super::environment::expect_bool;
use crate::lexer::{BinaryOp, UnaryOp};
use crate::{ExprError, ExprResult};
use vb_core::SlotValue;
use vb_core::numeric::{self, NumericError};

/// Evaluates one binary operation over two already-popped values.
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => eval_and_op(left, right),
        BinaryOp::Or => eval_or_op(left, right),
        BinaryOp::Eq => Ok(SlotValue::Bool(left == right)),
        BinaryOp::NotEq => Ok(SlotValue::Bool(left != right)),
        BinaryOp::Add => numeric::add(left, right).map_err(map_numeric),
        BinaryOp::Sub => numeric::sub(left, right).map_err(map_numeric),
        BinaryOp::Mul => numeric::mul(left, right).map_err(map_numeric),
        BinaryOp::Div => numeric::div(left, right).map_err(map_numeric),
        BinaryOp::Gt => eval_cmp_op(left, right, is_greater),
        BinaryOp::Gte => eval_cmp_op(left, right, is_greater_or_equal),
        BinaryOp::Lt => eval_cmp_op(left, right, is_less),
        BinaryOp::Lte => eval_cmp_op(left, right, is_less_or_equal),
    }
}

fn map_numeric(error: NumericError) -> ExprError {
    match error {
        NumericError::NotNumeric { found } => ExprError::TypeMismatch {
            expected: "number".into(),
            found: found.into(),
        },
        NumericError::Overflow => ExprError::IntegerOverflow,
        NumericError::DivisionByZero => ExprError::DivisionByZero,
        NumericError::NonFinite => ExprError::NonFiniteFloat,
    }
}

fn eval_and_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_bool = expect_bool(left)?;
    let right_bool = expect_bool(right)?;
    Ok(SlotValue::Bool(left_bool && right_bool))
}

fn eval_or_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_bool = expect_bool(left)?;
    let right_bool = expect_bool(right)?;
    Ok(SlotValue::Bool(left_bool || right_bool))
}

fn eval_cmp_op(
    left: SlotValue,
    right: SlotValue,
    accept: fn(Ordering) -> bool,
) -> ExprResult<SlotValue> {
    let ordering = numeric::compare(left, right).map_err(map_numeric)?;
    Ok(SlotValue::Bool(accept(ordering)))
}

fn is_greater(ordering: Ordering) -> bool {
    ordering == Ordering::Greater
}

fn is_greater_or_equal(ordering: Ordering) -> bool {
    matches!(ordering, Ordering::Greater | Ordering::Equal)
}

fn is_less(ordering: Ordering) -> bool {
    ordering == Ordering::Less
}

fn is_less_or_equal(ordering: Ordering) -> bool {
    matches!(ordering, Ordering::Less | Ordering::Equal)
}

/// Evaluates one unary operation over an already-popped value.
pub fn eval_unary_op(op: UnaryOp, value: SlotValue) -> ExprResult<SlotValue> {
    match op {
        UnaryOp::Not => Ok(SlotValue::Bool(!expect_bool(value)?)),
        UnaryOp::Neg => eval_neg_op(value),
    }
}

fn eval_neg_op(value: SlotValue) -> ExprResult<SlotValue> {
    match value {
        SlotValue::F64(f) => {
            let finite = vb_core::value::FiniteF64::new(-f.get())?;
            Ok(SlotValue::F64(finite))
        }
        SlotValue::I64(n) => {
            let negated = n.checked_neg().ok_or(ExprError::IntegerOverflow)?;
            Ok(SlotValue::I64(negated))
        }
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}
