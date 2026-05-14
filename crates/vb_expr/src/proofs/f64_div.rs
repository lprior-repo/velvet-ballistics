#![forbid(unsafe_code)]
//! Kani harnesses for F64 division semantics (PO-002).
//!
//! Proof obligations covered:
//! - PO-002: F64/0 → ±Inf → FiniteF64::new fails → ExprError::NonFiniteFloat
//!            (NOT DivisionByZero — this is the key design distinction from I64)
//!
//! Key design decisions captured:
//! - F64/0 yields Inf per IEEE 754
//! - FiniteF64::new(±Inf) fails → None
//! - eval_div_op maps this to ExprError::NonFiniteFloat
//! - I64/0 path is separate and returns DivisionByZero

use crate::eval::{eval_binary_op, BinaryOp};
use crate::ExprError;
use vb_core::value::FiniteF64;
use vb_core::SlotValue;

/// Kani harness for PO-002: F64/non-zero-finite/0 returns NonFiniteFloat (NOT DivisionByZero).
///
/// This covers the IEEE 754 case: non-zero-finite / 0 → ±Inf → NonFiniteFloat.
///
/// Note: IEEE 754 defines 0/0 → NaN (not Inf), which also produces NonFiniteFloat.
/// Both are acceptable outcomes for the F64/0 path. We use a non-zero dividend
/// to isolate the ±Inf path specifically.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_div_by_zero_returns_non_finite_float() {
    // Non-zero finite dividend (0/0 = NaN is a different IEEE 754 case)
    let dividend_f64: f64 = kani::any();
    kani::assume(dividend_f64.is_finite());
    kani::assume(dividend_f64 != 0.0); // Exclude 0/0 = NaN case

    let dividend = FiniteF64::new(dividend_f64).unwrap();

    // The divisor is zero
    let divisor = FiniteF64::new(0.0_f64).unwrap();

    let result =
        eval_binary_op(BinaryOp::Div, SlotValue::F64(dividend), SlotValue::F64(divisor));

    // PO-002: Result must be Err with NonFiniteFloat
    assert!(
        result.is_err(),
        "F64/non-zero-finite/0 must return an error (Inf from IEEE 754 → NonFiniteFloat)"
    );
    let Err(e) = result else { return };

    // The error MUST be NonFiniteFloat, NOT DivisionByZero
    assert!(
        matches!(e, ExprError::NonFiniteFloat),
        "F64/0 must return NonFiniteFloat, not DivisionByZero. Got: {:?}",
        e
    );
}

/// Kani harness for PO-002: F64/non-zero-finite returns finite quotient.
///
/// Note: Full f64 division verification (proving exact quotient matches IEEE 754)
/// is handled by proptest (PO-008). This Kani harness focuses on the finiteness
/// property — confirming that non-zero-finite / non-zero-finite never produces
/// NaN or Inf.
///
/// The quotient accuracy is covered by PO-008 via exhaustive proptest.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_div_by_nonzero_finite_succeeds() {
    let dividend_f64: f64 = kani::any();
    let divisor_f64: f64 = kani::any();

    kani::assume(dividend_f64.is_finite());
    kani::assume(divisor_f64.is_finite());
    kani::assume(divisor_f64 != 0.0);
    // Bound: keep quotient within finite range, dividend <= f64::MAX/2 and |divisor| >= 1.0
    kani::assume(dividend_f64.abs() <= f64::MAX / 2.0);
    kani::assume(divisor_f64.abs() >= 1.0);

    let dividend = FiniteF64::new(dividend_f64).unwrap();
    let divisor = FiniteF64::new(divisor_f64).unwrap();

    let result =
        eval_binary_op(BinaryOp::Div, SlotValue::F64(dividend), SlotValue::F64(divisor));

    // F64/non-zero must succeed
    assert!(
        result.is_ok(),
        "F64/non-zero-finite must succeed. Got: {:?}",
        result
    );
    let Ok(SlotValue::F64(f)) = result else { return };

    // The quotient must be finite
    assert!(
        f.get().is_finite(),
        "F64/non-zero-finite quotient must be finite. Got: {:?}",
        f.get()
    );
}

/// Kani harness for PO-002: I64/0 still returns DivisionByZero (not NonFiniteFloat).
///
/// This harness confirms the I64 division path is separate and correctly returns
/// DivisionByZero, proving the F64 vs I64 paths do not interfere.
#[kani::proof]
#[kani::unwind(4)]
fn kani_i64_div_by_zero_returns_division_by_zero() {
    let dividend: i64 = kani::any();
    let divisor: i64 = kani::any();
    kani::assume(divisor == 0);

    let result =
        eval_binary_op(BinaryOp::Div, SlotValue::I64(dividend), SlotValue::I64(divisor));

    // I64/0 must be DivisionByZero
    assert!(result.is_err(), "I64/0 must return an error");
    let Err(e) = result else { return };
    assert!(
        matches!(e, ExprError::DivisionByZero),
        "I64/0 must return DivisionByZero, not NonFiniteFloat. Got: {:?}",
        e
    );
}
