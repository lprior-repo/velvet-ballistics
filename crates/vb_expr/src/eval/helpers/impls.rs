#![forbid(unsafe_code)]
//! Store-aware helper implementations.

use vb_core::SlotValue;
use vb_core::ids::{ListId, ObjectId, SymbolId};
use vb_core::value_store::{ObjectField, ValueStore};

use crate::{ExprError, ExprResult};

use crate::eval::type_enforcers::{
    expect_bool, expect_i64, expect_list, expect_object, expect_symbol,
};

pub(crate) fn eval_helper_exists_with_store(
    value: &SlotValue,
    _store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(!matches!(value, SlotValue::Null)))
}

pub(crate) fn eval_helper_length_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let len = helper_length(value, store)?;
    let len_i64 = i64::try_from(len).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(len_i64))
}

fn helper_length(value: &SlotValue, store: &ValueStore) -> ExprResult<usize> {
    match *value {
        SlotValue::Symbol(id) => Ok(symbol_text(store, id)?.len()),
        SlotValue::List(id) => Ok(list_items(store, id)?.len()),
        SlotValue::Object(id) => Ok(object_fields(store, id)?.len()),
        ref other => Err(type_mismatch("text, list, or object", other.type_name())),
    }
}

pub(crate) fn eval_helper_empty_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let is_empty = helper_empty(value, store)?;
    Ok(SlotValue::Bool(is_empty))
}

fn helper_empty(value: &SlotValue, store: &ValueStore) -> ExprResult<bool> {
    match *value {
        SlotValue::Null => Ok(true),
        SlotValue::Symbol(id) => Ok(symbol_text(store, id)?.is_empty()),
        SlotValue::List(id) => Ok(list_items(store, id)?.is_empty()),
        SlotValue::Object(id) => Ok(object_fields(store, id)?.is_empty()),
        ref other => Err(type_mismatch(
            "text, list, object, or null",
            other.type_name(),
        )),
    }
}

pub(crate) fn eval_helper_count_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let count =
        i64::try_from(list_items(store, list_id)?.len()).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(count))
}

pub(crate) fn eval_helper_unique_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = list_items(store, list_id)?;
    let mut seen: Vec<SlotValue> = Vec::new();
    for &item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    let new_list = store
        .insert_list(seen.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

pub(crate) fn eval_helper_contains_with_store(
    haystack: &SlotValue,
    needle: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    if matches!(*haystack, SlotValue::F64(_)) || matches!(*needle, SlotValue::F64(_)) {
        return Err(type_mismatch("list, text, or object", "number"));
    }
    let haystack_id = expect_symbol(*haystack)?;
    let needle_id = expect_symbol(*needle)?;
    let haystack_str = symbol_text(store, haystack_id)?;
    let needle_str = symbol_text(store, needle_id)?;
    Ok(SlotValue::Bool(haystack_str.contains(needle_str)))
}

pub(crate) fn eval_helper_starts_with_with_store(
    text: &SlotValue,
    prefix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let prefix_id = expect_symbol(*prefix)?;
    let text_str = symbol_text(store, text_id)?;
    let prefix_str = symbol_text(store, prefix_id)?;
    Ok(SlotValue::Bool(text_str.starts_with(prefix_str)))
}

pub(crate) fn eval_helper_ends_with_with_store(
    text: &SlotValue,
    suffix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let suffix_id = expect_symbol(*suffix)?;
    let text_str = symbol_text(store, text_id)?;
    let suffix_str = symbol_text(store, suffix_id)?;
    Ok(SlotValue::Bool(text_str.ends_with(suffix_str)))
}

pub(crate) fn eval_helper_has_with_store(
    obj: &SlotValue,
    key: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let obj_id = expect_object(*obj)?;
    let key_id = expect_symbol(*key)?;
    let fields = object_fields(store, obj_id)?;
    let found = fields.iter().any(|f| f.key == key_id);
    Ok(SlotValue::Bool(found))
}

pub(crate) fn eval_helper_append_with_store(
    list: &SlotValue,
    item: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let items = list_items(store, list_id)?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(*item);
    insert_list(store, new_items)
}

pub(crate) fn eval_helper_append_if_with_store(
    list: &SlotValue,
    item: &SlotValue,
    condition: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let cond = expect_bool(*condition)?;
    let items = list_items(store, list_id)?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(*item);
    }
    insert_list(store, new_items)
}

pub(crate) fn eval_helper_merge_with_store(
    left: &SlotValue,
    right: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let left_id = expect_object(*left)?;
    let right_id = expect_object(*right)?;
    let left_fields = object_fields(store, left_id)?;
    let right_fields = object_fields(store, right_id)?;
    let merged = merge_fields(left_fields, right_fields);
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::Object(new_object))
}

fn merge_fields(left_fields: &[ObjectField], right_fields: &[ObjectField]) -> Vec<ObjectField> {
    let mut merged: Vec<ObjectField> = left_fields.to_vec();
    for &field in right_fields {
        merge_one_field(&mut merged, field);
    }
    merged
}

fn merge_one_field(merged: &mut Vec<ObjectField>, field: ObjectField) {
    if let Some(pos) = merged.iter().position(|f| f.key == field.key) {
        if let Some(entry) = merged.get_mut(pos) {
            *entry = field;
        }
    } else {
        merged.push(field);
    }
}

pub(crate) fn eval_helper_sum_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = list_items(store, list_id)?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64(item)?;
        sum = sum.checked_add(n).ok_or(ExprError::IntegerOverflow)?;
    }
    Ok(SlotValue::I64(sum))
}

fn insert_list(store: &mut ValueStore, items: Vec<SlotValue>) -> ExprResult<SlotValue> {
    let new_list = store
        .insert_list(items.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

fn symbol_text(store: &ValueStore, id: SymbolId) -> ExprResult<&str> {
    store.symbol(id).map_err(|_| invalid_symbol(id))
}

fn list_items(store: &ValueStore, id: ListId) -> ExprResult<&[SlotValue]> {
    store.list(id).map_err(|_| invalid_list(id))
}

fn object_fields(store: &ValueStore, id: ObjectId) -> ExprResult<&[ObjectField]> {
    store.object(id).map_err(|_| invalid_object(id))
}

fn invalid_symbol(id: SymbolId) -> ExprError {
    ExprError::InvalidReference {
        reference: format!("symbol:{id:?}"),
    }
}

fn invalid_list(id: ListId) -> ExprError {
    ExprError::InvalidReference {
        reference: format!("list:{id:?}"),
    }
}

fn invalid_object(id: ObjectId) -> ExprError {
    ExprError::InvalidReference {
        reference: format!("object:{id:?}"),
    }
}

fn type_mismatch(expected: &str, found: &str) -> ExprError {
    ExprError::TypeMismatch {
        expected: expected.into(),
        found: found.into(),
    }
}
