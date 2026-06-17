#![forbid(unsafe_code)]
//! PO-KANI-004: Evaluation stack bound verification (extends existing kani_expr_stack.rs)
//! Requirement: C-EVAL-1
//!
//! Production target: ArrayVec<SlotValue, 64> stack operations in crate::eval
//!
//! Verifies:
//! - try_push on full stack returns an error (StackOverflow via ExprError)
//! - try_push on non-full stack succeeds
//! - pop on empty stack returns None / error
//! - check_expr_stack_bound rejects oversized bytecode
//!
//! Note: The stack push/pop functions in eval.rs are private. This harness
//! verifies the ArrayVec behavior directly and checks the public
//! check_expr_stack_bound function.

use crate::ExprError;
use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::{ConstIdx, ExprOp, SlotValue};

/// Replicates the eval push_value logic: try_push + StackOverflow error mapping.
fn stack_push(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> Result<(), ExprError> {
    stack
        .try_push(value)
        .map_err(|_| ExprError::StackOverflow { max: 64 })
}

/// Replicates the eval pop_value logic: pop + StackUnderflow error mapping.
fn stack_pop(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> Result<SlotValue, ExprError> {
    stack.pop().ok_or(ExprError::StackUnderflow)
}

/// PO-KANI-004 H1: push on empty stack succeeds.
#[kani::proof]
fn check_push_on_empty_stack_succeeds() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();
    let result = stack_push(&mut stack, SlotValue::Bool(true));

    kani::assert(result.is_ok(), "push on empty stack must succeed");
    kani::assert(stack.len() == 1, "stack must have 1 element after push");
}

/// PO-KANI-004 H2: push on full stack returns StackOverflow.
#[kani::proof]
#[kani::unwind(65)]
fn check_push_on_full_stack_returns_overflow() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();
    // Fill to capacity
    for _ in 0..MAX_EXPRESSION_STACK_USIZE {
        let _ = stack_push(&mut stack, SlotValue::Bool(true));
    }

    kani::assert(stack.len() == MAX_EXPRESSION_STACK_USIZE,
        "stack must be full",
    );

    // Push one more — must fail
    let result = stack_push(&mut stack, SlotValue::Bool(false));

    kani::assert(result.is_err(), "push on full stack must return error");

    match result {
        Err(ExprError::StackOverflow { max }) => {
            , "push on full stack must return error");

    match result {
        Err(ExprError::StackOverflow { max }) => {
            kani::assert(max == 64, "max must be 64");
        }
        Err(_) => {
            // Any typed error
        }
        Ok(_) => {
            kani::assert(false, "push on full stack must fail");
        }
    }

    // Stack must not grow beyond capacity
    kani::assert(
        stack.len() == MAX_EXPRESSION_STACK_USIZE,
        "stack must not grow on failed push",
    );
}

/// PO-KANI-004 H3: pop on empty stack returns StackUnderflow.
#[kani::proof]
fn check_pop_on_empty_stack_returns_underflow() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();

    let result = stack_pop(&mut stack);

    kani::assert(result.is_err(), "pop on empty stack must return error");

    match result {
        Err(ExprError::StackUnderflow) => {
            // Correct
        }
        Err(_) => {
            // Any typed error
        }
        Ok(_) => {
            , "pop on empty stack must return error");

    match result {
        Err(ExprError::StackUnderflow) => {
            // Correct
        }
        Err(_) => {
            // Any typed error
        }
        Ok(_) => {
            kani::assert(false, "pop on empty stack must fail");
        }
    }
}

/// PO-KANI-004 H4: push then pop preserves value.
#[kani::proof]
fn check_push_pop_roundtrip() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();

    // Push a value
    let _ = stack_push(&mut stack, SlotValue::I64(42));
    // Pop it back
    let result = stack_pop(&mut stack);

    kani::assert(result.is_ok(), "pop after push must succeed");

    match result {
        Ok(SlotValue::I64(v)) => {
            , "pop after push must succeed");

    match result {
        Ok(SlotValue::I64(v)) => {
            kani::assert(v == 42, "popped value must match pushed value");
        }
        Ok(_) => {}
        Err(_) => {
            kani::assert(false, "pop after push must succeed");
        }
    }

    // Stack must be empty after pop
    kani::assert(stack.is_empty(), "stack must be empty after push+pop");
}

/// PO-KANI-004 H5: push many values up to capacity and pop all.
#[kani::proof]
#[kani::unwind(65)]
fn check_push_many_pop_all() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();
    let n: usize = kani::any();
    kani::assume(n <= MAX_EXPRESSION_STACK_USIZE);

    // Push n values
    for i in 0..n {
        let val = i64::try_from(i).unwrap_or(0);
        let result = stack_push(&mut stack, SlotValue::I64(val));
        kani::assert(result.is_ok(), "push within capacity must succeed");
    }

    kani::assert(stack.len() == n,
        "stack must have n elements after n pushes",
    );

    // Pop all n values
    for _ in 0..n {
        let result = stack_pop(&mut stack);
        kani::assert(result.is_ok(), "pop of pushed value must succeed");
    }

    // Stack must be empty
    kani::assert(stack.is_empty(),
        "stack must be empty after n pushes + n pops",
    );
}

/// PO-KANI-004 H6: compile-time check_expr_stack_bound catches oversized bytecode.
#[kani::proof]
#[kani::unwind(66)]
fn check_expr_stack_bound_oversized() {
    // 65 LoadConst ops — max stack depth = 65 > 64
    let mut ops = Vec::new();
    for i in 0..65u16 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
    }

    let result = crate::bytecode::check_expr_stack_bound(&ops);

    kani::assert(result.is_err(), "65 LoadConst ops must be rejected");

    match result {
        Err(ExprError::StackOverflow { max }) => {
            , "65 LoadConst ops must be rejected");

    match result {
        Err(ExprError::StackOverflow { max }) => {
            kani::assert(max == 64, "max must be 64");
        }
        Err(_) => {
            // Any error
        }
        Ok(_) => {
            kani::assert(false, "65 LoadConst ops must fail");
        }
    }
}

/// PO-KANI-004 H7: pop_pair on insufficient stack returns StackUnderflow.
#[kani::proof]
fn check_pop_pair_underflow() {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();

    // Push only one value, then try to pop a pair (needs two)
    let _ = stack_push(&mut stack, SlotValue::I64(1));

    // pop_pair logic: pop right, then pop left
    let right = stack_pop(&mut stack);
    kani::assert(right.is_ok(), "first pop should succeed");

    let left = stack_pop(&mut stack);
    kani::assert(left.is_err(),
        "second pop on single-element stack must fail",
    );
    match left {
        Err(ExprError::StackUnderflow) => {}
        Err(e) => {
            kani::assert(matches!(e, ExprError::StackUnderflow),
                "second pop on single-element stack must return StackUnderflow",
            );
        }
        Ok(_) => {
            ,
                "second pop on single-element stack must return StackUnderflow",
            );
        }
        Ok(_) => {
            kani::assert(false, "pop on insufficient stack must fail");
        }
    }
}
