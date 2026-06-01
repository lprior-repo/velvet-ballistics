#![forbid(unsafe_code)]
//! Binary and unary operation implementations.

use vb_core::value::FiniteF64;
use vb_core::SlotValue;

use crate::lexer::{BinaryOp, UnaryOp};
use crate::ExprResult;

use super::stack::pop_pair;
use super::type_enforcers::{expect_bool, expect_i64};

/// Evaluates one binary operation over two already-popped values.
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => {
            let left_bool = expect_bool(left)?;
            let right_bool = expect_bool(right)?;
            Ok(SlotValue::Bool(left_bool && right_bool))
        }
        BinaryOp::Or => {
            let left_bool = expect_bool(left)?;
            let right_bool = expect_bool(right)?;
            Ok(SlotValue::Bool(left_bool || right_bool))
        }
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

fn eval_add_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() + r.get();
            let finite = FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_add),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_add,
        ),
    }
}

fn eval_sub_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() - r.get();
            let finite = FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_sub),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_sub,
        ),
    }
}

fn eval_mul_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() * r.get();
            let finite = FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_mul),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_mul,
        ),
    }
}

fn eval_div_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() / r.get();
            let finite =
                FiniteF64::new(result).map_err(|_| crate::ExprError::NonFiniteFloat)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_div_values_(l, r),
        (other_left, other_right) => {
            eval_div_values_(expect_i64(other_left)?, expect_i64(other_right)?)
        }
    }
}

fn eval_gt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() > r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::gt),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::gt)
        }
    }
}

fn eval_gte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() >= r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::ge),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::ge)
        }
    }
}

fn eval_lt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() < r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::lt),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::lt)
        }
    }
}

fn eval_lte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() <= r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::le),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::le)
        }
    }
}

fn eval_i64_values_(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(left, right).ok_or(crate::ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values_(left: i64, right: i64) -> ExprResult<SlotValue> {
    if right == 0 {
        return Err(crate::ExprError::DivisionByZero);
    }
    let value = left.checked_div(right).ok_or(crate::ExprError::IntegerOverflow)?;
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
            let result = -f.get();
            let finite = FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        SlotValue::I64(n) => {
            let negated = n.checked_neg().ok_or(crate::ExprError::IntegerOverflow)?;
            Ok(SlotValue::I64(negated))
        }
        other => Err(crate::ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}
