#![forbid(unsafe_code)]
//! Public API helper evaluation.

use crate::parser::ExprHelper;
use vb_core::SlotValue;
use vb_core::value_store::ValueStore;

use crate::{ExprError, ExprResult};

use super::super::impls::{
    eval_helper_append_if_with_store, eval_helper_append_with_store,
    eval_helper_contains_with_store, eval_helper_count_with_store, eval_helper_empty_with_store,
    eval_helper_ends_with_with_store, eval_helper_exists_with_store, eval_helper_has_with_store,
    eval_helper_length_with_store, eval_helper_merge_with_store,
    eval_helper_starts_with_with_store, eval_helper_sum_with_store, eval_helper_unique_with_store,
};
use super::args::{one_arg, three_args, two_args};

/// Evaluates helper functions with full `ValueStore` access.
///
/// Unlike [`eval_helper`], this variant can resolve opaque handles (`List`,
/// `Object`, `Symbol`) through the store, enabling complete evaluation of
/// helpers like `Empty`, `Unique`, `Length`, `Contains`, etc.
pub fn eval_helper_with_store(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists | ExprHelper::Length | ExprHelper::Empty => {
            eval_store_one_arg(helper, args, store)
        }
        ExprHelper::Count | ExprHelper::Unique | ExprHelper::Sum => {
            eval_store_one_arg(helper, args, store)
        }
        ExprHelper::Contains | ExprHelper::StartsWith | ExprHelper::EndsWith => {
            eval_store_two_args(helper, args, store)
        }
        ExprHelper::Has | ExprHelper::Append | ExprHelper::Merge => {
            eval_store_two_args(helper, args, store)
        }
        ExprHelper::AppendIf => eval_store_three_args(helper, args, store),
    }
}

fn eval_store_one_arg(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let value = one_arg(args, helper)?;
    match helper {
        ExprHelper::Exists => eval_helper_exists_with_store(value, store),
        ExprHelper::Length => eval_helper_length_with_store(value, store),
        ExprHelper::Empty => eval_helper_empty_with_store(value, store),
        ExprHelper::Count => eval_helper_count_with_store(value, store),
        ExprHelper::Unique => eval_helper_unique_with_store(value, store),
        ExprHelper::Sum => eval_helper_sum_with_store(value, store),
        _ => Err(unknown_helper(helper)),
    }
}

fn eval_store_two_args(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let (left, right) = two_args(args, helper)?;
    match helper {
        ExprHelper::Contains => eval_helper_contains_with_store(left, right, store),
        ExprHelper::StartsWith => eval_helper_starts_with_with_store(left, right, store),
        ExprHelper::EndsWith => eval_helper_ends_with_with_store(left, right, store),
        ExprHelper::Has => eval_helper_has_with_store(left, right, store),
        ExprHelper::Append => eval_helper_append_with_store(left, right, store),
        ExprHelper::Merge => eval_helper_merge_with_store(left, right, store),
        _ => Err(unknown_helper(helper)),
    }
}

fn eval_store_three_args(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let (first, second, third) = three_args(args, helper)?;
    eval_helper_append_if_with_store(first, second, third, store)
}

/// Evaluates helper behavior that is local to scalar/handle values.
///
/// Note: Most helpers require a ValueStore to resolve opaque handles (List, Object, Symbol).
/// This function only supports helpers that work without store access.
/// For full helper evaluation, use [`eval_helper_with_store`].
pub fn eval_helper(helper: ExprHelper, args: &[SlotValue]) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => eval_exists_no_store(args, helper),
        ExprHelper::Empty => eval_empty_no_store(args, helper),
        ExprHelper::Length | ExprHelper::Count => eval_length_count_no_store(args, helper),
        ExprHelper::Unique => eval_unique_no_store(args, helper),
        ExprHelper::Contains => eval_contains_no_store(args, helper),
        ExprHelper::StartsWith | ExprHelper::EndsWith => {
            require_two_arg_store(args, helper, TEXT_STORE_REQUIRED, SYMBOL_HANDLE)
        }
        ExprHelper::Has => {
            require_two_arg_store(args, helper, OBJECT_FIELD_REQUIRED, OBJECT_HANDLE)
        }
        ExprHelper::Append => {
            require_two_arg_store(args, helper, LIST_APPEND_REQUIRED, LIST_HANDLE)
        }
        ExprHelper::AppendIf => require_three_arg_store(args, helper, LIST_APPEND_REQUIRED),
        ExprHelper::Merge => {
            require_two_arg_store(args, helper, OBJECT_MERGE_REQUIRED, OBJECT_HANDLE)
        }
        ExprHelper::Sum => require_one_arg_store(args, helper, LIST_SUM_REQUIRED),
    }
}

const LIST_HANDLE: &str = "list handle without store";
const LIST_APPEND_REQUIRED: &str = "value-store context required for list append";
const LIST_CONTAINS_REQUIRED: &str = "value-store context required for list contains check";
const LIST_LENGTH_REQUIRED: &str = "value-store context required for list length";
const LIST_SUM_REQUIRED: &str = "value-store context required for list sum";
const OBJECT_FIELD_REQUIRED: &str = "value-store context required for object field lookup";
const OBJECT_HANDLE: &str = "object handle without store";
const OBJECT_MERGE_REQUIRED: &str = "value-store context required for object merge";
const SYMBOL_HANDLE: &str = "symbol handle without store";
const TEXT_STORE_REQUIRED: &str = "value-store context required for text operations";

fn eval_exists_no_store(args: &[SlotValue], helper: ExprHelper) -> ExprResult<SlotValue> {
    let value = one_arg(args, helper)?;
    Ok(SlotValue::Bool(!matches!(*value, SlotValue::Null)))
}

fn eval_empty_no_store(args: &[SlotValue], helper: ExprHelper) -> ExprResult<SlotValue> {
    let value = one_arg(args, helper)?;
    match *value {
        SlotValue::Null => Ok(SlotValue::Bool(true)),
        SlotValue::F64(_) => type_mismatch("list, text, object, or null", "number"),
        SlotValue::List(_) => type_mismatch(
            "value-store context required for list emptiness check",
            LIST_HANDLE,
        ),
        other => type_mismatch("list or null", other.type_name()),
    }
}

fn eval_length_count_no_store(args: &[SlotValue], helper: ExprHelper) -> ExprResult<SlotValue> {
    let value = one_arg(args, helper)?;
    match *value {
        SlotValue::F64(_) => type_mismatch("list, text, or object", "number"),
        SlotValue::List(_) | SlotValue::Null => type_mismatch(LIST_LENGTH_REQUIRED, LIST_HANDLE),
        other => type_mismatch("list", other.type_name()),
    }
}

fn eval_unique_no_store(args: &[SlotValue], helper: ExprHelper) -> ExprResult<SlotValue> {
    let value = one_arg(args, helper)?;
    match *value {
        SlotValue::List(_) => type_mismatch(
            "value-store context required for list deduplication",
            LIST_HANDLE,
        ),
        other => type_mismatch("list", other.type_name()),
    }
}

fn eval_contains_no_store(args: &[SlotValue], helper: ExprHelper) -> ExprResult<SlotValue> {
    let (left, right) = two_args(args, helper)?;
    if matches!(*left, SlotValue::F64(_)) || matches!(*right, SlotValue::F64(_)) {
        return type_mismatch("list, text, or object", "number");
    }
    type_mismatch(LIST_CONTAINS_REQUIRED, LIST_HANDLE)
}

fn require_one_arg_store(
    args: &[SlotValue],
    helper: ExprHelper,
    expected: &str,
) -> ExprResult<SlotValue> {
    one_arg(args, helper)?;
    type_mismatch(expected, LIST_HANDLE)
}

fn require_two_arg_store(
    args: &[SlotValue],
    helper: ExprHelper,
    expected: &str,
    found: &str,
) -> ExprResult<SlotValue> {
    two_args(args, helper)?;
    type_mismatch(expected, found)
}

fn require_three_arg_store(
    args: &[SlotValue],
    helper: ExprHelper,
    expected: &str,
) -> ExprResult<SlotValue> {
    three_args(args, helper)?;
    type_mismatch(expected, LIST_HANDLE)
}

fn type_mismatch(expected: &str, found: &str) -> ExprResult<SlotValue> {
    Err(ExprError::TypeMismatch {
        expected: expected.into(),
        found: found.into(),
    })
}

fn unknown_helper(helper: ExprHelper) -> ExprError {
    ExprError::UnknownHelper {
        helper: crate::parser::helper_name(helper).into(),
    }
}
