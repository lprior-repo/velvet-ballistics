#![forbid(unsafe_code)]
//! Adversarial typecheck tests.

#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

use crate::ExprError;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::typecheck::{TypeContext, typecheck_expr};

fn check(source: &str) -> crate::ExprResult<crate::typecheck::ExprType> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

#[test]
fn typecheck_expr_rejects_null_in_arithmetic() -> crate::ExprResult<()> {
    let result = check("null + 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null + 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_text_in_arithmetic() -> crate::ExprResult<()> {
    let result = check("\"hello\" - 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for text - 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_null_in_comparison() -> crate::ExprResult<()> {
    let result = check("null < 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null < 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_number_in_and() -> crate::ExprResult<()> {
    let result = check("1 and 2");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 and 2".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_null_in_and() -> crate::ExprResult<()> {
    let result = check("null and true");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null and true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_null() -> crate::ExprResult<()> {
    let result = check("-null");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -null".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_allows_eq_on_mixed_types() -> crate::ExprResult<()> {
    let ty = check("null == 1")?;
    assert_eq!(ty, crate::typecheck::ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_not_eq_on_incompatible_types() -> crate::ExprResult<()> {
    let ty = check("true != null")?;
    assert_eq!(ty, crate::typecheck::ExprType::Bool);
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- typecheck
// =========================================================================

/// BH-TC-001: Typecheck rejects null in all arithmetic operations.
#[test]
fn blackhat_tc_001_null_rejected_in_all_arithmetic() -> crate::ExprResult<()> {
    for op in &["+", "-", "*", "/"] {
        let source = format!("null {op} 1");
        let result = check(&source);
        assert!(
            matches!(result, Err(ExprError::TypeMismatch { .. })),
            "BH-TC-001: null {op} 1 should be TypeMismatch"
        );
    }
    Ok(())
}

/// BH-TC-002: Typecheck rejects null in all comparison operations.
#[test]
fn blackhat_tc_002_null_rejected_in_all_comparisons() -> crate::ExprResult<()> {
    for op in &["<", "<=", ">", ">="] {
        let source = format!("null {op} 1");
        let result = check(&source);
        assert!(
            matches!(result, Err(ExprError::TypeMismatch { .. })),
            "BH-TC-002: null {op} 1 should be TypeMismatch"
        );
    }
    Ok(())
}

/// BH-TC-003: Typecheck rejects non-boolean in logical operators.
#[test]
fn blackhat_tc_003_non_bool_rejected_in_logical() -> crate::ExprResult<()> {
    // number and number
    let result = check("1 and 2");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { expected, .. }) if expected == "boolean"),
        "BH-TC-003a: 1 and 2 should fail"
    );
    // null and true
    let result = check("null and true");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-003b: null and true should fail"
    );
    // 1 or 0
    let result = check("1 or 0");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-003c: 1 or 0 should fail"
    );
    Ok(())
}

/// BH-TC-004: Equality operators allow mixed types (by design).
///
/// `==` and `!=` are polymorphic -- they can compare any two types.
/// This is by design for null-safety checks.
#[test]
fn blackhat_tc_004_equality_allows_mixed_types() -> crate::ExprResult<()> {
    assert_eq!(check("null == 1")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("true != null")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("null == null")?, crate::typecheck::ExprType::Bool);
    Ok(())
}

/// BH-TC-005: Negation rejects null and boolean.
#[test]
fn blackhat_tc_005_negation_rejects_non_numeric() -> crate::ExprResult<()> {
    let result = check("-null");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-005a: -null should fail"
    );
    let result = check("-true");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-005b: -true should fail"
    );
    Ok(())
}

/// BH-TC-006: Not rejects non-boolean.
#[test]
fn blackhat_tc_006_not_rejects_non_bool() -> crate::ExprResult<()> {
    let result = check("not 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-006a: not 1 should fail"
    );
    let result = check("not null");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-006b: not null should fail"
    );
    Ok(())
}

/// BH-TC-007: Unknown type allowed through (deferred to runtime).
///
/// Unresolved references produce `Unknown` type which passes through all
/// operators. This is by design -- the eval layer catches type errors.
#[test]
fn blackhat_tc_007_unknown_deferred_to_runtime() -> crate::ExprResult<()> {
    assert_eq!(check("$x + 1")?, crate::typecheck::ExprType::I64);
    assert_eq!(check("not $x")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("$x and $y")?, crate::typecheck::ExprType::Bool);
    Ok(())
}

// =========================================================================
// ADVERSARIAL extended tests
// =========================================================================

/// ADV-TC-008: Typecheck rejects F64 in logical not.
#[test]
fn adversarial_tc_008_rejects_f64_in_not() -> crate::ExprResult<()> {
    let result = check("not 3.14");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for not 3.14".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "f64");
    Ok(())
}

/// ADV-TC-009: Typecheck rejects text in negation.
#[test]
fn adversarial_tc_009_rejects_text_in_negation() -> crate::ExprResult<()> {
    let result = check("-\"hello\"");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -\"hello\"".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

/// ADV-TC-010: Typecheck rejects negation on object type.
#[test]
fn adversarial_tc_010_rejects_object_in_negation() -> crate::ExprResult<()> {
    let result = check("-merge($x, $y)");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -merge(..)".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "object");
    Ok(())
}

/// ADV-TC-011: Typecheck rejects text in all four comparison operators.
#[test]
fn adversarial_tc_011_text_rejected_in_all_comparisons() -> crate::ExprResult<()> {
    for op in &["<", "<=", ">", ">="] {
        let source = format!("\"hello\" {op} 1");
        let result = check(&source);
        assert!(
            matches!(result, Err(ExprError::TypeMismatch { expected, .. }) if expected == "number"),
            "ADV-TC-011: \"hello\" {op} 1 should be TypeMismatch"
        );
    }
    Ok(())
}

/// ADV-TC-012: Typecheck coerces Unknown + F64 to F64.
#[test]
fn adversarial_tc_012_unknown_plus_f64_coerces_to_f64() -> crate::ExprResult<()> {
    assert_eq!(check("$x + 3.14")?, crate::typecheck::ExprType::F64);
    assert_eq!(check("3.14 + $x")?, crate::typecheck::ExprType::F64);
    Ok(())
}

/// ADV-TC-013: Typecheck coerces Unknown * F64 to F64.
#[test]
fn adversarial_tc_013_unknown_mul_f64_coerces_to_f64() -> crate::ExprResult<()> {
    assert_eq!(check("$x * 2.5")?, crate::typecheck::ExprType::F64);
    Ok(())
}

/// ADV-TC-014: Typecheck rejects F64 in arithmetic comparison when one side is text.
#[test]
fn adversarial_tc_014_right_text_rejected_in_comparison() -> crate::ExprResult<()> {
    let result = check("1 <= \"world\"");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 <= \"world\"".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

/// ADV-TC-015: Nested arithmetic propagates inner type error.
#[test]
fn adversarial_tc_015_nested_arithmetic_propagates_inner_type_error() -> crate::ExprResult<()> {
    let result = check("1 + \"hello\" - 2");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch in nested arithmetic".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

/// ADV-TC-016: Nested logical propagates inner type error.
#[test]
fn adversarial_tc_016_nested_logical_propagates_inner_type_error() -> crate::ExprResult<()> {
    let result = check("1 and 2 or true");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch in nested logical".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

/// ADV-TC-017: Unmatched unknown type in comparison allows passthrough.
#[test]
fn adversarial_tc_017_unknown_in_all_comparisons_passthrough() -> crate::ExprResult<()> {
    for op in &["<", "<=", ">", ">="] {
        let source = format!("$x {op} $y");
        assert_eq!(
            check(&source)?,
            crate::typecheck::ExprType::Bool,
            "ADV-TC-017: $x {op} $y should infer Bool"
        );
    }
    Ok(())
}

/// ADV-TC-018: TypeContext lookup returns Unknown for unregistered variable after real registrations.
#[test]
fn adversarial_tc_018_context_lookup_returns_unknown_after_real_bindings() {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$a"), crate::typecheck::ExprType::I64);
    ctx.add_variable(Box::from("$b"), crate::typecheck::ExprType::Text);
    assert_eq!(ctx.lookup("$c"), crate::typecheck::ExprType::Unknown);
}

/// ADV-TC-019: TypeContext shadow lookup uses most recent binding.
#[test]
fn adversarial_tc_019_context_lookup_uses_most_recent_shadow() {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$v"), crate::typecheck::ExprType::I64);
    ctx.add_variable(Box::from("$v"), crate::typecheck::ExprType::F64);
    ctx.add_variable(Box::from("$v"), crate::typecheck::ExprType::Text);
    assert_eq!(ctx.lookup("$v"), crate::typecheck::ExprType::Text);
}

/// ADV-TC-020: Large type context with many variables.
#[test]
fn adversarial_tc_020_large_context_with_many_variables() {
    let mut ctx = TypeContext::new();
    for i in 0..100 {
        let name = Box::<str>::from(format!("$var{i}"));
        ctx.add_variable(name, crate::typecheck::ExprType::I64);
    }
    ctx.add_variable(Box::from("$target"), crate::typecheck::ExprType::Bool);
    assert_eq!(ctx.lookup("$target"), crate::typecheck::ExprType::Bool);
    assert_eq!(ctx.lookup("$var0"), crate::typecheck::ExprType::I64);
    assert_eq!(ctx.lookup("$var99"), crate::typecheck::ExprType::I64);
    assert_eq!(ctx.lookup("$missing"), crate::typecheck::ExprType::Unknown);
}

/// ADV-TC-021: Helper type-inference propagates arg error rather than returning inferred type.
#[test]
fn adversarial_tc_021_helper_propagates_inner_type_error() -> crate::ExprResult<()> {
    let result = check("sum(\"hello\" + 1)");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch in sum arg".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}
