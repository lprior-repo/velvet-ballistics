#![forbid(unsafe_code)]
//! Evaluator-error reachability tests (Category E).
//!
//! Verifies that the fuzz harness pipeline produces the correct `ExprError`
//! variants at the evaluator stage.

use crate::ExprError;
use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use vb_core::SlotValue;

// ── Helper: full pipeline simulator ──

fn harness_full_pipeline(source: &str) -> Result<SlotValue, ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    eval_expr_program(&program, &[], &constants)
}

// ── E-2: Integer overflow ──

#[test]
fn harness_returns_integer_overflow_for_i64_max_plus_one() {
    // i64::MAX + 1 overflows
    let source = "9223372036854775807 + 1";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::IntegerOverflow) => {}
        other => panic!("expected IntegerOverflow, got {:?}", other),
    }
}

// NOTE: i64::MIN overflow tests via the full pipeline are limited because
// the literal `9223372036854775808` (|i64::MIN|) exceeds i64::MAX and is
// rejected by the lexer as IntegerOutOfRange before reaching evaluation.
// The expression `-9223372036854775808` is lexed as operator `-` then int
// `9223372036854775808`, and the integer parse fails.
//
// i64::MIN-related overflow paths are tested via hand-crafted ExprProgram
// in the existing eval_tests.rs.

#[test]
fn harness_returns_integer_overflow_for_i64_max_times_two() {
    let source = "4611686018427387904 * 2";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::IntegerOverflow) => {}
        other => panic!("expected IntegerOverflow, got {:?}", other),
    }
}

// ── E-3: Division by zero ──

#[test]
fn harness_returns_division_by_zero_for_one_div_zero() {
    let source = "1 / 0";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero, got {:?}", other),
    }
}

#[test]
fn harness_returns_division_by_zero_for_zero_div_zero() {
    let source = "0 / 0";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero, got {:?}", other),
    }
}

#[test]
fn harness_returns_division_by_zero_for_neg_one_div_zero() {
    let source = "-1 / 0";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero, got {:?}", other),
    }
}

// ── E-4: TypeMismatch — bool in arithmetic ──

#[test]
fn harness_returns_type_mismatch_for_bool_plus_int() {
    let source = "true + 1";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "number");
            assert_eq!(found, "boolean");
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

#[test]
fn harness_returns_type_mismatch_for_null_plus_int() {
    let source = "null + 5";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "number");
            assert!(
                found.contains("null"),
                "found type should be null, got: {}",
                found
            );
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

// ── E-5: TypeMismatch — int in logical ──

#[test]
fn harness_returns_type_mismatch_for_int_and_int() {
    let source = "1 and 2";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "boolean");
            assert_eq!(found, "number");
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

#[test]
fn harness_returns_type_mismatch_for_int_or_int() {
    let source = "10 or 20";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "boolean");
            assert_eq!(found, "number");
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

// ── E-6: TypeMismatch — not on non-bool ──

#[test]
fn harness_returns_type_mismatch_for_not_int() {
    let source = "not 42";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "boolean");
            assert_eq!(found, "number");
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

#[test]
fn harness_returns_type_mismatch_for_not_null() {
    let source = "not null";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "boolean");
            assert!(
                found.contains("null"),
                "found type should be null, got: {}",
                found
            );
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

// ── E-7: TypeMismatch — negation on non-number ──

#[test]
fn harness_returns_type_mismatch_for_neg_bool() {
    let source = "-true";
    let result = harness_full_pipeline(source);
    match result {
        Err(ExprError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "number");
            assert_eq!(found, "boolean");
        }
        other => panic!("expected TypeMismatch for neg(bool), got {:?}", other),
    }
}

// ── Verify Ok values have non-empty type_name (C-FUZZ-2 assertion) ──

#[test]
fn ok_value_has_non_empty_type_name_for_integer() {
    let result = harness_full_pipeline("42");
    let value = result.expect("42 must evaluate successfully");
    let type_name = value.type_name();
    assert!(!type_name.is_empty(), "type_name must not be empty for I64");
}

#[test]
fn ok_value_has_non_empty_type_name_for_bool() {
    let result = harness_full_pipeline("true");
    let value = result.expect("true must evaluate successfully");
    let type_name = value.type_name();
    assert!(
        !type_name.is_empty(),
        "type_name must not be empty for Bool"
    );
}

#[test]
fn ok_value_has_non_empty_type_name_for_null() {
    let result = harness_full_pipeline("null");
    let value = result.expect("null must evaluate successfully");
    let type_name = value.type_name();
    assert!(
        !type_name.is_empty(),
        "type_name must not be empty for Null"
    );
}
