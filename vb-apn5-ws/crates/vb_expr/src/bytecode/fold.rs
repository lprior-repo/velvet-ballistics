#![forbid(unsafe_code)]
//! Constant folding utilities for expression bytecode compilation.

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::ExprAst;
use vb_core::ConstValue;

pub fn fold_literal(lit: &crate::parser::ExprLiteral) -> Option<ConstValue> {
    match lit {
        crate::parser::ExprLiteral::Null => Some(ConstValue::Null),
        crate::parser::ExprLiteral::Bool(v) => Some(ConstValue::Bool(*v)),
        crate::parser::ExprLiteral::I64(v) => Some(ConstValue::I64(*v)),
        crate::parser::ExprLiteral::Text(_) => None,
    }
}

pub fn fold_unary(op: UnaryOp, inner: &ExprAst) -> Option<ConstValue> {
    let value = super::const_fold_expr(inner)?;
    match op {
        UnaryOp::Not => match value {
            ConstValue::Bool(b) => Some(ConstValue::Bool(!b)),
            _ => None,
        },
        UnaryOp::Neg => match value {
            ConstValue::I64(n) => n.checked_neg().map(ConstValue::I64),
            _ => None,
        },
    }
}

pub fn fold_binary(op: BinaryOp, left: &ExprAst, right: &ExprAst) -> Option<ConstValue> {
    let lv = super::const_fold_expr(left)?;
    let rv = super::const_fold_expr(right)?;
    match op {
        BinaryOp::Add => fold_i64_binop(lv, rv, i64::checked_add),
        BinaryOp::Sub => fold_i64_binop(lv, rv, i64::checked_sub),
        BinaryOp::Mul => fold_i64_binop(lv, rv, i64::checked_mul),
        BinaryOp::Div => fold_i64_div(lv, rv),
        BinaryOp::Eq => Some(ConstValue::Bool(lv == rv)),
        BinaryOp::NotEq => Some(ConstValue::Bool(lv != rv)),
        BinaryOp::Lt => fold_i64_cmp(lv, rv, i64::lt),
        BinaryOp::Lte => fold_i64_cmp(lv, rv, i64::le),
        BinaryOp::Gt => fold_i64_cmp(lv, rv, i64::gt),
        BinaryOp::Gte => fold_i64_cmp(lv, rv, i64::ge),
        BinaryOp::And => fold_bool_binop(lv, rv, |a, b| a && b),
        BinaryOp::Or => fold_bool_binop(lv, rv, |a, b| a || b),
    }
}

fn fold_i64_binop(
    lv: ConstValue,
    rv: ConstValue,
    op: fn(i64, i64) -> Option<i64>,
) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(a), ConstValue::I64(b)) => op(a, b).map(ConstValue::I64),
        _ => None,
    }
}

fn fold_i64_div(lv: ConstValue, rv: ConstValue) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(_), ConstValue::I64(0)) => None,
        (ConstValue::I64(a), ConstValue::I64(b)) => a.checked_div(b).map(ConstValue::I64),
        _ => None,
    }
}

fn fold_i64_cmp(lv: ConstValue, rv: ConstValue, op: fn(&i64, &i64) -> bool) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(a), ConstValue::I64(b)) => Some(ConstValue::Bool(op(&a, &b))),
        _ => None,
    }
}

fn fold_bool_binop(
    lv: ConstValue,
    rv: ConstValue,
    op: fn(bool, bool) -> bool,
) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::Bool(a), ConstValue::Bool(b)) => Some(ConstValue::Bool(op(a, b))),
        _ => None,
    }
}
