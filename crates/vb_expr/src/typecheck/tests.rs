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
//! BDD typecheck tests.

mod adversarial;

#[allow(unused_imports)]
use crate::ExprError;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::typecheck::{ExprType, TypeContext, typecheck_expr};

#[allow(dead_code)]
fn check(source: &str) -> crate::ExprResult<ExprType> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

#[test]
fn infers_literal_types() -> crate::ExprResult<()> {
    assert_eq!(check("42")?, ExprType::I64);
    assert_eq!(check("true")?, ExprType::Bool);
    assert_eq!(check("null")?, ExprType::Null);
    assert_eq!(check("\"hello\"")?, ExprType::Text);
    assert_eq!(check("3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn infers_arithmetic_result() -> crate::ExprResult<()> {
    assert_eq!(check("1 + 2")?, ExprType::I64);
    Ok(())
}

#[test]
fn infers_comparison_result() -> crate::ExprResult<()> {
    assert_eq!(check("1 < 2")?, ExprType::Bool);
    assert_eq!(check("1 == 2")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_logical_result() -> crate::ExprResult<()> {
    assert_eq!(check("true and false")?, ExprType::Bool);
    assert_eq!(check("true or false")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_helper_result() -> crate::ExprResult<()> {
    assert_eq!(check("length($x)")?, ExprType::I64);
    assert_eq!(check("empty($x)")?, ExprType::Bool);
    assert_eq!(check("contains($x, $y)")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_unary_not() -> crate::ExprResult<()> {
    assert_eq!(check("not true")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_negation_preserves_type() -> crate::ExprResult<()> {
    assert_eq!(check("-42")?, ExprType::I64);
    Ok(())
}

#[test]
fn unknown_type_for_unresolved_reference() -> crate::ExprResult<()> {
    assert_eq!(check("$unknown")?, ExprType::Unknown);
    Ok(())
}

#[test]
fn context_resolves_known_variables() -> crate::ExprResult<()> {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$x"), ExprType::I64);
    let tokens = lex_expr("$x + 1")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &ctx)?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

// --- BDD typecheck tests ---

#[test]
fn typecheck_expr_validates_numeric_operands() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_string_in_arithmetic() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello\" + 1")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_validates_boolean_operands_for_logical_ops() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_number_in_logical_op() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 and 2")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

#[test]
fn infix_binding_power_returns_correct_precedence_for_operators() {
    let (or_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Or);
    let (and_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::And);
    let (add_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Add);
    let (mul_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Mul);
    assert!(
        or_bp < and_bp,
        "or bp ({or_bp}) should be less than and bp ({and_bp})"
    );
    assert!(
        and_bp < add_bp,
        "and bp ({and_bp}) should be less than add bp ({add_bp})"
    );
    assert!(
        add_bp < mul_bp,
        "add bp ({add_bp}) should be less than mul bp ({mul_bp})"
    );
}

#[test]
fn typecheck_expr_validates_negation_on_number() -> crate::ExprResult<()> {
    let tokens = lex_expr("-42")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_boolean() -> crate::ExprResult<()> {
    let tokens = lex_expr("-true")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "boolean");
    Ok(())
}

#[test]
fn typecheck_expr_infers_helper_return_types() -> crate::ExprResult<()> {
    let ty_len = check("length($x)")?;
    assert_eq!(ty_len, ExprType::I64);

    let ty_empty = check("empty($x)")?;
    assert_eq!(ty_empty, ExprType::Bool);

    let ty_contains = check("contains($x, $y)")?;
    assert_eq!(ty_contains, ExprType::Bool);

    let ty_sum = check("sum($x)")?;
    assert_eq!(ty_sum, ExprType::I64);

    let ty_unique = check("unique($x)")?;
    assert_eq!(ty_unique, ExprType::List);

    let ty_merge = check("merge($x, $y)")?;
    assert_eq!(ty_merge, ExprType::Object);
    Ok(())
}

#[test]
fn typecheck_expr_allows_unknown_in_arithmetic_left() -> crate::ExprResult<()> {
    let ty = check("$x + 1")?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

// --- Coercion tests ---

#[test]
fn coercion_i64_plus_f64_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("1 + 3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_f64_plus_i64_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("3.14 + 1")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_i64_times_f64_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("2 * 3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_i64_minus_f64_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("10 - 2.5")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_i64_div_f64_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("10 / 3.0")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_two_f64s_gives_f64_in_arithmetic() -> crate::ExprResult<()> {
    assert_eq!(check("1.5 * 2.0")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_unknown_left_with_f64_right_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("$x + 3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn coercion_f64_left_with_unknown_right_gives_f64() -> crate::ExprResult<()> {
    assert_eq!(check("3.14 + $x")?, ExprType::F64);
    Ok(())
}

// --- Unary type error tests ---

#[test]
fn typecheck_expr_rejects_not_on_f64() -> crate::ExprResult<()> {
    let tokens = lex_expr("not 3.14")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for not 3.14".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "f64");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_text() -> crate::ExprResult<()> {
    let tokens = lex_expr("-\"hello\"")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -\"hello\"".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_object_type() -> crate::ExprResult<()> {
    let tokens = lex_expr("-merge($x, $y)")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, .. }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -merge(..)".into(),
        });
    };
    assert_eq!(expected, "number");
    Ok(())
}

// --- TypeContext tests ---

#[test]
fn typecontext_lookup_returns_unknown_for_empty_context() {
    let ctx = TypeContext::new();
    assert_eq!(ctx.lookup("$x"), ExprType::Unknown);
    assert_eq!(ctx.lookup("anything"), ExprType::Unknown);
}

#[test]
fn typecontext_lookup_returns_unknown_for_unregistered_variable() {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$a"), ExprType::I64);
    assert_eq!(ctx.lookup("$b"), ExprType::Unknown);
}

#[test]
fn typecontext_shadows_earlier_binding_with_later() {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$x"), ExprType::I64);
    ctx.add_variable(Box::from("$x"), ExprType::Text);
    assert_eq!(ctx.lookup("$x"), ExprType::Text);
}

#[test]
fn typecontext_preserves_multiple_distinct_variables() {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$a"), ExprType::I64);
    ctx.add_variable(Box::from("$b"), ExprType::Text);
    ctx.add_variable(Box::from("$c"), ExprType::Bool);
    assert_eq!(ctx.lookup("$a"), ExprType::I64);
    assert_eq!(ctx.lookup("$b"), ExprType::Text);
    assert_eq!(ctx.lookup("$c"), ExprType::Bool);
}

#[test]
fn typecontext_default_is_empty() {
    let ctx = TypeContext::default();
    assert_eq!(ctx.lookup("$x"), ExprType::Unknown);
}

// --- Comparison operator tests ---

#[test]
fn typecheck_expr_allows_f64_in_comparison() -> crate::ExprResult<()> {
    assert_eq!(check("3.14 < 2.71")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_mixed_i64_f64_in_comparison() -> crate::ExprResult<()> {
    assert_eq!(check("1 < 3.14")?, ExprType::Bool);
    assert_eq!(check("3.14 >= 1")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_text_in_comparison() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello\" < 1")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for \"hello\" < 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_text_in_greater_than_or_equal() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 >= \"world\"")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 >= \"world\"".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

// --- Logical operator edge tests ---

#[test]
fn typecheck_expr_rejects_non_bool_right_in_or() -> crate::ExprResult<()> {
    let tokens = lex_expr("true or 1")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for true or 1".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_non_bool_left_in_and() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 and true")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 and true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

// --- Equality operator tests (polymorphic by design) ---

#[test]
fn typecheck_expr_allows_eq_with_null_and_number() -> crate::ExprResult<()> {
    assert_eq!(check("null == 1")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_neq_with_text_and_number() -> crate::ExprResult<()> {
    assert_eq!(check("\"hello\" != 42")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_eq_with_different_numeric_types() -> crate::ExprResult<()> {
    assert_eq!(check("1 == 3.14")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_neq_with_bool_and_text() -> crate::ExprResult<()> {
    assert_eq!(check("true != \"hello\"")?, ExprType::Bool);
    Ok(())
}

// --- Unknown type passthrough tests ---

#[test]
fn typecheck_expr_allows_unknown_in_comparison_both_sides() -> crate::ExprResult<()> {
    assert_eq!(check("$x < $y")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_unknown_in_logical_or() -> crate::ExprResult<()> {
    assert_eq!(check("$x or $y")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_unknown_with_f64_in_arithmetic_result() -> crate::ExprResult<()> {
    assert_eq!(check("$x + 2.5")?, ExprType::F64);
    Ok(())
}

// --- Helper type inference tests ---

#[test]
fn typecheck_expr_infers_boolean_helpers_extended() -> crate::ExprResult<()> {
    assert_eq!(check("starts_with($x, $y)")?, ExprType::Bool);
    assert_eq!(check("ends_with($x, $y)")?, ExprType::Bool);
    assert_eq!(check("has($x, $y)")?, ExprType::Bool);
    assert_eq!(check("exists($x)")?, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_infers_count_as_i64() -> crate::ExprResult<()> {
    assert_eq!(check("count($x)")?, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_infers_append_as_list() -> crate::ExprResult<()> {
    assert_eq!(check("append($x, $y)")?, ExprType::List);
    Ok(())
}

#[test]
fn typecheck_expr_infers_append_if_as_list() -> crate::ExprResult<()> {
    assert_eq!(check("append_if($x, $y, true)")?, ExprType::List);
    Ok(())
}

#[test]
fn typecheck_expr_propagates_type_error_from_helper_arg() -> crate::ExprResult<()> {
    let tokens = lex_expr("sum(\"hello\" + 1)")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch in sum arg".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_allows_helper_with_mismatched_arg_types_at_typecheck_level()
-> crate::ExprResult<()> {
    let ty = check("sum(true)")?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

// --- Nested expression tests ---

#[test]
fn typecheck_expr_infers_nested_arithmetic_result() -> crate::ExprResult<()> {
    assert_eq!(check("1 + 2 * 3")?, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_infers_deeply_nested_coercion() -> crate::ExprResult<()> {
    assert_eq!(check("1 + 2 * 3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn typecheck_expr_infers_nested_logical_with_comparison() -> crate::ExprResult<()> {
    assert_eq!(check("1 < 2 and 3 > 0")?, ExprType::Bool);
    Ok(())
}

// --- F64 negation ---

#[test]
fn typecheck_expr_infers_negation_on_f64_preserves_type() -> crate::ExprResult<()> {
    assert_eq!(check("-3.14")?, ExprType::F64);
    Ok(())
}

// --- Expression that produces object type ---

#[test]
fn typecheck_expr_infers_merge_as_object() -> crate::ExprResult<()> {
    assert_eq!(check("merge($x, $y)")?, ExprType::Object);
    Ok(())
}

#[test]
fn typecheck_expr_infers_unique_as_list() -> crate::ExprResult<()> {
    assert_eq!(check("unique($x)")?, ExprType::List);
    Ok(())
}
