#![forbid(unsafe_code)]
//! Argument count validation utilities.

use crate::parser::ExprHelper;
use vb_core::SlotValue;

use crate::{ExprError, ExprResult};

/// Shared argument extraction utilities for helpers.
///
/// These are used by both the public API (`eval_helper_with_store`)
/// and bytecode dispatch (`eval_helper_op_with_store`) paths.
pub(crate) fn one_arg(args: &[SlotValue], helper: ExprHelper) -> ExprResult<&SlotValue> {
    if args.len() != 1 {
        return Err(arity_mismatch(helper, 1, args.len()));
    }
    args.first().ok_or(ExprError::StackUnderflow)
}

pub(crate) fn two_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue)> {
    if args.len() != 2 {
        return Err(arity_mismatch(helper, 2, args.len()));
    }
    let left = args.first().ok_or(ExprError::StackUnderflow)?;
    let right = args.get(1).ok_or(ExprError::StackUnderflow)?;
    Ok((left, right))
}

pub(crate) fn three_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue, &SlotValue)> {
    if args.len() != 3 {
        return Err(arity_mismatch(helper, 3, args.len()));
    }
    let first = args.first().ok_or(ExprError::StackUnderflow)?;
    let second = args.get(1).ok_or(ExprError::StackUnderflow)?;
    let third = args.get(2).ok_or(ExprError::StackUnderflow)?;
    Ok((first, second, third))
}

fn arity_mismatch(helper: ExprHelper, expected: usize, actual: usize) -> ExprError {
    ExprError::HelperArityMismatch {
        helper: crate::parser::helper_name(helper).into(),
        expected,
        actual,
    }
}
