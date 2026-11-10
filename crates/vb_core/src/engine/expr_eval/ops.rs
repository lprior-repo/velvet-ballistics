#![forbid(unsafe_code)]
//! Expression operator evaluation.

use crate::errors::EngineError;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::ExprOp;

use super::ops_text_list::{
    eval_append, eval_append_if, eval_contains, eval_count, eval_empty, eval_ends_with, eval_has,
    eval_length, eval_starts_with, eval_sum, eval_unique,
};
use super::stack::{ExprStack, expect_bool, expect_object, pop_pair, push_value};

fn eval_eq(stack: &mut ExprStack, positive: bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

fn eval_not(stack: &mut ExprStack) -> Result<(), EngineError> {
    let value = expect_bool(super::stack::pop_value(stack)?)?;
    push_value(stack, SlotValue::Bool(!value))
}

fn eval_bool_pair(stack: &mut ExprStack, op: fn(bool, bool) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(
        stack,
        SlotValue::Bool(op(expect_bool(left)?, expect_bool(right)?)),
    )
}

fn eval_numeric_pair(
    stack: &mut ExprStack,
    op: fn(i64, i64) -> Option<i64>,
    float: fn(f64, f64) -> f64,
) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let (SlotValue::I64(left), SlotValue::I64(right)) = (left, right) else {
        let result = float(promote(left)?, promote(right)?);
        return push_value(stack, SlotValue::F64(crate::value::FiniteF64::new(result)?));
    };
    let value = op(left, right).ok_or(EngineError::InvalidCompiledWorkflow {
        reason: "integer arithmetic overflow",
    })?;
    push_value(stack, SlotValue::I64(value))
}

/// Evaluates integer division.
///
/// Division uses `checked_div` which truncates toward zero (not floor).
/// For example: `-7 / 2 = -3` (not `-4`), and `7 / -2 = -3` (not `-4`).
/// This matches the IEEE 754 semantics for integer division in Rust.
fn eval_div(stack: &mut ExprStack) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let (SlotValue::I64(left), SlotValue::I64(right)) = (left, right) else {
        let value = crate::value::FiniteF64::new(promote(left)? / promote(right)?)?;
        return push_value(stack, SlotValue::F64(value));
    };
    if right == 0 {
        return Err(EngineError::DivisionByZero);
    }
    let value = left
        .checked_div(right)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "integer division overflow",
        })?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_numeric_cmp(
    stack: &mut ExprStack,
    op: fn(&i64, &i64) -> bool,
    float: fn(f64, f64) -> bool,
) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let result = if let (SlotValue::I64(left), SlotValue::I64(right)) = (left, right) {
        op(&left, &right)
    } else {
        float(promote(left)?, promote(right)?)
    };
    push_value(stack, SlotValue::Bool(result))
}

fn promote(value: SlotValue) -> Result<f64, EngineError> {
    match value {
        SlotValue::I64(value) => Ok(crate::value::FiniteF64::from_i64(value).get()),
        SlotValue::F64(value) => Ok(value.get()),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

// ===== Object operations =====

fn eval_exists(stack: &mut ExprStack, _store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    push_value(stack, SlotValue::Bool(!matches!(value, SlotValue::Null)))
}

fn eval_merge_get_fields(
    store: &ValueStore,
    left: SlotValue,
    right: SlotValue,
) -> Result<
    (
        &[crate::value_store::ObjectField],
        &[crate::value_store::ObjectField],
    ),
    EngineError,
> {
    let left_id = expect_object(left)?;
    let right_id = expect_object(right)?;
    let left_fields = store
        .object(left_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: left_id })?;
    let right_fields = store
        .object(right_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: right_id })?;
    Ok((left_fields, right_fields))
}

fn eval_merge_combine_fields(
    left_fields: &[crate::value_store::ObjectField],
    right_fields: &[crate::value_store::ObjectField],
) -> Vec<crate::value_store::ObjectField> {
    let mut merged: Vec<_> = left_fields.to_vec();
    for &field in right_fields {
        if let Some(pos) = merged.iter().position(|&f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    merged
}

fn eval_merge_insert_and_push(
    stack: &mut ExprStack,
    store: &mut ValueStore,
    merged: Vec<crate::value_store::ObjectField>,
) -> Result<(), EngineError> {
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::Object(new_object))
}

fn eval_merge(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let (left_fields, right_fields) = eval_merge_get_fields(store, left, right)?;
    let merged = eval_merge_combine_fields(left_fields, right_fields);
    eval_merge_insert_and_push(stack, store, merged)
}

// ===== Main operator dispatcher =====

pub(super) fn eval_expr_operator(
    op: ExprOp,
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    match op {
        ExprOp::Eq => eval_eq(stack, true),
        ExprOp::NotEq => eval_eq(stack, false),
        ExprOp::And => eval_bool_pair(stack, |left, right| left && right),
        ExprOp::Or => eval_bool_pair(stack, |left, right| left || right),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_numeric_pair(stack, i64::checked_add, |a, b| a + b),
        ExprOp::Sub => eval_numeric_pair(stack, i64::checked_sub, |a, b| a - b),
        ExprOp::Mul => eval_numeric_pair(stack, i64::checked_mul, |a, b| a * b),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_numeric_cmp(stack, i64::gt, |a, b| a > b),
        ExprOp::Gte => eval_numeric_cmp(stack, i64::ge, |a, b| a >= b),
        ExprOp::Lt => eval_numeric_cmp(stack, i64::lt, |a, b| a < b),
        ExprOp::Lte => eval_numeric_cmp(stack, i64::le, |a, b| a <= b),
        ExprOp::Contains => eval_contains(stack, store),
        ExprOp::StartsWith => eval_starts_with(stack, store),
        ExprOp::EndsWith => eval_ends_with(stack, store),
        ExprOp::Has => eval_has(stack, store),
        ExprOp::Exists => eval_exists(stack, store),
        ExprOp::Length => eval_length(stack, store),
        ExprOp::Empty => eval_empty(stack, store),
        ExprOp::Append => eval_append(stack, store),
        ExprOp::AppendIf => eval_append_if(stack, store),
        ExprOp::Merge => eval_merge(stack, store),
        ExprOp::Sum => eval_sum(stack, store),
        ExprOp::Count => eval_count(stack, store),
        ExprOp::Unique => eval_unique(stack, store),
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => {
            Err(EngineError::InternalInvariantViolation {
                reason: "load ops should be handled in eval_expr_op",
            })
        }
    }
}

#[cfg(test)]
mod tests;
