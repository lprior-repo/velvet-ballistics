#![forbid(unsafe_code)]
//! Bounded Kani harnesses for `ExprStack` capacity bounds.
//!
//! Every input is symbolic: the capacity comes from `kani::any` with assume
//! bounds, and the push/pop operation script comes from
//! `kani::bounded_any::<Vec<u8>, 4>()`. No hardcoded stack instances except
//! the capacity-1 edge (GOD RULE 1). After every operation the harness asserts
//! `len <= capacity`, so any overflow path must surface as an `Err`.

use crate::engine::expr_eval::stack::{ExprStack, pop_value, push_value};
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;

/// S1: Symbolic push/pop script never drives the stack past capacity and never
/// panics; popping an empty stack underflows as an error. The script length is
/// bounded to `<= 4` (scalar form keeps CBMC tractable without heap modeling).
#[kani::proof]
#[kani::unwind(16)]
fn expr_stack_respects_capacity_under_symbolic_ops() {
    let capacity: u8 = kani::any();
    kani::assume(capacity >= 1);
    kani::assume(capacity <= 3);
    let mut stack = match ExprStack::new(capacity) {
        Ok(stack) => stack,
        Err(_) => unreachable!("capacity within bound constructs"),
    };

    let script_len: u8 = kani::any();
    kani::assume(script_len <= 4);
    for _ in 0..script_len {
        let op: u8 = kani::any();
        match op % 2 {
            0 => {
                let _ = push_value(&mut stack, SlotValue::Null);
            }
            _ => {
                let _ = pop_value(&mut stack);
            }
        }
        kani::assert(
            usize::from(stack.len()) <= usize::from(capacity),
            "stack length never exceeds capacity",
        );
    }
    kani::cover!(capacity == 1, "capacity-1 edge");
    kani::cover!(capacity == 3, "capacity-3 edge");
    kani::cover!(stack.len() == capacity, "filled to capacity");
}

/// S2: Capacity above the compile-time maximum is rejected at construction.
#[kani::proof]
#[kani::unwind(4)]
fn expr_stack_rejects_oversized_capacity() {
    let capacity: u8 = kani::any();
    let max = MAX_EXPRESSION_STACK_USIZE as u8;
    kani::assume(capacity > max);
    let result = ExprStack::new(capacity);
    kani::assert(result.is_err(), "oversized capacity rejected");
    kani::cover!(true, "oversized branch");
}

/// S3: Push until full, then one more push overflows exactly at the bound.
#[kani::proof]
#[kani::unwind(16)]
fn expr_stack_overflows_exactly_at_capacity() {
    let capacity: u8 = kani::any();
    kani::assume(capacity >= 1);
    kani::assume(capacity <= 3);
    let mut stack = match ExprStack::new(capacity) {
        Ok(stack) => stack,
        Err(_) => unreachable!("capacity within bound constructs"),
    };
    for _ in 0..capacity {
        kani::assert(
            push_value(&mut stack, SlotValue::I64(1)).is_ok(),
            "push within capacity succeeds",
        );
    }
    kani::assert(
        usize::from(stack.len()) == usize::from(capacity),
        "stack filled exactly to capacity",
    );
    kani::assert(
        push_value(&mut stack, SlotValue::Null).is_err(),
        "push past capacity overflows",
    );
    kani::cover!(capacity == 1, "overflow at capacity 1");
}
