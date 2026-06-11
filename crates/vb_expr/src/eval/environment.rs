#![forbid(unsafe_code)]
//! Evaluation environment: stack operations and type expectations.

use arrayvec::ArrayVec;
use vb_core::SlotValue;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;

use crate::parser::ExprHelper;
use crate::{ExprError, ExprResult};
use vb_core::limits::MAX_EXPRESSION_STACK;

pub(crate) fn push_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> ExprResult<()> {
    stack.try_push(value).map_err(|_| ExprError::StackOverflow {
        max: MAX_EXPRESSION_STACK,
    })
}

pub(crate) fn pop_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<SlotValue> {
    stack.pop().ok_or(ExprError::StackUnderflow)
}

pub(crate) fn pop_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue)> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

pub(crate) fn pop_triple(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue, SlotValue)> {
    let third = pop_value(stack)?;
    let second = pop_value(stack)?;
    let first = pop_value(stack)?;
    Ok((first, second, third))
}

pub(super) fn expect_bool(value: SlotValue) -> ExprResult<bool> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        other => Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(super) fn expect_i64(value: SlotValue) -> ExprResult<i64> {
    match value {
        SlotValue::I64(n) => Ok(n),
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(super) fn expect_symbol(value: SlotValue) -> ExprResult<vb_core::ids::SymbolId> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(super) fn expect_list(value: SlotValue) -> ExprResult<vb_core::ids::ListId> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(super) fn expect_object(value: SlotValue) -> ExprResult<vb_core::ids::ObjectId> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "object".into(),
            found: other.type_name().into(),
        }),
    }
}

pub fn eval_helper(helper: ExprHelper, args: &[SlotValue]) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => eval_helper_exists(args),
        ExprHelper::Length | ExprHelper::Count => eval_helper_length(args),
        ExprHelper::Empty => eval_helper_empty(args),
        ExprHelper::Unique => eval_helper_unique(args),
        ExprHelper::Contains => eval_helper_contains(args),
        ExprHelper::StartsWith => eval_helper_starts_with(args),
        ExprHelper::EndsWith => eval_helper_ends_with(args),
        ExprHelper::Has => eval_helper_has(args),
        ExprHelper::Append => eval_helper_append(args),
        ExprHelper::AppendIf => eval_helper_append_if(args),
        ExprHelper::Merge => eval_helper_merge(args),
        ExprHelper::Sum => eval_helper_sum(args),
    }
}

fn eval_helper_exists(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Exists)?;
    Ok(SlotValue::Bool(!matches!(*value, SlotValue::Null)))
}

fn eval_helper_length(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Length)?;
    match value {
        SlotValue::F64(_) => Err(ExprError::TypeMismatch {
            expected: "list, text, or object".into(),
            found: "number".into(),
        }),
        SlotValue::List(_) | SlotValue::Null => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list length".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_empty(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Empty)?;
    match *value {
        SlotValue::F64(_) => Err(ExprError::TypeMismatch {
            expected: "list, text, object, or null".into(),
            found: "number".into(),
        }),
        SlotValue::Null => Ok(SlotValue::Bool(true)),
        SlotValue::List(_) => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list emptiness check".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list or null".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_unique(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Unique)?;
    match *value {
        SlotValue::List(_) => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list deduplication".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_contains(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (haystack, needle) = two_args(args, ExprHelper::Contains)?;
    match (*haystack, *needle) {
        (SlotValue::Symbol(_), SlotValue::Symbol(_)) => context_required_contains("text"),
        (SlotValue::Symbol(_), other) => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: other.type_name().into(),
        }),
        (SlotValue::List(_), _) => context_required_contains("list"),
        (SlotValue::Object(_), SlotValue::Symbol(_)) => context_required_contains("object"),
        (SlotValue::Object(_), other) => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: other.type_name().into(),
        }),
        (other, _) => Err(ExprError::TypeMismatch {
            expected: "list, text, or object".into(),
            found: other.type_name().into(),
        }),
    }
}

fn context_required_contains(kind: &'static str) -> ExprResult<SlotValue> {
    Err(ExprError::TypeMismatch {
        expected: format!("value-store context required for {kind} contains check"),
        found: format!("{kind} handle without store"),
    })
}

fn eval_helper_starts_with(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (text, prefix) = two_args(args, ExprHelper::StartsWith)?;
    let _text_id = expect_symbol(*text)?;
    let _prefix_id = expect_symbol(*prefix)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for text operations".into(),
        found: "symbol handle without store".into(),
    })
}

fn eval_helper_ends_with(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (text, suffix) = two_args(args, ExprHelper::EndsWith)?;
    let _text_id = expect_symbol(*text)?;
    let _suffix_id = expect_symbol(*suffix)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for text operations".into(),
        found: "symbol handle without store".into(),
    })
}

fn eval_helper_has(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (obj, key) = two_args(args, ExprHelper::Has)?;
    let _obj_id = expect_object(*obj)?;
    let _key_id = expect_symbol(*key)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for object field lookup".into(),
        found: "object handle without store".into(),
    })
}

fn eval_helper_append(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (list, _item) = two_args(args, ExprHelper::Append)?;
    let _list_id = expect_list(*list)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list append".into(),
        found: "list handle without store".into(),
    })
}

fn eval_helper_append_if(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (list, _item, condition) = three_args(args, ExprHelper::AppendIf)?;
    let _list_id = expect_list(*list)?;
    let _condition = expect_bool(*condition)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list append".into(),
        found: "list handle without store".into(),
    })
}

fn eval_helper_merge(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (left, right) = two_args(args, ExprHelper::Merge)?;
    let _left_id = expect_object(*left)?;
    let _right_id = expect_object(*right)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for object merge".into(),
        found: "object handle without store".into(),
    })
}

fn eval_helper_sum(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let list = one_arg(args, ExprHelper::Sum)?;
    let _list_id = expect_list(*list)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list sum".into(),
        found: "list handle without store".into(),
    })
}

fn one_arg(args: &[SlotValue], helper: ExprHelper) -> ExprResult<&SlotValue> {
    if args.len() != 1 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 1,
            actual: args.len(),
        });
    }
    args.first().ok_or(ExprError::StackUnderflow)
}

fn two_args(args: &[SlotValue], helper: ExprHelper) -> ExprResult<(&SlotValue, &SlotValue)> {
    if args.len() != 2 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 2,
            actual: args.len(),
        });
    }
    let left = args.first().ok_or(ExprError::StackUnderflow)?;
    let right = args.get(1).ok_or(ExprError::StackUnderflow)?;
    Ok((left, right))
}

fn three_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue, &SlotValue)> {
    if args.len() != 3 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 3,
            actual: args.len(),
        });
    }
    let first = args.first().ok_or(ExprError::StackUnderflow)?;
    let second = args.get(1).ok_or(ExprError::StackUnderflow)?;
    let third = args.get(2).ok_or(ExprError::StackUnderflow)?;
    Ok((first, second, third))
}
