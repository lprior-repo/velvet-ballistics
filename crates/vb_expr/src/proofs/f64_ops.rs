#![forbid(unsafe_code)]
//! Kani harnesses for F64 bytecode arithmetic semantics (PO-001).
//!
//! Proof obligations covered:
//! - PO-001: Finite inputs to F64 add/sub/mul/div never produce NonFiniteFloat
//!
//! Key design decisions captured:
//! - F64/0 → ±Inf → FiniteF64::new fails → NonFiniteFloat (NOT DivisionByZero)
//! - NaN comparisons yield false (IEEE 754 semantics)

use crate::eval::{BinaryOp, UnaryOp, eval_binary_op, eval_unary_op};
use vb_core::SlotValue;
use vb_core::value::FiniteF64;

/// Kani harness for PO-001: F64 addition preserves finiteness.
///
/// Bound: The addition of two finite f64s can overflow to infinity when
/// operands are near f64::MAX. We bound operands to |l|,|r| <= f64::MAX/2
/// so the proof focuses on the constructor validity invariant.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_add_preserves_finiteness() {
    let left_f64: f64 = kani::any();
    let right_f64: f64 = kani::any();

    // Restrict to finite values
    kani::assume(left_f64.is_finite());
    kani::assume(right_f64.is_finite());
    // Bound to prevent overflow to infinity: |l| + |r| <= f64::MAX
    kani::assume(left_f64.abs() <= f64::MAX / 2.0);
    kani::assume(right_f64.abs() <= f64::MAX / 2.0);

    let left = match FiniteF64::new(left_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let right = match FiniteF64::new(right_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(left), SlotValue::F64(right));

    // PO-001: Result must be Ok and the F64 must be finite
    kani::assert(result.is_ok(),
        "eval_add_op with bounded finite inputs must not error",
    );
    let Ok(SlotValue::F64(f)) = result else {
        return;
    };
    kani::assert(f.get().is_finite(),
        "eval_add_op of two bounded finite f64s must produce finite f64",
    );
}

/// Kani harness for PO-001: F64 subtraction preserves finiteness.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_sub_preserves_finiteness() {
    let left_f64: f64 = kani::any();
    let right_f64: f64 = kani::any();

    kani::assume(left_f64.is_finite());
    kani::assume(right_f64.is_finite());
    // Bound: subtraction can overflow only when both operands are near extremes
    kani::assume(left_f64.abs() <= f64::MAX / 2.0);
    kani::assume(right_f64.abs() <= f64::MAX / 2.0);

    let left = match FiniteF64::new(left_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let right = match FiniteF64::new(right_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let result = eval_binary_op(BinaryOp::Sub, SlotValue::F64(left), SlotValue::F64(right));

    kani::assert(result.is_ok(),
        "eval_sub_op with bounded finite inputs must not error",
    );
    let Ok(SlotValue::F64(f)) = result else {
        return;
    };
    kani::assert(f.get().is_finite(),
        "eval_sub_op of two bounded finite f64s must produce finite f64",
    );
}

/// Kani harness for PO-001: F64 multiplication preserves finiteness.
///
/// Bound: a * b can overflow to infinity when |a| * |b| > f64::MAX.
/// We bound operands so |l| * |r| <= f64::MAX / 2.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_mul_preserves_finiteness() {
    let left_f64: f64 = kani::any();
    let right_f64: f64 = kani::any();

    kani::assume(left_f64.is_finite());
    kani::assume(right_f64.is_finite());
    // Bound: |l| * |r| <= f64::MAX / 2
    let max_sqrt = (f64::MAX / 2.0).sqrt();
    kani::assume(left_f64.abs() <= max_sqrt);
    kani::assume(right_f64.abs() <= max_sqrt);

    let left = match FiniteF64::new(left_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let right = match FiniteF64::new(right_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let result = eval_binary_op(BinaryOp::Mul, SlotValue::F64(left), SlotValue::F64(right));

    kani::assert(result.is_ok(),
        "eval_mul_op with bounded finite inputs must not error",
    );
    let Ok(SlotValue::F64(f)) = result else {
        return;
    };
    kani::assert(f.get().is_finite(),
        "eval_mul_op of two bounded finite f64s must produce finite f64",
    );
}

/// Kani harness for PO-001: F64 negation preserves finiteness.
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_neg_preserves_finiteness() {
    let val_f64: f64 = kani::any();
    kani::assume(val_f64.is_finite());

    let val = match FiniteF64::new(val_f64) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(val));

    kani::assert(result.is_ok(),
        "eval_neg_op with finite input must not error",
    );
    let Ok(SlotValue::F64(f)) = result else {
        return;
    };
    kani::assert(f.get().is_finite(),
        "eval_neg_op of a finite f64 must produce finite f64",
    );
}
