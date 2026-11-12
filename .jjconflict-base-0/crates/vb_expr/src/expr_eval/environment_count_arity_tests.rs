#![forbid(unsafe_code)]
//! Regression tests for helper arity and validation precedence.
//! Uses the actual `eval_helper` API from expr_eval.

use super::eval_helper;
use crate::parser::ExprHelper;
use crate::{ExprError, ExprResult};
use vb_core::SlotValue;

#[test]
fn helper_count_rejects_zero_args() -> ExprResult<()> {
    let result = eval_helper(ExprHelper::Count, &[]);
    assert!(result.is_err());
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "count".into(),
            expected: 1,
            actual: 0
        })
    );
    Ok(())
}

#[test]
fn helper_count_rejects_too_many_args() -> ExprResult<()> {
    let args = [SlotValue::Bool(true), SlotValue::I64(1)];
    let result = eval_helper(ExprHelper::Count, &args);
    assert!(result.is_err());
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "count".into(),
            expected: 1,
            actual: 2
        })
    );
    Ok(())
}

#[test]
fn helper_count_accepts_one_arg() -> ExprResult<()> {
    let args = [SlotValue::Null];
    let result = eval_helper(ExprHelper::Count, &args);
    assert!(result.is_ok());
    assert_eq!(result?, SlotValue::I64(0));
    Ok(())
}

#[test]
fn helper_length_rejects_zero_args() -> ExprResult<()> {
    let result = eval_helper(ExprHelper::Length, &[]);
    assert!(result.is_err());
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "length".into(),
            expected: 1,
            actual: 0
        })
    );
    Ok(())
}
