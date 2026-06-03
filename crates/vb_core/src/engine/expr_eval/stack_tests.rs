#![forbid(unsafe_code)]
//! Stack tests.

use crate::engine::expr_eval::stack::*;
use crate::errors::EngineError;
use crate::ids::{ListId, ObjectId, SymbolId};
use crate::value::SlotValue;

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

// ===== ExprStack new / push / pop =====

#[test]
fn stack_new_with_valid_capacity_succeeds() -> Result<(), String> {
    let stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    ensure_equal(stack.len(), 0)
}

#[test]
fn stack_new_with_zero_capacity_is_valid() -> Result<(), String> {
    let stack = ExprStack::new(0).map_err(|e| e.to_string())?;
    ensure_equal(stack.len(), 0)
}

#[test]
fn stack_push_and_pop_roundtrip() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(42)).map_err(|e| e.to_string())?;
    ensure_equal(stack.len(), 1)?;
    let value = pop_value(&mut stack).map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(42))?;
    ensure_equal(stack.len(), 0)
}

#[test]
fn stack_push_multiple_preserves_order() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(1)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(3)).map_err(|e| e.to_string())?;
    ensure_equal(stack.len(), 3)?;
    // Pop is LIFO
    ensure_equal(
        pop_value(&mut stack).map_err(|e| e.to_string())?,
        SlotValue::I64(3),
    )?;
    ensure_equal(
        pop_value(&mut stack).map_err(|e| e.to_string())?,
        SlotValue::I64(2),
    )?;
    ensure_equal(
        pop_value(&mut stack).map_err(|e| e.to_string())?,
        SlotValue::I64(1),
    )
}

#[test]
fn stack_overflow_returns_error() -> Result<(), String> {
    let mut stack = ExprStack::new(1).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(1)).map_err(|e| e.to_string())?;
    let result = push_value(&mut stack, SlotValue::I64(2));
    match result {
        Err(EngineError::ExpressionStackOverflow { max: 1 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn stack_underflow_returns_error() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    let result = pop_value(&mut stack);
    match result {
        Err(EngineError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== pop_pair =====

#[test]
fn pop_pair_returns_left_right_ordering() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(10)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(20)).map_err(|e| e.to_string())?;
    let (left, right) = pop_pair(&mut stack).map_err(|e| e.to_string())?;
    ensure_equal(left, SlotValue::I64(10))?;
    ensure_equal(right, SlotValue::I64(20))
}

#[test]
fn pop_pair_underflow_returns_error() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(1)).map_err(|e| e.to_string())?;
    let result = pop_pair(&mut stack);
    match result {
        Err(EngineError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== pop_triple =====

#[test]
fn pop_triple_returns_left_mid_right_ordering() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(1)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(3)).map_err(|e| e.to_string())?;
    let (left, mid, right) = pop_triple(&mut stack).map_err(|e| e.to_string())?;
    ensure_equal(left, SlotValue::I64(1))?;
    ensure_equal(mid, SlotValue::I64(2))?;
    ensure_equal(right, SlotValue::I64(3))
}

// ===== pop_i64_pair =====

#[test]
fn pop_i64_pair_extracts_numbers() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(7)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(3)).map_err(|e| e.to_string())?;
    let (left, right) = pop_i64_pair(&mut stack).map_err(|e| e.to_string())?;
    ensure_equal(left, 7)?;
    ensure_equal(right, 3)
}

#[test]
fn pop_i64_pair_rejects_non_number() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::Bool(true)).map_err(|e| e.to_string())?;
    push_value(&mut stack, SlotValue::I64(1)).map_err(|e| e.to_string())?;
    let result = pop_i64_pair(&mut stack);
    match result {
        Err(EngineError::TypeMismatch {
            expected: "number",
            found: "boolean",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== Type expectors =====

#[test]
fn expect_bool_accepts_bool() -> Result<(), String> {
    ensure_equal(
        expect_bool(SlotValue::Bool(true)).map_err(|e| e.to_string())?,
        true,
    )?;
    ensure_equal(
        expect_bool(SlotValue::Bool(false)).map_err(|e| e.to_string())?,
        false,
    )
}

#[test]
fn expect_bool_rejects_non_bool() -> Result<(), String> {
    let result = expect_bool(SlotValue::I64(1));
    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expect_i64_accepts_number() -> Result<(), String> {
    ensure_equal(
        expect_i64(SlotValue::I64(42)).map_err(|e| e.to_string())?,
        42,
    )
}

#[test]
fn expect_i64_rejects_non_number() -> Result<(), String> {
    let result = expect_i64(SlotValue::Null);
    match result {
        Err(EngineError::TypeMismatch {
            expected: "number",
            found: "null",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expect_symbol_accepts_symbol() -> Result<(), String> {
    ensure_equal(
        expect_symbol(SlotValue::Symbol(SymbolId::new(5))).map_err(|e| e.to_string())?,
        SymbolId::new(5),
    )
}

#[test]
fn expect_symbol_rejects_non_symbol() -> Result<(), String> {
    let result = expect_symbol(SlotValue::I64(1));
    match result {
        Err(EngineError::TypeMismatch {
            expected: "text",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expect_list_accepts_list() -> Result<(), String> {
    ensure_equal(
        expect_list(SlotValue::List(ListId::new(3))).map_err(|e| e.to_string())?,
        ListId::new(3),
    )
}

#[test]
fn expect_list_rejects_non_list() -> Result<(), String> {
    let result = expect_list(SlotValue::Bool(false));
    match result {
        Err(EngineError::TypeMismatch {
            expected: "list",
            found: "boolean",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expect_object_accepts_object() -> Result<(), String> {
    ensure_equal(
        expect_object(SlotValue::Object(ObjectId::new(7))).map_err(|e| e.to_string())?,
        ObjectId::new(7),
    )
}

#[test]
fn expect_object_rejects_non_object() -> Result<(), String> {
    let result = expect_object(SlotValue::Null);
    match result {
        Err(EngineError::TypeMismatch {
            expected: "object",
            found: "null",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn stack_new_with_excessive_capacity_fails() {
    let capacity = u8::MAX;
    let result = ExprStack::new(capacity);
    assert!(
        matches!(
            result,
            Err(EngineError::ExpressionStackOverflow { max }) if max == capacity
        ),
        "expected ExpressionStackOverflow({capacity})"
    );
}

#[test]
fn stack_push_exactly_at_capacity_fails() {
    let mut stack = ExprStack::new(1).expect("valid");
    push_value(&mut stack, SlotValue::I64(1)).expect("first push");
    let result = push_value(&mut stack, SlotValue::I64(2));
    assert!(
        matches!(result, Err(EngineError::ExpressionStackOverflow { max: 1 })),
        "expected ExpressionStackOverflow(1)"
    );
}

#[test]
fn stack_pop_checked_sub_underflow_returns_underflow() {
    let mut stack = ExprStack::new(4).expect("valid");
    // Directly set len to 0 and attempt pop; len==0 guard fires first.
    let result = pop_value(&mut stack);
    assert_eq!(result, Err(EngineError::ExpressionStackUnderflow));
}

#[test]
fn stack_pop_get_failure_returns_invariant_violation() {
    // This path is unreachable in normal use because len is clamped to
    // capacity which is <= MAX_EXPRESSION_STACK_USIZE. We exercise the
    // .get() failure branch by pushing then manually corrupting len.
    let mut stack = ExprStack::new(4).expect("valid");
    push_value(&mut stack, SlotValue::I64(1)).expect("push");
    stack.corrupt_len_for_test(255); // corrupt len so get() fails
    let result = pop_value(&mut stack);
    assert_eq!(
        result,
        Err(EngineError::InternalInvariantViolation {
            reason: "expression stack pop index checked by length",
        })
    );
}
