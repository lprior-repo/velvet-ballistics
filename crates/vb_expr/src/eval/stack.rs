#![forbid(unsafe_code)]
//! Stack push/pop operations.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::SlotValue;

use crate::ExprResult;

use super::MAX_EXPRESSION_STACK;

pub fn push_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> ExprResult<()> {
    stack.try_push(value).map_err(|_| crate::ExprError::StackOverflow {
        max: MAX_EXPRESSION_STACK,
    })
}

pub fn pop_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<SlotValue> {
    stack.pop().ok_or(crate::ExprError::StackUnderflow)
}

pub fn pop_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue)> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

pub fn pop_triple(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue, SlotValue)> {
    let third = pop_value(stack)?;
    let second = pop_value(stack)?;
    let first = pop_value(stack)?;
    Ok((first, second, third))
}
