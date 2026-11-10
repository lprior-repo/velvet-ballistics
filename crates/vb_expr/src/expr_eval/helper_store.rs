#![forbid(unsafe_code)]
//! Store-aware helper dispatch for expression evaluation.

use vb_core::value_store::ValueStore;
use vb_core::{ExprOp, SlotValue};

use super::helper_store_values::*;
use super::vm_stack::{VmStack, pop_pair, pop_value, push_value};
use crate::parser::ExprHelper;
use crate::{ExprError, ExprResult};
use vb_core::{Taint, value::join_taint};

pub(super) fn eval_helper_op_with_store(
    op: ExprOp,
    stack: &mut VmStack,
    store: &mut ValueStore,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => push_unary_stack(op, stack, store, eval_helper_exists_with_store),
        ExprOp::Length => push_unary_stack(op, stack, store, eval_helper_length_with_store),
        ExprOp::Empty => push_unary_stack(op, stack, store, eval_helper_empty_with_store),
        ExprOp::Count => push_unary_stack(op, stack, store, eval_helper_count_with_store),
        ExprOp::Unique => push_unary_stack(op, stack, store, eval_helper_unique_with_store),
        ExprOp::Contains => push_binary_stack(op, stack, store, eval_helper_contains_with_store),
        ExprOp::StartsWith => {
            push_binary_stack(op, stack, store, eval_helper_starts_with_with_store)
        }
        ExprOp::EndsWith => push_binary_stack(op, stack, store, eval_helper_ends_with_with_store),
        ExprOp::Has => push_binary_stack(op, stack, store, eval_helper_has_with_store),
        ExprOp::Append => push_append_stack(stack, store, false),
        ExprOp::AppendIf => push_append_stack(stack, store, true),
        ExprOp::Merge => push_binary_stack(op, stack, store, eval_helper_merge_with_store),
        ExprOp::Sum => push_unary_stack(op, stack, store, eval_helper_sum_with_store),
        _ => Err(ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}

fn push_unary_stack(
    op: ExprOp,
    stack: &mut VmStack,
    store: &mut ValueStore,
    helper: fn(&SlotValue, &mut ValueStore) -> ExprResult<SlotValue>,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = helper(&value, store)?;
    stack.join_operand_taint(helper_read_taint(op, value, None, store)?);
    push_value(stack, result)
}

fn push_binary_stack(
    op: ExprOp,
    stack: &mut VmStack,
    store: &mut ValueStore,
    helper: fn(&SlotValue, &SlotValue, &mut ValueStore) -> ExprResult<SlotValue>,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    let result = helper(&left, &right, store)?;
    stack.join_operand_taint(helper_read_taint(op, left, Some(right), store)?);
    push_value(stack, result)
}

/// Evaluates helper functions with full `ValueStore` access.
pub fn eval_helper_with_store(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => eval_helper_exists_with_store(one_arg(args, helper)?, store),
        ExprHelper::Length => eval_helper_length_with_store(one_arg(args, helper)?, store),
        ExprHelper::Empty => eval_helper_empty_with_store(one_arg(args, helper)?, store),
        ExprHelper::Count => eval_helper_count_with_store(one_arg(args, helper)?, store),
        ExprHelper::Unique => eval_helper_unique_with_store(one_arg(args, helper)?, store),
        ExprHelper::Contains => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_contains_with_store(left, right, store)
        }
        ExprHelper::StartsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_starts_with_with_store(left, right, store)
        }
        ExprHelper::EndsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_ends_with_with_store(left, right, store)
        }
        ExprHelper::Has => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_has_with_store(left, right, store)
        }
        ExprHelper::Append => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_append_with_store(left, right, store)
        }
        ExprHelper::AppendIf => {
            let (first, second, third) = three_args(args, helper)?;
            eval_helper_append_if_with_store(first, second, third, store)
        }
        ExprHelper::Merge => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_merge_with_store(left, right, store)
        }
        ExprHelper::Sum => eval_helper_sum_with_store(one_arg(args, helper)?, store),
    }
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

fn helper_read_taint(
    op: ExprOp,
    value: SlotValue,
    right: Option<SlotValue>,
    store: &ValueStore,
) -> ExprResult<Taint> {
    let SlotValue::List(id) = value else {
        return Ok(Taint::Clean);
    };
    match op {
        ExprOp::Sum | ExprOp::Unique => {
            let entries =
                store
                    .list_items_with_taint(id)
                    .map_err(|_| ExprError::InvalidReference {
                        reference: format!("list:{id:?}"),
                    })?;
            Ok(entries.fold(Taint::Clean, |taint, (_, item)| join_taint(taint, item)))
        }
        ExprOp::Contains => {
            let needle = right.ok_or(ExprError::StackUnderflow)?;
            store
                .list_contains_with_taint(id, needle)
                .map(|(_, taint)| taint)
                .map_err(|_| ExprError::InvalidReference {
                    reference: format!("list:{id:?}"),
                })
        }
        _ => Ok(Taint::Clean),
    }
}
fn push_append_stack(
    stack: &mut VmStack,
    store: &mut ValueStore,
    conditional: bool,
) -> ExprResult<()> {
    let condition = if conditional {
        Some(stack.pop_tainted()?)
    } else {
        None
    };
    let (item, item_taint) = stack.pop_tainted()?;
    let (list, list_taint) = stack.pop_tainted()?;
    let result = super::helper_store_lists::eval_append_tainted(
        &list,
        &item,
        item_taint,
        condition.as_ref().map(|(value, _)| value),
        store,
    )?;
    let mut taint = join_taint(list_taint, item_taint);
    if let Some((_, condition_taint)) = condition {
        taint = join_taint(taint, condition_taint);
    }
    stack.push_tainted(result, taint)
}
