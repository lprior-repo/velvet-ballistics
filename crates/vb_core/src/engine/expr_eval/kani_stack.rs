#![forbid(unsafe_code)]
//! Kani harnesses proving `ExprStack` never panics.

use crate::engine::expr_eval::stack::{pop_value, push_value, ExprStack};
use crate::errors::EngineError;
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;

#[kani::proof]
#[kani::unwind(4)]
fn harness_new_valid_capacity() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);

    let result = ExprStack::new(capacity);
    assert!(result.is_ok());

    let stack = result.unwrap();
    assert_eq!(stack.len(), 0);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_new_invalid_capacity() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) > MAX_EXPRESSION_STACK_USIZE);

    let result = ExprStack::new(capacity);
    assert!(result.is_err());

    match result {
        Err(EngineError::ExpressionStackOverflow { max }) => {
            assert_eq!(max, capacity);
        }
        _ => unreachable!(),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_push_with_room() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);

    let mut stack = ExprStack::new(capacity).unwrap();

    let result = stack.push(SlotValue::Null);
    assert!(result.is_ok());
    assert_eq!(stack.len(), 1);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_push_overflow_returns_error() {
    let mut stack = ExprStack::new(1).unwrap();
    stack.push(SlotValue::Null).unwrap();
    assert_eq!(stack.len(), 1);

    let result = stack.push(SlotValue::Null);
    assert!(result.is_err());

    match result {
        Err(EngineError::ExpressionStackOverflow { max }) => {
            assert_eq!(max, 1);
        }
        _ => unreachable!("expected overflow error"),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_pop_with_items() {
    let initial_len: u8 = kani::any();
    kani::assume(initial_len >= 1);
    kani::assume(initial_len <= 3);

    let mut stack = ExprStack::new(4).unwrap();
    for _ in 0..initial_len {
        stack.push(SlotValue::Null).unwrap();
    }

    let result = stack.pop();
    assert!(result.is_ok());
    assert_eq!(stack.len(), initial_len - 1);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_pop_empty_returns_underflow() {
    let stack = ExprStack::new(4).unwrap();
    assert_eq!(stack.len(), 0);
    let mut stack = stack;

    let result = stack.pop();
    assert!(result.is_err());

    match result {
        Err(EngineError::ExpressionStackUnderflow) => {}
        _ => unreachable!("expected underflow error"),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_push_pop_roundtrip() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);

    let mut stack = ExprStack::new(capacity).unwrap();

    push_value(&mut stack, SlotValue::Null).unwrap();
    let popped = pop_value(&mut stack).unwrap();
    assert_eq!(popped, SlotValue::Null);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_pop_pair_underflow() {
    let mut stack = ExprStack::new(4).unwrap();
    stack.push(SlotValue::I64(42)).unwrap();
    assert_eq!(stack.len(), 1);

    let result = pop_value(&mut stack);
    assert!(result.is_ok());
    assert_eq!(stack.len(), 0);

    let result2 = pop_value(&mut stack);
    assert!(result2.is_err());
    match result2 {
        Err(EngineError::ExpressionStackUnderflow) => {}
        _ => unreachable!("expected underflow"),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_push_respects_capacity_exactly() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);

    let mut stack = ExprStack::new(capacity).unwrap();

    for i in 0..capacity {
        let result = stack.push(SlotValue::I64(i64::from(i)));
        assert!(result.is_ok());
        assert_eq!(stack.len(), i + 1);
    }

    let result = stack.push(SlotValue::Null);
    assert!(result.is_err());
    match result {
        Err(EngineError::ExpressionStackOverflow { max }) => {
            assert_eq!(max, capacity);
        }
        _ => unreachable!("expected overflow"),
    }
}