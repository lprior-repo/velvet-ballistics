#![forbid(unsafe_code)]
//! Bytecode-level helper dispatch.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::value_store::ValueStore;
use vb_core::{ExprOp, SlotValue};

use crate::ExprResult;

use super::super::impls::{
    eval_helper_append_if_with_store, eval_helper_append_with_store,
    eval_helper_contains_with_store, eval_helper_count_with_store, eval_helper_empty_with_store,
    eval_helper_ends_with_with_store, eval_helper_exists_with_store, eval_helper_has_with_store,
    eval_helper_length_with_store, eval_helper_merge_with_store,
    eval_helper_starts_with_with_store, eval_helper_sum_with_store, eval_helper_unique_with_store,
};
use crate::eval::stack::{pop_pair, pop_triple, pop_value, push_value};

/// Dispatch for helper operations from bytecode evaluation.
pub fn eval_helper_op_with_store(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => {
            let value = pop_value(stack)?;
            let result = eval_helper_exists_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Length => {
            let value = pop_value(stack)?;
            let result = eval_helper_length_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Empty => {
            let value = pop_value(stack)?;
            let result = eval_helper_empty_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Count => {
            let value = pop_value(stack)?;
            let result = eval_helper_count_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Unique => {
            let value = pop_value(stack)?;
            let result = eval_helper_unique_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Contains => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_contains_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::StartsWith => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_starts_with_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::EndsWith => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_ends_with_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Has => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_has_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Append => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_append_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::AppendIf => {
            let (third, second, first) = pop_triple(stack)?;
            let result = eval_helper_append_if_with_store(&first, &second, &third, store)?;
            push_value(stack, result)
        }
        ExprOp::Merge => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_merge_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Sum => {
            let value = pop_value(stack)?;
            let result = eval_helper_sum_with_store(&value, store)?;
            push_value(stack, result)
        }
        _ => Err(crate::ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}
