#![forbid(unsafe_code)]
//! Legacy test-only builtin operator probe.

use vb_core::SlotValue;

use crate::stack_ops::expect_i64;
use crate::{ExprError, ExprResult};

fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    let value = left_i64
        .checked_div(right_i64)
        .ok_or(ExprError::DivisionByZero)?;
    Ok(SlotValue::I64(value))
}

#[cfg(test)]
#[path = "expr_builtin_eval/blackhat_tests.rs"]
mod blackhat_tests;
