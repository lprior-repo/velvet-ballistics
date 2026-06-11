#![forbid(unsafe_code)]
//! Kani regression harnesses for vb-xo50x builtin_eval overflow semantics.

use crate::ExprError;
use crate::eval::eval_binary_op;
use crate::lexer::BinaryOp;
use vb_core::SlotValue;

/// vb-xo50x: builtin_eval must report i64::MIN / -1 as IntegerOverflow.
#[kani::proof]
fn kani_builtin_eval_min_neg_one() {
    let left: i64 = kani::any();
    let right: i64 = kani::any();

    kani::assume(left == i64::MIN);
    kani::assume(right == -1);
    kani::cover!(
        left == i64::MIN && right == -1,
        "domain includes i64::MIN / -1"
    );

    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(left), SlotValue::I64(right));

    match result {
        Err(ExprError::IntegerOverflow) => {}
        Err(_) => kani::assert(false, "i64::MIN / -1 must return IntegerOverflow"),
        Ok(_) => kani::assert(false, "i64::MIN / -1 must not succeed"),
    }
}
