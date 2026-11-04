//! Edge case property tests designed to kill surviving mutants.
//!
//! These tests target specific operator boundaries and type interactions
//! that mutation testing revealed as weakly covered.

use crate::eval::{BinaryOp, eval_binary_op};
use vb_core::{FiniteF64, SlotValue};

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Boundary: Integer overflow detection in addition
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn addition_near_overflow(a in i64::MAX - 10..=i64::MAX, b in 1..=10i64) {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), SlotValue::I64(b));
        // Should return error for overflow, not wrap
        if result.is_ok() {
            let val = result.unwrap();
            if let SlotValue::I64(v) = val {
                // Verify it didn't wrap around
                assert!(v >= a, "overflow should be detected");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Boundary: Division by small values
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn division_by_one_is_always_identity(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(a), SlotValue::I64(1));
        assert_eq!(result, Ok(SlotValue::I64(a)));
    }
}

// ---------------------------------------------------------------------------
// Boundary: Type mismatch detection
// ---------------------------------------------------------------------------

#[test]
fn add_i64_to_f64_returns_type_mismatch() {
    let result = eval_binary_op(
        BinaryOp::Add,
        SlotValue::I64(10),
        SlotValue::F64(FiniteF64::new(5.0).unwrap()),
    );
    assert!(result.is_err(), "I64 + F64 should be a type mismatch");
}

#[test]
fn add_i64_to_null_returns_type_mismatch() {
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(10), SlotValue::Null);
    assert!(result.is_err(), "I64 + Null should be a type mismatch");
}

// ---------------------------------------------------------------------------
// Boundary: Comparison with null
// ---------------------------------------------------------------------------

#[test]
fn eq_i64_to_null_is_false() {
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(10), SlotValue::Null);
    assert_eq!(result, Ok(SlotValue::Bool(false)));
}

// ---------------------------------------------------------------------------
// Boundary: F64 precision limits
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn f64_addition_precision(a in -1e15f64..=1e15f64, b in -1e15f64..=1e15f64) {
        // Result should be finite for reasonable inputs
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(FiniteF64::new(a).unwrap()),
            SlotValue::F64(FiniteF64::new(b).unwrap()),
        );
        // FiniteF64 constructor guarantees finiteness, no need to check
        assert!(result.is_ok());
    }
}

// ---------------------------------------------------------------------------
// Boundary: Boolean short-circuit
// ---------------------------------------------------------------------------

#[test]
fn and_with_false_returns_false() {
    let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false));
    assert_eq!(result, Ok(SlotValue::Bool(false)));
}

#[test]
fn or_with_true_returns_true() {
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true));
    assert_eq!(result, Ok(SlotValue::Bool(true)));
}
