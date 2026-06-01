#![forbid(unsafe_code)]
//! Argument count validation utilities.

use vb_core::value_store::ValueStore;
use vb_core::SlotValue;
use crate::parser::ExprHelper;

use crate::ExprResult;

/// Shared argument extraction utilities for helpers.
///
/// These are used by both the public API (`eval_helper_with_store`)
/// and bytecode dispatch (`eval_helper_op_with_store`) paths.

pub fn one_arg(args: &[SlotValue], helper: ExprHelper) -> ExprResult<&SlotValue> {
    if args.len() != 1 {
        return Err(crate::ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 1,
            actual: args.len(),
        });
    }
    args.first().ok_or(crate::ExprError::StackUnderflow)
}

pub fn two_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue)> {
    if args.len() != 2 {
        return Err(crate::ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 2,
            actual: args.len(),
        });
    }
    let left = args.first().ok_or(crate::ExprError::StackUnderflow)?;
    let right = args.get(1).ok_or(crate::ExprError::StackUnderflow)?;
    Ok((left, right))
}

pub fn three_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue, &SlotValue)> {
    if args.len() != 3 {
        return Err(crate::ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 3,
            actual: args.len(),
        });
    }
    let first = args.first().ok_or(crate::ExprError::StackUnderflow)?;
    let second = args.get(1).ok_or(crate::ExprError::StackUnderflow)?;
    let third = args.get(2).ok_or(crate::ExprError::StackUnderflow)?;
    Ok((first, second, third))
}

/// Re-export pop_pair and pop_triple for bytecode dispatch.
use crate::eval::stack::{pop_pair, pop_triple};

/// Helper to pop two values from stack in correct order.
pub fn pop_pair_pop_triple(
    stack: &mut arrayvec::ArrayVec<SlotValue, vb_core::limits::MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<((SlotValue, SlotValue), (SlotValue, SlotValue, SlotValue))> {
    let (a, b) = pop_pair(stack)?;
    let (c, d, e) = pop_triple(stack)?;
    Ok(((a, b), (c, d, e)))
}
