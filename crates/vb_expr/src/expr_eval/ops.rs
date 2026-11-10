#![forbid(unsafe_code)]
//! Scalar expression operator evaluation.

use super::environment::expect_bool;
use crate::lexer::{BinaryOp, UnaryOp};
use crate::{ExprError, ExprResult};
use vb_core::SlotValue;

/// Convert a numeric slot to f64 (i64 values are promoted).
fn slot_to_f64(value: SlotValue) -> ExprResult<f64> {
    match value {
        SlotValue::F64(f) => Ok(f.get()),
        SlotValue::I64(n) => Ok(vb_core::value::FiniteF64::from_i64(n).get()),
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}

/// Evaluates one binary operation over two already-popped values.
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => eval_and_op(left, right),
        BinaryOp::Or => eval_or_op(left, right),
        BinaryOp::Eq => Ok(SlotValue::Bool(left == right)),
        BinaryOp::NotEq => Ok(SlotValue::Bool(left != right)),
        BinaryOp::Add => eval_add_op(left, right),
        BinaryOp::Sub => eval_sub_op(left, right),
        BinaryOp::Mul => eval_mul_op(left, right),
        BinaryOp::Div => eval_div_op(left, right),
        BinaryOp::Gt => eval_gt_op(left, right),
        BinaryOp::Gte => eval_gte_op(left, right),
        BinaryOp::Lt => eval_lt_op(left, right),
        BinaryOp::Lte => eval_lte_op(left, right),
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

fn eval_add_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_values_(l, r, i64::checked_add);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    let finite = vb_core::value::FiniteF64::new(l + r)?;
    Ok(SlotValue::F64(finite))
}

fn eval_sub_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_values_(l, r, i64::checked_sub);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    let finite = vb_core::value::FiniteF64::new(l - r)?;
    Ok(SlotValue::F64(finite))
}

fn eval_mul_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_values_(l, r, i64::checked_mul);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    let finite = vb_core::value::FiniteF64::new(l * r)?;
    Ok(SlotValue::F64(finite))
}

fn eval_div_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_div_values_(l, r);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    let finite = vb_core::value::FiniteF64::new(l / r).map_err(|_| ExprError::NonFiniteFloat)?;
    Ok(SlotValue::F64(finite))
}

fn eval_gt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_cmp_values_(l, r, i64::gt);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    Ok(SlotValue::Bool(l > r))
}

fn eval_gte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_cmp_values_(l, r, i64::ge);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    Ok(SlotValue::Bool(l >= r))
}

fn eval_lt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_cmp_values_(l, r, i64::lt);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    Ok(SlotValue::Bool(l < r))
}

fn eval_lte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    if let (SlotValue::I64(l), SlotValue::I64(r)) = (left, right) {
        return eval_i64_cmp_values_(l, r, i64::le);
    }
    let l = slot_to_f64(left)?;
    let r = slot_to_f64(right)?;
    Ok(SlotValue::Bool(l <= r))
}

fn eval_i64_values_(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(left, right).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values_(left: i64, right: i64) -> ExprResult<SlotValue> {
    if right == 0 {
        return Err(ExprError::DivisionByZero);
    }
    let value = left.checked_div(right).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_i64_cmp_values_(
    left: i64,
    right: i64,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(op(&left, &right)))
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
