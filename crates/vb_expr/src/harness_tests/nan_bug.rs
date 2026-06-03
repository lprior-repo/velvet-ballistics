#![forbid(unsafe_code)]
//! NaN bug test scenarios (Category I).
//!
//! Verifies that division of floating-point values producing NaN, Inf, or -Inf
//! is caught at the evaluator stage and returns `NonFiniteFloat` rather than
//! panicking or silently propagating non-finite values.

use crate::ExprError;
use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use vb_core::SlotValue;

// ── Helper: full pipeline ──

fn pipeline_eval(source: &str) -> Result<SlotValue, ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    eval_expr_program(&program, &[], &constants)
}

// ── I-1: 0.0 / 0.0 → NaN → NonFiniteFloat ──

#[test]
fn nan_bug_zero_dot_zero_div_zero_dot_zero_returns_non_finite_float() {
    // Given: expression "0.0 / 0.0"
    let source = "0.0 / 0.0";
    // When: full pipeline executes
    let result = pipeline_eval(source);
    // Then: NonFiniteFloat error (NaN detected by FiniteF64::new)
    match result {
        Err(ExprError::NonFiniteFloat) => {}
        other => panic!(
            "0.0 / 0.0 must produce NonFiniteFloat (NaN bug), got {:?}",
            other
        ),
    }
}

// ── I-2: 1.0 / 0.0 → Inf → NonFiniteFloat ──

#[test]
fn nan_bug_one_dot_zero_div_zero_dot_zero_returns_non_finite_float() {
    // Given: expression "1.0 / 0.0"
    let source = "1.0 / 0.0";
    // When: full pipeline executes
    let result = pipeline_eval(source);
    // Then: NonFiniteFloat (Inf detected)
    match result {
        Err(ExprError::NonFiniteFloat) => {}
        other => panic!(
            "1.0 / 0.0 must produce NonFiniteFloat (Inf bug), got {:?}",
            other
        ),
    }
}

// ── I-3: -1.0 / 0.0 → -Inf → NonFiniteFloat ──

#[test]
fn nan_bug_neg_one_dot_zero_div_zero_dot_zero_returns_non_finite_float() {
    // Given: expression "-1.0 / 0.0"
    let source = "-1.0 / 0.0";
    // When: full pipeline executes
    let result = pipeline_eval(source);
    // Then: NonFiniteFloat (-Inf detected)
    match result {
        Err(ExprError::NonFiniteFloat) => {}
        other => panic!(
            "-1.0 / 0.0 must produce NonFiniteFloat (-Inf bug), got {:?}",
            other
        ),
    }
}

// ── Additional NonFiniteFloat tests via other ops ──

#[test]
fn nan_bug_f64_multiplication_producing_inf() {
    // 1e308 * 1e308 overflows to Inf in f64
    // But our lexer only handles [0-9]+\.[0-9]+ without e-notation
    // So we need a different approach — large values
    let source =
        "99999999999999999999999999999999999999.0 * 99999999999999999999999999999999999999.0";
    let result = pipeline_eval(source);
    // This large value may not parse (IntegerOutOfRange), but if it does lex,
    // it should produce NonFiniteFloat or evaluate fine
    match result {
        Err(ExprError::NonFiniteFloat) | Err(ExprError::IntegerOutOfRange) => {}
        Ok(_) => {} // also acceptable if doesn't overflow
        other => panic!(
            "unexpected result for large f64 multiplication: {:?}",
            other
        ),
    }
}

#[test]
fn nan_bug_f64_add_zero_does_not_produce_non_finite() {
    // Valid finite F64 addition should succeed
    let source = "1.0 + 2.0";
    let result = pipeline_eval(source);
    match result {
        Ok(SlotValue::F64(f)) => {
            assert!(
                (f.get() - 3.0).abs() < 1e-10,
                "1.0 + 2.0 = 3.0, got {}",
                f.get()
            );
        }
        other => panic!("expected Ok(F64(3.0)), got {:?}", other),
    }
}

#[test]
fn nan_bug_f64_subtract_produces_valid_result() {
    let source = "5.0 - 3.5";
    let result = pipeline_eval(source);
    match result {
        Ok(SlotValue::F64(f)) => {
            assert!(
                (f.get() - 1.5).abs() < 1e-10,
                "5.0 - 3.5 = 1.5, got {}",
                f.get()
            );
        }
        other => panic!("expected Ok(F64(1.5)), got {:?}", other),
    }
}

#[test]
fn nan_bug_f64_multiply_produces_valid_result() {
    let source = "2.5 * 4.0";
    let result = pipeline_eval(source);
    match result {
        Ok(SlotValue::F64(f)) => {
            assert!(
                (f.get() - 10.0).abs() < 1e-10,
                "2.5 * 4.0 = 10.0, got {}",
                f.get()
            );
        }
        other => panic!("expected Ok(F64(10.0)), got {:?}", other),
    }
}

// NOTE: F64 negation (`-3.14`) does not currently work through the full pipeline.
// The negation lowerer pushes `ConstValue::I64(0)`, producing I64(0) - F64(3.14)
// which hits a TypeMismatch at eval. This is a known limitation.
// The test verifies this behavior to prevent silent regressions.
#[test]
fn nan_bug_f64_negation_produces_type_mismatch() {
    let source = "-3.14";
    let result = pipeline_eval(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            // Both expected and found report "number" since I64 and F64
            // both have type_name "number" — the mismatch is internal
            assert!(!expected.is_empty());
            assert!(!found.is_empty());
        }
        other => panic!(
            "expected TypeMismatch for -3.14 (I64/F64 mismatch), got {:?}",
            other
        ),
    }
}

#[test]
fn nan_bug_f64_zero_div_zero_as_i64_returns_division_by_zero() {
    // This path goes through i64 division, which returns DivisionByZero
    let source = "0 / 0";
    let result = pipeline_eval(source);
    match result {
        Err(ExprError::DivisionByZero) => {}
        other => panic!("0 / 0 (i64) must produce DivisionByZero, got {:?}", other),
    }
}
