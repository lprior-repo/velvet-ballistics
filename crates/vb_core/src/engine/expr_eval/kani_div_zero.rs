#![forbid(unsafe_code)]
//! Kani harnesses proving division by zero returns an error, not a panic.
//!
//! Contract from master doc Section 46: "division by zero" is an explicit error
//! variant requiring proof.

use crate::engine::expr_eval::ops::eval_expr_operator;
use crate::engine::expr_eval::stack::ExprStack;
use crate::errors::EngineError;
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::ExprOp;

#[kani::proof]
#[kani::unwind(4)]
fn kani_div_by_zero_returns_error() {
    let mut stack = match ExprStack::new(MAX_EXPRESSION_STACK) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    let mut store = ValueStore::new();

    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right == 0);

    match stack.push(SlotValue::I64(left)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    match stack.push(SlotValue::I64(right)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    kani::assert(result.is_err(, "assertion failed"), "kani harness assertion");
    kani::assert(matches!(result, Err(EngineError::DivisionByZero), "assertion failed"));
}

#[kani::proof]
#[kani::unwind(4)]
fn kani_div_by_nonzero_succeeds() {
    let mut stack = match ExprStack::new(MAX_EXPRESSION_STACK) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    let mut store = ValueStore::new();

    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right != 0);
    kani::assume(left.checked_div(right).is_some());

    match stack.push(SlotValue::I64(left)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    match stack.push(SlotValue::I64(right)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    kani::assert(result.is_ok(, "assertion failed"), "kani harness assertion");
}

#[kani::proof]
#[kani::unwind(4)]
fn kani_div_i64_min_neg_one() {
    let mut stack = match ExprStack::new(MAX_EXPRESSION_STACK) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    let mut store = ValueStore::new();

    match stack.push(SlotValue::I64(i64::MIN)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    match stack.push(SlotValue::I64(-1)) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    kani::assert(result.is_err(, "assertion failed"), "kani harness assertion");
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            kani::assume(false);
            loop {}
        }
    };
    kani::assert(matches!(err, EngineError::InvalidCompiledWorkflow { .. }, "assertion failed"));
}
