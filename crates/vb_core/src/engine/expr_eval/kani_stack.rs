#![forbid(unsafe_code)]
//! Kani harnesses proving `ExprStack` never panics.

use crate::engine::expr_eval::stack::{ExprStack, pop_value, push_value};
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

    let stack = match result {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
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

    let mut stack = match ExprStack::new(capacity) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };

    let result = stack.push(SlotValue::Null);
    assert!(result.is_ok());
    assert_eq!(stack.len(), 1);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_push_overflow_returns_error() {
    let mut stack = match ExprStack::new(1) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    match stack.push(SlotValue::Null) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
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

    let mut stack = match ExprStack::new(4) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    for _ in 0..initial_len {
        match stack.push(SlotValue::Null) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        };
    }

    let result = stack.pop();
    assert!(result.is_ok());
    assert_eq!(stack.len(), initial_len - 1);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_pop_empty_returns_underflow() {
    let stack = match ExprStack::new(4) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
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

    let mut stack = match ExprStack::new(capacity) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };

    match push_value(&mut stack, SlotValue::Null) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    let popped = match pop_value(&mut stack) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    assert_eq!(popped, SlotValue::Null);
}

#[kani::proof]
#[kani::unwind(4)]
fn harness_pop_pair_underflow() {
    let mut stack = match ExprStack::new(4) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    match stack.push(SlotValue::I64(42)) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
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
fn harness_push_to_capacity_then_overflow() {
    let capacity: u8 = kani::any();
    kani::assume(usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE);
    kani::assume(capacity > 0);
    kani::assume(capacity <= 3);

    let mut stack = match ExprStack::new(capacity) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };

    match stack.push(SlotValue::I64(1)) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    if capacity >= 2 {
        match stack.push(SlotValue::I64(2)) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        };
    }
    if capacity >= 3 {
        match stack.push(SlotValue::I64(3)) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        };
    }

    let result = stack.push(SlotValue::Null);
    assert!(result.is_err());
}
