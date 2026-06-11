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

type UnaryHelper = fn(&SlotValue, &mut ValueStore) -> ExprResult<SlotValue>;
type BinaryHelper = fn(&SlotValue, &SlotValue, &mut ValueStore) -> ExprResult<SlotValue>;

/// Dispatch for helper operations from bytecode evaluation.
pub(crate) fn eval_helper_op_with_store(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => push_unary(stack, store, eval_helper_exists_with_store),
        ExprOp::Length => push_unary(stack, store, eval_helper_length_with_store),
        ExprOp::Empty => push_unary(stack, store, eval_helper_empty_with_store),
        ExprOp::Count => push_unary(stack, store, eval_helper_count_with_store),
        ExprOp::Unique => push_unary(stack, store, eval_helper_unique_with_store),
        ExprOp::Contains => push_binary_bytecode(stack, store, eval_helper_contains_with_store),
        ExprOp::StartsWith => {
            push_binary_bytecode(stack, store, eval_helper_starts_with_with_store)
        }
        ExprOp::EndsWith => push_binary_bytecode(stack, store, eval_helper_ends_with_with_store),
        ExprOp::Has => push_binary_bytecode(stack, store, eval_helper_has_with_store),
        ExprOp::Append => push_binary_bytecode(stack, store, eval_helper_append_with_store),
        ExprOp::AppendIf => push_append_if_bytecode(stack, store),
        ExprOp::Merge => push_binary_bytecode(stack, store, eval_helper_merge_with_store),
        ExprOp::Sum => push_unary(stack, store, eval_helper_sum_with_store),
        _ => unknown_operator(op),
    }
}

fn unknown_operator(op: ExprOp) -> ExprResult<()> {
    Err(crate::ExprError::UnknownOperator {
        op: format!("{op:?}"),
    })
}

fn push_unary(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
    eval: UnaryHelper,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = eval(&value, store)?;
    push_value(stack, result)
}

fn push_binary_bytecode(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
    eval: BinaryHelper,
) -> ExprResult<()> {
    let (right, left) = pop_pair(stack)?;
    let result = eval(&left, &right, store)?;
    push_value(stack, result)
}

fn push_append_if_bytecode(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
) -> ExprResult<()> {
    let (third, second, first) = pop_triple(stack)?;
    let result = eval_helper_append_if_with_store(&first, &second, &third, store)?;
    push_value(stack, result)
}
