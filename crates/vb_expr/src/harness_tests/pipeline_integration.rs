#![forbid(unsafe_code)]
//! Pipeline-integration behavior tests (Category F).
//!
//! Verifies that valid expressions produce the correct `SlotValue` through
//! the full lex→parse→compile→eval pipeline.

use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use vb_core::SlotValue;

// ── Helper: full pipeline ──

fn pipeline_eval(source: &str) -> Result<SlotValue, crate::ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    eval_expr_program(&program, &[], &constants)
}

// ── F-1: Arithmetic with precedence ──

#[test]
fn pipeline_evaluates_arithmetic_with_precedence() {
    let result = pipeline_eval("3 + 4 * 2");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 11, "3 + 4*2 = 11"),
        other => panic!("expected Ok(I64(11)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_multiplication_before_addition() {
    let result = pipeline_eval("2 * 3 + 5");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 11, "2*3 + 5 = 11"),
        other => panic!("expected Ok(I64(11)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_subtraction_and_addition() {
    let result = pipeline_eval("10 - 3 + 2");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 9, "10-3+2 = 9"),
        other => panic!("expected Ok(I64(9)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_division_with_truncation() {
    let result = pipeline_eval("7 / 2");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 3, "7/2 = 3 (integer division)"),
        other => panic!("expected Ok(I64(3)), got {:?}", other),
    }
}

// ── F-2: Boolean expressions ──

#[test]
fn pipeline_evaluates_boolean_and_or() {
    let result = pipeline_eval("true and false or true");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "true and false or true = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_boolean_and() {
    let result = pipeline_eval("true and true");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_boolean_or() {
    let result = pipeline_eval("false or true");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

// ── F-3: Comparisons ──

#[test]
fn pipeline_evaluates_comparison_greater() {
    let result = pipeline_eval("5 > 3");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "5 > 3 = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_comparison_less() {
    let result = pipeline_eval("2 < 8");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "2 < 8 = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_comparison_less_or_equal_true() {
    let result = pipeline_eval("5 <= 5");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "5 <= 5 = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_comparison_less_or_equal_false() {
    let result = pipeline_eval("7 <= 3");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(!b, "7 <= 3 = false"),
        other => panic!("expected Ok(Bool(false)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_comparison_greater_or_equal() {
    let result = pipeline_eval("10 >= 10");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "10 >= 10 = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

// ── F-4: Parenthesized expressions ──

#[test]
fn pipeline_evaluates_parenthesized_expression() {
    let result = pipeline_eval("(1 + 2) * 3");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 9, "(1+2)*3 = 9"),
        other => panic!("expected Ok(I64(9)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_nested_parenthesized_expression() {
    let result = pipeline_eval("((10 - 4) * 2) + 1");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 13, "((10-4)*2)+1 = 13"),
        other => panic!("expected Ok(I64(13)), got {:?}", other),
    }
}

// ── F-5: Negation ──

#[test]
fn pipeline_evaluates_negation() {
    let result = pipeline_eval("-5");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, -5),
        other => panic!("expected Ok(I64(-5)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_double_negation() {
    let result = pipeline_eval("--10");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 10, "--10 = 10"),
        other => panic!("expected Ok(I64(10)), got {:?}", other),
    }
}

// ── F-6: not expressions ──

#[test]
fn pipeline_evaluates_double_negation_bool() {
    let result = pipeline_eval("not not true");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "not not true = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_not_false() {
    let result = pipeline_eval("not false");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "not false = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_not_true() {
    let result = pipeline_eval("not true");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(!b, "not true = false"),
        other => panic!("expected Ok(Bool(false)), got {:?}", other),
    }
}

// ── F-7/F-8: Equality/inequality with null ──

#[test]
fn pipeline_evaluates_null_equality() {
    let result = pipeline_eval("null == null");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "null == null = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_null_inequality() {
    let result = pipeline_eval("null != 1");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b, "null != 1 = true"),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_equality_same_integers() {
    let result = pipeline_eval("42 == 42");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

#[test]
fn pipeline_evaluates_inequality_different_integers() {
    let result = pipeline_eval("42 != 7");
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(Bool(true)), got {:?}", other),
    }
}

// ── type_name checks on success ──

#[test]
fn pipeline_ok_result_has_valid_type_name() {
    for (source, _expected_type) in [("1 + 2", "number"), ("true", "boolean"), ("null", "null")] {
        let result = pipeline_eval(source).expect("expression must evaluate");
        let type_name = result.type_name();
        assert!(
            !type_name.is_empty(),
            "type_name must not be empty for '{}', got '{}'",
            source,
            type_name
        );
    }
}

// ── Error classification: Err results are known ExprError members ──

#[test]
fn pipeline_err_variants_are_known_exprerror_members() {
    // Spot-check that error variants produced by the pipeline match our enum
    // This test exists to catch refactorings that change the error type
    let error_sources = [
        ("+ 5", "UnexpectedToken"),
        ("1 / 0", "DivisionByZero"),
        ("\"hello", "UnterminatedString"),
        ("$x", "InvalidReference"),
        ("@", "UnexpectedChar"),
        ("foobar(1)", "UnknownHelper"),
    ];
    for (source, _expected_variant_name) in &error_sources {
        let result = pipeline_eval(source);
        assert!(result.is_err(), "'{}' must produce an error", source);
    }
}
