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
    let mut stack = ExprStack::new(MAX_EXPRESSION_STACK).unwrap();
    let mut store = ValueStore::new();

    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right == 0);

    stack.push(SlotValue::I64(left)).unwrap();
    stack.push(SlotValue::I64(right)).unwrap();

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    assert!(result.is_err());
    assert!(matches!(result, Err(EngineError::DivisionByZero)));
}

#[kani::proof]
#[kani::unwind(4)]
fn kani_div_by_nonzero_succeeds() {
    let mut stack = ExprStack::new(MAX_EXPRESSION_STACK).unwrap();
    let mut store = ValueStore::new();

    let left: i64 = kani::any();
    let right: i64 = kani::any();
    kani::assume(right != 0);
    kani::assume(left.checked_div(right).is_some());

    stack.push(SlotValue::I64(left)).unwrap();
    stack.push(SlotValue::I64(right)).unwrap();

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    assert!(result.is_ok());
}

#[kani::proof]
#[kani::unwind(4)]
fn kani_div_i64_min_neg_one() {
    let mut stack = ExprStack::new(MAX_EXPRESSION_STACK).unwrap();
    let mut store = ValueStore::new();

    stack.push(SlotValue::I64(i64::MIN)).unwrap();
    stack.push(SlotValue::I64(-1)).unwrap();

    let result = eval_expr_operator(ExprOp::Div, &mut stack, &mut store);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, EngineError::InvalidCompiledWorkflow { .. }));
}
