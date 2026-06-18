#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
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
    for (source, expected_variant_name) in &error_sources {
        let result = pipeline_eval(source);
        let Err(err) = result else {
            panic!("'{}' must produce an error, got Ok", source);
        };
        match *expected_variant_name {
            "UnexpectedToken" => {
                assert!(
                    matches!(err, crate::ExprError::UnexpectedToken { .. }),
                    "expected UnexpectedToken for '{}', got {:?}",
                    source,
                    err
                );
            }
            "DivisionByZero" => {
                assert!(
                    matches!(err, crate::ExprError::DivisionByZero),
                    "expected DivisionByZero for '{}', got {:?}",
                    source,
                    err
                );
            }
            "UnterminatedString" => {
                assert!(
                    matches!(err, crate::ExprError::UnterminatedString { .. }),
                    "expected UnterminatedString for '{}', got {:?}",
                    source,
                    err
                );
            }
            "InvalidReference" => {
                assert!(
                    matches!(err, crate::ExprError::InvalidReference { .. }),
                    "expected InvalidReference for '{}', got {:?}",
                    source,
                    err
                );
            }
            "UnexpectedChar" => {
                assert!(
                    matches!(err, crate::ExprError::UnexpectedChar { .. }),
                    "expected UnexpectedChar for '{}', got {:?}",
                    source,
                    err
                );
            }
            "UnknownHelper" => {
                assert!(
                    matches!(err, crate::ExprError::UnknownHelper { .. }),
                    "expected UnknownHelper for '{}', got {:?}",
                    source,
                    err
                );
            }
            other => panic!("unknown expected variant: {}", other),
        }
    }
}
