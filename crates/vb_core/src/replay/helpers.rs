#![forbid(unsafe_code)]
//! Store-aware helper operations for replay expression evaluation.
//!
//! Mirrors the engine's helper semantics (`engine::expr_eval`): replay
//! re-executes engine behavior, so helper results here must agree with the
//! forward path. All failures collapse to `ExpressionEvalFailed`, matching
//! the replay arithmetic convention.

use crate::ids::{ListId, ObjectId, StepIdx, SymbolId};
use crate::value::{SlotValue, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::ExprOp;

use super::{ReplayError, ReplayExprStack, pop_pair};

fn failed() -> ReplayError {
    ReplayError::ExpressionEvalFailed {
        step: StepIdx::ZERO,
    }
}

fn expect_symbol(value: SlotValue) -> Result<SymbolId, ReplayError> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        _ => Err(failed()),
    }
}

fn expect_list(value: SlotValue) -> Result<ListId, ReplayError> {
    match value {
        SlotValue::List(id) => Ok(id),
        _ => Err(failed()),
    }
}

fn expect_object(value: SlotValue) -> Result<ObjectId, ReplayError> {
    match value {
        SlotValue::Object(id) => Ok(id),
        _ => Err(failed()),
    }
}

fn expect_bool(value: SlotValue) -> Result<bool, ReplayError> {
    match value {
        SlotValue::Bool(value) => Ok(value),
        _ => Err(failed()),
    }
}

fn expect_i64(value: SlotValue) -> Result<i64, ReplayError> {
    match value {
        SlotValue::I64(value) => Ok(value),
        _ => Err(failed()),
    }
}

/// Dispatches one helper expression operation.
pub(super) fn eval_helper_op(
    op: ExprOp,
    stack: &mut ReplayExprStack,
    store: &mut ValueStore,
) -> Result<(), ReplayError> {
    match op {
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
        _ => Err(ReplayError::Internal {
            reason: "unsupported expression op for replay",
        }),
    }
}

fn eval_contains(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (haystack, needle) = pop_pair(stack)?;
    if let SlotValue::List(id) = haystack {
        let (found, taint) = store
            .list_contains_with_taint(id, needle)
            .map_err(|_| failed())?;
        stack.join_operand_taint(taint);
        return stack.push(SlotValue::Bool(found));
    }
    let haystack_id = expect_symbol(haystack)?;
    let needle_id = expect_symbol(needle)?;
    let haystack_str = store.symbol(haystack_id).map_err(|_| failed())?;
    let needle_str = store.symbol(needle_id).map_err(|_| failed())?;
    stack.push(SlotValue::Bool(haystack_str.contains(needle_str)))
}

fn eval_starts_with(
    stack: &mut ReplayExprStack,
    store: &mut ValueStore,
) -> Result<(), ReplayError> {
    let (text, prefix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let prefix_id = expect_symbol(prefix)?;
    let text_str = store.symbol(text_id).map_err(|_| failed())?;
    let prefix_str = store.symbol(prefix_id).map_err(|_| failed())?;
    stack.push(SlotValue::Bool(text_str.starts_with(prefix_str)))
}

fn eval_ends_with(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (text, suffix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let suffix_id = expect_symbol(suffix)?;
    let text_str = store.symbol(text_id).map_err(|_| failed())?;
    let suffix_str = store.symbol(suffix_id).map_err(|_| failed())?;
    stack.push(SlotValue::Bool(text_str.ends_with(suffix_str)))
}

fn eval_has(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let (found, taint) = store
        .list_contains_with_taint(list_id, item)
        .map_err(|_| failed())?;
    stack.join_operand_taint(taint);
    stack.push(SlotValue::Bool(found))
}

fn eval_exists(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    match value {
        SlotValue::Null => stack.push(SlotValue::Bool(false)),
        SlotValue::Object(object_id) => {
            let fields = store.object(object_id).map_err(|_| failed())?;
            stack.push(SlotValue::Bool(!fields.is_empty()))
        }
        _ => Err(failed()),
    }
}

fn eval_length(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let len = match value {
        SlotValue::Symbol(id) => {
            let text = store.symbol(id).map_err(|_| failed())?;
            text.len()
        }
        SlotValue::List(id) => {
            let items = store.list(id).map_err(|_| failed())?;
            items.len()
        }
        SlotValue::Object(id) => {
            let fields = store.object(id).map_err(|_| failed())?;
            fields.len()
        }
        _ => return Err(failed()),
    };
    let len_i64 = i64::try_from(len).map_err(|_| failed())?;
    stack.push(SlotValue::I64(len_i64))
}

fn eval_empty(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    match value {
        SlotValue::Null => stack.push(SlotValue::Bool(true)),
        SlotValue::Symbol(id) => {
            let text = store.symbol(id).map_err(|_| failed())?;
            stack.push(SlotValue::Bool(text.is_empty()))
        }
        SlotValue::List(id) => {
            let items = store.list(id).map_err(|_| failed())?;
            stack.push(SlotValue::Bool(items.is_empty()))
        }
        SlotValue::Object(id) => {
            let fields = store.object(id).map_err(|_| failed())?;
            stack.push(SlotValue::Bool(fields.is_empty()))
        }
        _ => Err(failed()),
    }
}

fn eval_sum(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list(value)?;
    let items = store.list_items_with_taint(list_id).map_err(|_| failed())?;
    let mut sum: i64 = 0;
    for (item, taint) in items {
        let number = expect_i64(item)?;
        stack.join_operand_taint(taint);
        sum = sum.checked_add(number).ok_or_else(failed)?;
    }
    stack.push(SlotValue::I64(sum))
}

fn eval_count(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list(value)?;
    let items = store.list(list_id).map_err(|_| failed())?;
    let count = i64::try_from(items.len()).map_err(|_| failed())?;
    stack.push(SlotValue::I64(count))
}

fn eval_append(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (item, item_taint) = stack.pop_tainted()?;
    let (list, list_taint) = stack.pop_tainted()?;
    let list_id = expect_list(list)?;
    let (values, taints) = store
        .prepare_list_append(list_id, Some((item, item_taint)))
        .map_err(|_| failed())?;
    let new_list = store
        .insert_list_with_taint(values, taints)
        .map_err(|_| failed())?;
    stack.push_tainted(
        SlotValue::List(new_list),
        join_taint(list_taint, item_taint),
    )
}

fn eval_append_if(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (condition, condition_taint) = stack.pop_tainted()?;
    let (item, item_taint) = stack.pop_tainted()?;
    let (list, list_taint) = stack.pop_tainted()?;
    let list_id = expect_list(list)?;
    let cond = expect_bool(condition)?;
    let (values, taints) = store
        .prepare_list_append(list_id, cond.then_some((item, item_taint)))
        .map_err(|_| failed())?;
    let new_list = store
        .insert_list_with_taint(values, taints)
        .map_err(|_| failed())?;
    stack.push_tainted(
        SlotValue::List(new_list),
        join_taint(join_taint(list_taint, item_taint), condition_taint),
    )
}

fn eval_merge(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_id = expect_object(left)?;
    let right_id = expect_object(right)?;
    let left_fields = store.object(left_id).map_err(|_| failed())?;
    let right_fields = store.object(right_id).map_err(|_| failed())?;
    let mut merged: Vec<crate::value_store::ObjectField> = left_fields.to_vec();
    for field in right_fields {
        if let Some(position) = merged.iter().position(|existing| existing.key == field.key) {
            if let Some(entry) = merged.get_mut(position) {
                *entry = *field;
            }
        } else {
            merged.push(*field);
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| failed())?;
    stack.push(SlotValue::Object(new_object))
}

fn eval_unique(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list(value)?;
    let (values, taints) = store.prepare_list_unique(list_id).map_err(|_| failed())?;
    for taint in taints.iter().copied() {
        stack.join_operand_taint(taint);
    }
    let new_list = store
        .insert_list_with_taint(values, taints)
        .map_err(|_| failed())?;
    stack.push(SlotValue::List(new_list))
}
