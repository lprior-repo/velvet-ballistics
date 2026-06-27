#![forbid(unsafe_code)]
//! Store-aware helper implementations for expression evaluation.

use super::environment::{expect_bool, expect_i64, expect_list, expect_object, expect_symbol};
use crate::{ExprError, ExprResult};
use vb_core::SlotValue;
use vb_core::value_store::{ObjectField, ValueStore};

pub(super) fn eval_helper_exists_with_store(
    value: &SlotValue,
    _store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(!matches!(value, SlotValue::Null)))
}

pub(super) fn eval_helper_length_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let len = match *value {
        SlotValue::Symbol(id) => store
            .symbol(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("symbol:{id:?}"),
            })?
            .len(),
        SlotValue::List(id) => store
            .list(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("list:{id:?}"),
            })?
            .len(),
        SlotValue::Object(id) => store
            .object(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("object:{id:?}"),
            })?
            .len(),
        ref other => {
            return Err(ExprError::TypeMismatch {
                expected: "text, list, or object".into(),
                found: other.type_name().into(),
            });
        }
    };
    let len_i64 = i64::try_from(len).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(len_i64))
}

pub(super) fn eval_helper_empty_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let is_empty = match *value {
        SlotValue::Null => true,
        SlotValue::Symbol(id) => store
            .symbol(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("symbol:{id:?}"),
            })?
            .is_empty(),
        SlotValue::List(id) => store
            .list(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("list:{id:?}"),
            })?
            .is_empty(),
        SlotValue::Object(id) => store
            .object(id)
            .map_err(|_| ExprError::InvalidReference {
                reference: format!("object:{id:?}"),
            })?
            .is_empty(),
        ref other => {
            return Err(ExprError::TypeMismatch {
                expected: "text, list, object, or null".into(),
                found: other.type_name().into(),
            });
        }
    };
    Ok(SlotValue::Bool(is_empty))
}

pub(super) fn eval_helper_count_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store.list(list_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("list:{list_id:?}"),
    })?;
    let count = i64::try_from(items.len()).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(count))
}

pub(super) fn eval_helper_unique_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store.list(list_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("list:{list_id:?}"),
    })?;
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

pub(super) fn eval_helper_contains_with_store(
    haystack: &SlotValue,
    needle: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    if matches!(*haystack, SlotValue::F64(_)) || matches!(*needle, SlotValue::F64(_)) {
        return Err(ExprError::TypeMismatch {
            expected: "list, text, or object".into(),
            found: "number".into(),
        });
    }
    let haystack_id = expect_symbol(*haystack)?;
    let needle_id = expect_symbol(*needle)?;
    let haystack_str = store
        .symbol(haystack_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{haystack_id:?}"),
        })?;
    let needle_str = store
        .symbol(needle_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{needle_id:?}"),
        })?;
    Ok(SlotValue::Bool(haystack_str.contains(needle_str)))
}

pub(super) fn eval_helper_starts_with_with_store(
    text: &SlotValue,
    prefix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let prefix_id = expect_symbol(*prefix)?;
    let text_str = store.symbol(text_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("symbol:{text_id:?}"),
    })?;
    let prefix_str = store
        .symbol(prefix_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{prefix_id:?}"),
        })?;
    Ok(SlotValue::Bool(text_str.starts_with(prefix_str)))
}

pub(super) fn eval_helper_ends_with_with_store(
    text: &SlotValue,
    suffix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let suffix_id = expect_symbol(*suffix)?;
    let text_str = store.symbol(text_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("symbol:{text_id:?}"),
    })?;
    let suffix_str = store
        .symbol(suffix_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{suffix_id:?}"),
        })?;
    Ok(SlotValue::Bool(text_str.ends_with(suffix_str)))
}

pub(super) fn eval_helper_has_with_store(
    obj: &SlotValue,
    key: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let obj_id = expect_object(*obj)?;
    let key_id = expect_symbol(*key)?;
    let fields = store.object(obj_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("object:{obj_id:?}"),
    })?;
    let found = fields.iter().any(|f| f.key == key_id);
    Ok(SlotValue::Bool(found))
}

pub(super) fn eval_helper_append_with_store(
    list: &SlotValue,
    item: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let items = store.list(list_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("list:{list_id:?}"),
    })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(*item);
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

pub(super) fn eval_helper_append_if_with_store(
    list: &SlotValue,
    item: &SlotValue,
    condition: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let cond = expect_bool(*condition)?;
    let items = store.list(list_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("list:{list_id:?}"),
    })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(*item);
    }
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

pub(super) fn eval_helper_merge_with_store(
    left: &SlotValue,
    right: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let left_id = expect_object(*left)?;
    let right_id = expect_object(*right)?;
    let left_fields = store
        .object(left_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("object:{left_id:?}"),
        })?;
    let right_fields = store
        .object(right_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("object:{right_id:?}"),
        })?;
    let mut merged: Vec<ObjectField> = left_fields.to_vec();
    for &field in right_fields {
        if let Some(pos) = merged.iter().position(|f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::Object(new_object))
}

pub(super) fn eval_helper_sum_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store.list(list_id).map_err(|_| ExprError::InvalidReference {
        reference: format!("list:{list_id:?}"),
    })?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64(item)?;
        sum = sum.checked_add(n).ok_or(ExprError::IntegerOverflow)?;
    }
    Ok(SlotValue::I64(sum))
}
