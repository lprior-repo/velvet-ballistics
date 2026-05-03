//! Expression operator evaluation.

use crate::errors::EngineError;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::ExprOp;

use super::stack::{
    expect_bool, expect_object, pop_i64_pair, pop_pair, push_value, ExprStack,
};
use super::ops_text_list::{
    eval_append, eval_append_if, eval_contains, eval_count, eval_empty, eval_ends_with,
    eval_has, eval_length, eval_starts_with, eval_sum, eval_unique,
};

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

fn eval_i64_pair(
    stack: &mut ExprStack,
    op: fn(i64, i64) -> Option<i64>,
) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = op(left, right).ok_or(EngineError::InvalidCompiledWorkflow {
        reason: "integer arithmetic overflow",
    })?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_div(stack: &mut ExprStack) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    if right == 0 {
        return Err(EngineError::DivisionByZero);
    }
    let value = left.checked_div(right).ok_or(EngineError::InvalidCompiledWorkflow {
        reason: "integer division overflow",
    })?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_i64_cmp(stack: &mut ExprStack, op: fn(&i64, &i64) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    push_value(stack, SlotValue::Bool(op(&left, &right)))
}

// ===== Object operations =====

fn eval_exists(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    match value {
        SlotValue::Null => push_value(stack, SlotValue::Bool(false)),
        SlotValue::Object(object_id) => {
            let fields = store
                .object(object_id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: object_id })?;
            push_value(stack, SlotValue::Bool(!fields.is_empty()))
        }
        other => Err(EngineError::TypeMismatch {
            expected: "object or null",
            found: other.type_name(),
        }),
    }
}

fn eval_merge(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let left_id = expect_object(left)?;
    let right_id = expect_object(right)?;
    let left_fields = store
        .object(left_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: left_id })?;
    let right_fields = store
        .object(right_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: right_id })?;
    let mut merged: Vec<crate::value_store::ObjectField> = left_fields.to_vec();
    for &field in right_fields {
        if let Some(pos) = merged.iter().position(|&f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::Object(new_object))
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
        ExprOp::Add => eval_i64_pair(stack, i64::checked_add),
        ExprOp::Sub => eval_i64_pair(stack, i64::checked_sub),
        ExprOp::Mul => eval_i64_pair(stack, i64::checked_mul),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_i64_cmp(stack, i64::gt),
        ExprOp::Gte => eval_i64_cmp(stack, i64::ge),
        ExprOp::Lt => eval_i64_cmp(stack, i64::lt),
        ExprOp::Lte => eval_i64_cmp(stack, i64::le),
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
