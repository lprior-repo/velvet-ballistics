#![forbid(unsafe_code)]
//! Expression operator evaluation.

use crate::errors::EngineError;
use crate::numeric::{self, NumericError};
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
    op: fn(SlotValue, SlotValue) -> Result<SlotValue, NumericError>,
) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, op(left, right).map_err(map_numeric)?)
}

fn map_numeric(error: NumericError) -> EngineError {
    match error {
        NumericError::NotNumeric { found } => EngineError::TypeMismatch {
            expected: "number",
            found,
        },
        NumericError::Overflow => EngineError::InvalidCompiledWorkflow {
            reason: "integer arithmetic overflow",
        },
        NumericError::DivisionByZero => EngineError::DivisionByZero,
        NumericError::NonFinite => EngineError::NonFiniteNumber,
    }
}

fn eval_div(stack: &mut ExprStack) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let result = numeric::div(left, right).map_err(|error| match error {
        NumericError::Overflow => EngineError::InvalidCompiledWorkflow {
            reason: "integer division overflow",
        },
        other => map_numeric(other),
    })?;
    push_value(stack, result)
}

fn eval_numeric_cmp(
    stack: &mut ExprStack,
    accept: fn(core::cmp::Ordering) -> bool,
) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let ordering = numeric::compare(left, right).map_err(map_numeric)?;
    push_value(stack, SlotValue::Bool(accept(ordering)))
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
        ExprOp::Add => eval_numeric_pair(stack, numeric::add),
        ExprOp::Sub => eval_numeric_pair(stack, numeric::sub),
        ExprOp::Mul => eval_numeric_pair(stack, numeric::mul),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_numeric_cmp(stack, core::cmp::Ordering::is_gt),
        ExprOp::Gte => eval_numeric_cmp(stack, core::cmp::Ordering::is_ge),
        ExprOp::Lt => eval_numeric_cmp(stack, core::cmp::Ordering::is_lt),
        ExprOp::Lte => eval_numeric_cmp(stack, core::cmp::Ordering::is_le),
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
                reason: "load ops should be handled in eval_expr_op".into(),
            })
        }
    }
}

#[cfg(test)]
mod tests;
