//! Text and list operations for expression evaluation.

use crate::errors::EngineError;
use crate::value::SlotValue;
use crate::value_store::ValueStore;

use super::stack::{
    ExprStack, expect_i64, expect_list, expect_symbol, pop_pair, pop_triple, push_value,
};

// ===== Text operations =====

pub(super) fn eval_contains(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (haystack, needle) = pop_pair(stack)?;
    let haystack_id = expect_symbol(haystack)?;
    let needle_id = expect_symbol(needle)?;
    let haystack_str = store
        .symbol(haystack_id)
        .map_err(|_| EngineError::SymbolOutOfBounds {
            symbol: haystack_id,
        })?;
    let needle_str = store
        .symbol(needle_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: needle_id })?;
    push_value(stack, SlotValue::Bool(haystack_str.contains(needle_str)))
}

pub(super) fn eval_starts_with(
    stack: &mut ExprStack,
    store: &ValueStore,
) -> Result<(), EngineError> {
    let (text, prefix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let prefix_id = expect_symbol(prefix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: text_id })?;
    let prefix_str = store
        .symbol(prefix_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: prefix_id })?;
    push_value(stack, SlotValue::Bool(text_str.starts_with(prefix_str)))
}

pub(super) fn eval_ends_with(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (text, suffix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let suffix_id = expect_symbol(suffix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: text_id })?;
    let suffix_str = store
        .symbol(suffix_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: suffix_id })?;
    push_value(stack, SlotValue::Bool(text_str.ends_with(suffix_str)))
}

// ===== List operations =====

pub(super) fn eval_has(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let found = items.contains(&item);
    push_value(stack, SlotValue::Bool(found))
}

pub(super) fn eval_length(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let len = match value {
        SlotValue::Symbol(id) => {
            let s = store
                .symbol(id)
                .map_err(|_| EngineError::SymbolOutOfBounds { symbol: id })?;
            s.len()
        }
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .map_err(|_| EngineError::ListOutOfBounds { list: id })?;
            items.len()
        }
        SlotValue::Object(id) => {
            let fields = store
                .object(id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: id })?;
            fields.len()
        }
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "text, list, or object",
                found: other.type_name(),
            });
        }
    };
    let len_i64 = i64::try_from(len).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "length exceeds i64 range",
    })?;
    push_value(stack, SlotValue::I64(len_i64))
}

pub(super) fn eval_empty(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let is_empty = match value {
        SlotValue::Null => true,
        SlotValue::Symbol(id) => {
            let s = store
                .symbol(id)
                .map_err(|_| EngineError::SymbolOutOfBounds { symbol: id })?;
            s.is_empty()
        }
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .map_err(|_| EngineError::ListOutOfBounds { list: id })?;
            items.is_empty()
        }
        SlotValue::Object(id) => {
            let fields = store
                .object(id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: id })?;
            fields.is_empty()
        }
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "text, list, object, or null",
                found: other.type_name(),
            });
        }
    };
    push_value(stack, SlotValue::Bool(is_empty))
}

pub(super) fn eval_sum(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64(item)?;
        sum = sum
            .checked_add(n)
            .ok_or(EngineError::InvalidCompiledWorkflow {
                reason: "sum overflow",
            })?;
    }
    push_value(stack, SlotValue::I64(sum))
}

pub(super) fn eval_count(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let count = i64::try_from(items.len()).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "count exceeds i64 range",
    })?;
    push_value(stack, SlotValue::I64(count))
}

pub(super) fn eval_append(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(item);
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

pub(super) fn eval_append_if(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    let (list, item, condition) = pop_triple(stack)?;
    let list_id = expect_list(list)?;
    let cond = super::stack::expect_bool(condition)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(item);
    }
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

pub(super) fn eval_unique(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut seen: Vec<SlotValue> = Vec::new();
    for &item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    let new_list = store
        .insert_list(seen.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}
