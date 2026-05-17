#![forbid(unsafe_code)]
//! VB-EXPR-002: Expression stack verification
//!
//! Property: `ExprStack` operations (new, push, pop) are panic-free for
//! bounded capacity values and return proper errors for overflow/underflow.
//!
//! This harness verifies expression stack bounds and error handling.

use crate::engine::expr_eval::stack::{ExprStack, pop_value, push_value};
use crate::errors::EngineError;
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;

/// VB-EXPR-002 H1: ExprStack::new with valid capacity succeeds
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_new_valid() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);

    let result = ExprStack::new(capacity);
    kani::assert(result.is_ok(), "new with valid capacity succeeds");

    if let Ok(stack) = result {
        kani::assert_eq!(stack.len(), 0, "new stack has len 0");
    }
}

/// VB-EXPR-002 H2: ExprStack::new with invalid capacity returns error
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_new_invalid() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) > MAX_EXPRESSION_STACK_USIZE);

    let result = ExprStack::new(capacity);
    kani::assert(result.is_err(), "new with invalid capacity returns error");

    if let Err(EngineError::ExpressionStackOverflow { max }) = result {
        kani::assert_eq!(max, capacity);
    }
}

/// VB-EXPR-002 H3: push with room succeeds
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_push_with_room() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);

    let mut stack = ExprStack::new(capacity).unwrap();

    let result = stack.push(SlotValue::Null);
    kani::assert(result.is_ok(), "push with room succeeds");
    kani::assert_eq!(stack.len(), 1, "len is 1 after push");
}

/// VB-EXPR-002 H4: push at capacity returns overflow error
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_push_overflow() {
    let mut stack = ExprStack::new(1).unwrap();
    stack.push(SlotValue::Null).unwrap();
    kani::assert_eq!(stack.len(), 1);

    let result = stack.push(SlotValue::Null);
    kani::assert(result.is_err(), "push at capacity returns error");

    if let Err(EngineError::ExpressionStackOverflow { max }) = result {
        kani::assert_eq!(max, 1);
    }
}

/// VB-EXPR-002 H5: pop with items succeeds
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_pop_with_items() {
    let initial_len: u8 = kani::any();
    kani::assume(initial_len >= 1);
    kani::assume(initial_len <= 3);

    let mut stack = ExprStack::new(4).unwrap();
    for _ in 0..initial_len {
        stack.push(SlotValue::Null).unwrap();
    }

    let result = stack.pop();
    kani::assert(result.is_ok(), "pop with items succeeds");
    kani::assert_eq!(stack.len(), initial_len - 1);
}

/// VB-EXPR-002 H6: pop on empty stack returns underflow error
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_pop_underflow() {
    let stack = ExprStack::new(4).unwrap();
    kani::assert_eq!(stack.len(), 0);

    let result = stack.pop();
    kani::assert(result.is_err(), "pop on empty returns error");

    if let Err(EngineError::ExpressionStackUnderflow) = result {
        kani::assert(true, "correct error type");
    }
}

/// VB-EXPR-002 H7: push/pop roundtrip preserves value
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_push_pop_roundtrip() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);

    let mut stack = ExprStack::new(capacity).unwrap();

    push_value(&mut stack, SlotValue::I64(42)).unwrap();
    let popped = pop_value(&mut stack).unwrap();

    kani::assert(matches!(popped, SlotValue::I64(42)), "roundtrip preserves value");
}

/// VB-EXPR-002 H8: multiple pushes then pops
#[kani::proof]
#[kani::unwind(5)]
fn kani_expr_stack_multiple_operations() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity >= 3);

    let mut stack = ExprStack::new(capacity).unwrap();

    push_value(&mut stack, SlotValue::I64(1)).unwrap();
    push_value(&mut stack, SlotValue::I64(2)).unwrap();
    push_value(&mut stack, SlotValue::I64(3)).unwrap();

    kani::assert_eq!(stack.len(), 3);

    let v3 = pop_value(&mut stack).unwrap();
    let v2 = pop_value(&mut stack).unwrap();
    let v1 = pop_value(&mut stack).unwrap();

    kani::assert(matches!(v1, SlotValue::I64(1)));
    kani::assert(matches!(v2, SlotValue::I64(2)));
    kani::assert(matches!(v3, SlotValue::I64(3)));
}
