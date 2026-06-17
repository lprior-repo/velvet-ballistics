#![forbid(unsafe_code)]
//! vb-xo50x: active i64 division error-taxonomy verification.
//!
//! Production targets: `crate::eval::eval_binary_op` and its active i64 helper.

use crate::ExprError;
use crate::eval::{eval_binary_op, eval_i64_div_values};
use crate::lexer::BinaryOp;
use vb_core::SlotValue;

#[kani::proof]
fn kani_builtin_eval_min_neg_one() {
    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(left == i64::MIN);
    kani::assume(right == -1);
    kani::cover!(
        left == i64::MIN && right == -1,
        "i64::MIN divided by -1 domain is reachable"
    );

    let result = eval_i64_div_values(left, right);

    kani::assert(matches!(result, Err(ExprError::IntegerOverflow)),
        "active i64::MIN / -1 must return IntegerOverflow",
    );
}

#[kani::proof]
fn kani_builtin_eval_zero_divisor_partition() {
    let left: i64 = kani::any();
    let result = eval_i64_div_values(left, 0);

    kani::assert(matches!(result, Err(ExprError::DivisionByZero)),
        "zero divisor must return DivisionByZero",
    );
}

#[kani::proof]
fn kani_builtin_eval_representable_i64_partition() {
    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right != 0);
    kani::assume(!(left == i64::MIN && right == -1));

    let result = eval_i64_div_values(left, right);

    match result {
        Ok(SlotValue::I64(_)) => {}
        Ok(_) => ,
        "zero divisor must return DivisionByZero",
    );
}

#[kani::proof]
fn kani_builtin_eval_representable_i64_partition() {
    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right != 0);
    kani::assume(!(left == i64::MIN && right == -1));

    let result = eval_i64_div_values(left, right);

    match result {
        Ok(SlotValue::I64(_)) => {}
        Ok(_) => kani::assert(false, "i64 division must return an i64 slot"),
        Err(_) => kani::assert(false, "representable nonzero i64 division must succeed"),
    }
}

#[kani::proof]
fn kani_builtin_eval_bounded_i16_quotient_matches_checked_div() {
    let left_i16: i16 = kani::any();
    let right_i16: i16 = kani::any();
    kani::assume(right_i16 != 0);

    let left = i64::from(left_i16);
    let right = i64::from(right_i16);
    let result = eval_i64_div_values(left, right);

    match (result, left.checked_div(right)) {
        (Ok(SlotValue::I64(actual)), Some(expected)) => ) {
        (Ok(SlotValue::I64(actual)), Some(expected)) => kani::assert(
            actual == expected,
            "bounded i16 quotient must match checked_div",
        ),
        _ => kani::assert(false, "bounded nonzero i16 division must succeed"),
    }
}

#[kani::proof]
fn kani_builtin_eval_public_i64_partition_bridge() {
    let left: i64 = kani::any();
    let right: i64 = kani::any();
    let public_result = eval_binary_op(BinaryOp::Div, SlotValue::I64(left), SlotValue::I64(right));

    if right == 0 {
        kani::assert(matches!(public_result, Err(ExprError::DivisionByZero)),
            "public i64 division must preserve zero-divisor classification",
        );
    } else if left == i64::MIN && right == -1 {
        kani::assert(matches!(public_result, Err(ExprError::IntegerOverflow)),
            "public i64 division must preserve overflow classification",
        );
    } else {
        kani::assert(matches!(public_result, Ok(SlotValue::I64(_))),
            "public i64 division must return a representable i64 quotient",
        );
    }
}
