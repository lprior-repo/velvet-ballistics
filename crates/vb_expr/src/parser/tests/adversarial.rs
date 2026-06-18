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
    unused_variables
)]
#![allow(dead_code, unused_imports)]
#![forbid(unsafe_code)]
//! Adversarial parser tests.

use crate::ExprError;
use crate::lexer::{BinaryOp, UnaryOp, lex_expr};
use crate::parser::{ExprAst, ExprHelper, ExprLiteral, parse_expr};

fn parse(source: &str) -> crate::ExprResult<ExprAst> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)
}

fn as_binary(expr: &ExprAst) -> crate::ExprResult<(BinaryOp, &ExprAst, &ExprAst)> {
    match expr {
        ExprAst::Binary { op, left, right } => Ok((*op, left, right)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected binary, got {other:?}"),
        }),
    }
}

fn as_unary(expr: &ExprAst) -> crate::ExprResult<(UnaryOp, &ExprAst)> {
    match expr {
        ExprAst::Unary { op, expr } => Ok((*op, expr)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected unary, got {other:?}"),
        }),
    }
}

fn as_helper(expr: &ExprAst) -> crate::ExprResult<(ExprHelper, &[ExprAst])> {
    match expr {
        ExprAst::Helper { name, args } => Ok((*name, args)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected helper, got {other:?}"),
        }),
    }
}

#[test]
fn parse_expr_chained_unary_not_true() -> crate::ExprResult<()> {
    let expr = parse("not not not not true")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Not);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Not);
    let (op3, inner3) = as_unary(inner2)?;
    assert_eq!(op3, UnaryOp::Not);
    let (op4, inner4) = as_unary(inner3)?;
    assert_eq!(op4, UnaryOp::Not);
    assert_eq!(*inner4, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

#[test]
fn parse_expr_double_negation_parses_correctly() -> crate::ExprResult<()> {
    let expr = parse("--5")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Neg);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Neg);
    assert_eq!(*inner2, ExprAst::Literal(ExprLiteral::I64(5)));
    Ok(())
}

#[test]
fn parse_expr_rejects_trailing_operator() -> crate::ExprResult<()> {
    let result = parse("1 +");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "trailing operator should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_double_operator() -> crate::ExprResult<()> {
    let result = parse("1 + * 2");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "double operator should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_deeply_nested_parentheses_within_limit() -> crate::ExprResult<()> {
    let expr = parse("(((((((1 + 2)))))))")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(2)));
    Ok(())
}

#[test]
fn parse_expr_rejects_empty_parentheses() -> crate::ExprResult<()> {
    let result = parse("()");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "empty parentheses should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_extra_right_paren() -> crate::ExprResult<()> {
    let result = parse("1)");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "trailing right paren should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_unknown_identifier_without_paren() -> crate::ExprResult<()> {
    let result = parse("foo");
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("unknown identifier"),
        "token should mention unknown identifier, got: {token}"
    );
    Ok(())
}

#[test]
fn parse_expr_null_equality_parses_as_binary_eq() -> crate::ExprResult<()> {
    let expr = parse("null == null")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Eq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::Null));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Null));
    Ok(())
}

#[test]
fn parse_expr_rejects_helper_with_too_many_args() -> crate::ExprResult<()> {
    let result = parse("contains(1, 2, 3, 4, 5, 6, 7, 8, 9)");
    assert!(
        matches!(result, Err(ExprError::TooManyHelperArgs { len: 9, max: 8 })),
        "9 helper args should exceed the 8-arg limit"
    );
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- parser
// =========================================================================

/// BH-PA-001: Deep nesting does not cause stack overflow in the parser.
#[test]
fn blackhat_pa_001_deep_nesting_no_crash() {
    let depth = usize::from(crate::parser::MAX_DEPTH).saturating_add(2);
    let open = "(".repeat(depth);
    let close = ")".repeat(depth);
    let source = format!("{open}true{close}");
    let result = parse(&source);
    assert!(
        matches!(result, Err(ExprError::ParseDepthExceeded { .. })),
        "BH-PA-001: deeply nested parens must hit depth limit"
    );
}

/// BH-PA-002: Unknown identifier without parens is rejected.
#[test]
fn blackhat_pa_002_unknown_identifier_rejected() -> crate::ExprResult<()> {
    let result = parse("foo");
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(token.contains("unknown identifier"), "got: {token}");
    Ok(())
}

/// BH-PA-003: Helper arity mismatch produces typed error.
#[test]
fn blackhat_pa_003_helper_arity_mismatch() -> crate::ExprResult<()> {
    let result = parse("contains(1)");
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch".into(),
        });
    };
    assert_eq!(helper, "contains");
    assert_eq!(expected, 2);
    assert_eq!(actual, 1);
    Ok(())
}

/// BH-PA-004: Trailing operator produces error.
#[test]
fn blackhat_pa_004_trailing_operator() {
    let result = parse("1 +");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "BH-PA-004: trailing operator should error"
    );
}

/// BH-PA-005: Empty parentheses rejected.
#[test]
fn blackhat_pa_005_empty_parens_rejected() {
    let result = parse("()");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "BH-PA-005: empty parens should error"
    );
}

/// BH-PA-006: Double operator rejected.
#[test]
fn blackhat_pa_006_double_operator_rejected() {
    let result = parse("1 + * 2");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "BH-PA-006: double operator should error"
    );
}

/// BH-PA-007: Extra right paren rejected.
#[test]
fn blackhat_pa_007_extra_rparen_rejected() {
    let result = parse("1)");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "BH-PA-007: extra right paren should error"
    );
}

/// BH-PA-008: Missing right paren rejected.
#[test]
fn blackhat_pa_008_missing_rparen_rejected() {
    let result = parse("(1 + 2");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "BH-PA-008: missing right paren should error"
    );
}

// =========================================================================
// Adversarial: boundary conditions
// =========================================================================

/// Parsing at exact MAX_DEPTH succeeds.
#[test]
fn parse_expr_at_max_depth_succeeds() -> crate::ExprResult<()> {
    let depth = usize::from(crate::parser::MAX_DEPTH);
    let open = "(".repeat(depth);
    let close = ")".repeat(depth);
    let source = format!("{open}42{close}");
    let expr = parse(&source)?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::I64(42)));
    Ok(())
}

/// Parsing at MAX_DEPTH + 1 fails.
#[test]
fn parse_expr_one_past_max_depth_fails() {
    let depth = usize::from(crate::parser::MAX_DEPTH).saturating_add(1);
    let open = "(".repeat(depth);
    let close = ")".repeat(depth);
    let source = format!("{open}42{close}");
    let result = parse(&source);
    assert!(
        matches!(result, Err(ExprError::ParseDepthExceeded { .. })),
        "MAX_DEPTH + 1 must fail"
    );
}

/// 8 args on a 2-arity helper is rejected as arity mismatch.
#[test]
fn parse_expr_eight_args_on_two_arity_helper_is_arity_mismatch() -> crate::ExprResult<()> {
    let result = parse("contains(1, 2, 3, 4, 5, 6, 7, 8)");
    if let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    {
        assert_eq!(helper, "contains");
        assert_eq!(expected, 2);
        assert_eq!(actual, 8);
    } else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch".into(),
        });
    }
    Ok(())
}

/// Max helper args on a 1-arity helper is rejected as arity mismatch.
#[test]
fn parse_expr_eight_args_on_unary_helper_is_arity_mismatch() -> crate::ExprResult<()> {
    let result = parse("exists(1, 2, 3, 4, 5, 6, 7, 8)");
    if let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    {
        assert_eq!(helper, "exists");
        assert_eq!(expected, 1);
        assert_eq!(actual, 8);
    } else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch".into(),
        });
    }
    Ok(())
}

// =========================================================================
// Adversarial: unary combinatorics
// =========================================================================

/// Many chained negations.
#[test]
fn parse_expr_many_chained_negations() -> crate::ExprResult<()> {
    let expr = parse("-----42")?;
    let mut current = &expr;
    for _ in 0..5 {
        let (op, inner) = as_unary(current)?;
        assert_eq!(op, UnaryOp::Neg);
        current = inner;
    }
    assert_eq!(*current, ExprAst::Literal(ExprLiteral::I64(42)));
    Ok(())
}

/// Mixed not and neg chained.
#[test]
fn parse_expr_mixed_not_and_neg_chain() -> crate::ExprResult<()> {
    let expr = parse("not - not - true")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Not);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Neg);
    let (op3, inner3) = as_unary(inner2)?;
    assert_eq!(op3, UnaryOp::Not);
    let (op4, inner4) = as_unary(inner3)?;
    assert_eq!(op4, UnaryOp::Neg);
    assert_eq!(*inner4, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

/// Unary not on helper call.
#[test]
fn parse_expr_not_on_helper_call() -> crate::ExprResult<()> {
    let expr = parse("not exists($x)")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Not);
    let (name, args) = as_helper(inner)?;
    assert_eq!(name, ExprHelper::Exists);
    assert_eq!(args.len(), 1);
    Ok(())
}

/// Unary neg on helper call.
#[test]
fn parse_expr_neg_on_helper_call() -> crate::ExprResult<()> {
    let expr = parse("-length($items)")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    let (name, args) = as_helper(inner)?;
    assert_eq!(name, ExprHelper::Length);
    assert_eq!(args.len(), 1);
    Ok(())
}

// =========================================================================
// Adversarial: binary combinatorics
// =========================================================================

/// Ternary-equivalent binary chain with correct associativity.
#[test]
fn parse_expr_long_left_assoc_chain() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 + 3 + 4 + 5")?;
    let (op1, left1, _right1) = as_binary(&expr)?;
    assert_eq!(op1, BinaryOp::Add);
    let (op2, left2, _right2) = as_binary(left1)?;
    assert_eq!(op2, BinaryOp::Add);
    let (op3, left3, _right3) = as_binary(left2)?;
    assert_eq!(op3, BinaryOp::Add);
    let (op4, left4, _right4) = as_binary(left3)?;
    assert_eq!(op4, BinaryOp::Add);
    assert_eq!(*left4, ExprAst::Literal(ExprLiteral::I64(1)));
    Ok(())
}

/// Mixed right-associative implicit from precedence: a + b * c + d
#[test]
fn parse_expr_mixed_precedence_chain() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3 + 4 / 2")?;
    let (op1, left1, right1) = as_binary(&expr)?;
    assert_eq!(op1, BinaryOp::Add);
    let (op2, left2, right2) = as_binary(left1)?;
    assert_eq!(op2, BinaryOp::Add);
    assert_eq!(*left2, ExprAst::Literal(ExprLiteral::I64(1)));
    let (mul_op, _, _) = as_binary(right2)?;
    assert_eq!(mul_op, BinaryOp::Mul);
    let (div_op, _, _) = as_binary(right1)?;
    assert_eq!(div_op, BinaryOp::Div);
    Ok(())
}

/// All comparison operators in a chain (each pair).
#[test]
fn parse_expr_all_comparisons_in_one_expr() -> crate::ExprResult<()> {
    let expr = parse("1 < 2 and 3 <= 4 and 5 > 4 and 6 >= 5 and 7 == 7 and 8 != 9")?;
    let mut cur = &expr;
    for _ in 0..5 {
        let (op, left, right) = as_binary(cur)?;
        assert_eq!(op, BinaryOp::And);
        cur = left;
        let _ = right;
    }
    let (op_last, _l, _r) = as_binary(cur)?;
    assert!(matches!(
        op_last,
        BinaryOp::Lt
            | BinaryOp::Lte
            | BinaryOp::Gt
            | BinaryOp::Gte
            | BinaryOp::Eq
            | BinaryOp::NotEq
    ));
    Ok(())
}

/// Or binds looser than And with three terms.
#[test]
fn parse_expr_a_and_b_or_c_and_d() -> crate::ExprResult<()> {
    let expr = parse("true and false or false and true")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::And);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::And);
    Ok(())
}

// =========================================================================
// Adversarial: complex expressions
// =========================================================================

/// Helper call used as binary operand.
#[test]
fn parse_expr_helper_in_binary_expression() -> crate::ExprResult<()> {
    let expr = parse("length($x) > 0 and empty($y) == false")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::And);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::Gt);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::Eq);
    Ok(())
}

/// Nested parenthesized binary inside helper args.
#[test]
fn parse_expr_parens_inside_helper_args() -> crate::ExprResult<()> {
    let expr = parse("contains((1 + 2), (3 * 4))")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    Ok(())
}

/// String literal equality.
#[test]
fn parse_expr_string_eq_string() -> crate::ExprResult<()> {
    let expr = parse("\"abc\" == \"abc\"")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Eq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::Text(Box::from("abc"))));
    assert_eq!(
        *right,
        ExprAst::Literal(ExprLiteral::Text(Box::from("abc")))
    );
    Ok(())
}

/// Float in expression with all numeric operators.
#[test]
fn parse_expr_float_with_all_arith_ops() -> crate::ExprResult<()> {
    let expr = parse("1.5 + 2.5 * 3.0 / 1.0 - 0.5")?;
    let (op1, left1, _right1) = as_binary(&expr)?;
    assert_eq!(op1, BinaryOp::Sub);
    let (op2, _left2, _right2) = as_binary(left1)?;
    assert_eq!(op2, BinaryOp::Add);
    Ok(())
}

/// Null in comparisons.
#[test]
fn parse_expr_null_in_comparison() -> crate::ExprResult<()> {
    let expr = parse("null != true")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::NotEq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::Null));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

/// Reference as unary neg operand.
#[test]
fn parse_expr_neg_on_reference() -> crate::ExprResult<()> {
    let expr = parse("-$value")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Reference(Box::from("$value")));
    Ok(())
}

/// Maximum depth within helper args.
#[test]
fn parse_expr_deeply_nested_in_helper_args() -> crate::ExprResult<()> {
    let depth = usize::from(crate::parser::MAX_DEPTH).saturating_sub(2);
    let open = "(".repeat(depth);
    let close = ")".repeat(depth);
    let source = format!("contains({open}1{close}, 2)");
    let expr = parse(&source)?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    Ok(())
}

// =========================================================================
// Adversarial: overflow and limits
// =========================================================================

/// Very large integer at i64 boundary.
#[test]
fn parse_expr_max_i64() -> crate::ExprResult<()> {
    let expr = parse("9223372036854775807")?;
    assert_eq!(
        expr,
        ExprAst::Literal(ExprLiteral::I64(9223372036854775807))
    );
    Ok(())
}

/// Negative i64::MAX as unary neg (i64::MIN is special in lexer).
#[test]
fn parse_expr_neg_max_i64() -> crate::ExprResult<()> {
    let expr = parse("-9223372036854775807")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(
        *inner,
        ExprAst::Literal(ExprLiteral::I64(9223372036854775807))
    );
    Ok(())
}

/// Many unary ops approaching but within depth limit.
#[test]
fn parse_expr_many_unary_ops_within_depth() -> crate::ExprResult<()> {
    let depth = usize::from(crate::parser::MAX_DEPTH);
    let source = format!("{}true", "not ".repeat(depth));
    let expr = parse(&source)?;
    let mut current = &expr;
    for _ in 0..depth {
        let (op, inner) = as_unary(current)?;
        assert_eq!(op, UnaryOp::Not);
        current = inner;
    }
    assert_eq!(*current, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

// =========================================================================
// Adversarial: malformation edge cases
// =========================================================================

/// Unclosed paren with multiple tokens inside.
#[test]
fn parse_expr_rejects_unclosed_paren_with_content() {
    let result = parse("(1 + 2 * 3");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "unclosed paren with content should error"
    );
}

/// Two consecutive open parens then a close.
#[test]
fn parse_expr_rejects_extra_open_paren() {
    let result = parse("((1)");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "mismatched parens should error"
    );
}

/// Expression containing a bare comma.
#[test]
fn parse_expr_rejects_bare_comma() {
    let result = parse("1 , 2");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "bare comma in expression should error"
    );
}

/// Valid expression followed by extra tokens.
#[test]
fn parse_expr_rejects_extra_tokens_after_valid_expr() {
    let result = parse("1 + 2 3");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "extra tokens after valid expr should error"
    );
}

/// Expression that exceeds MAX_DEPTH via unary nesting.
#[test]
fn parse_expr_unary_nesting_exceeding_max_depth() {
    let depth = usize::from(crate::parser::MAX_DEPTH).saturating_add(2);
    let source = format!("{}true", "not ".repeat(depth));
    let result = parse(&source);
    assert!(
        matches!(result, Err(ExprError::ParseDepthExceeded { .. })),
        "excessive unary nesting must hit depth limit"
    );
}

// =========================================================================
// BLACKHAT security regression tests (continued)
// =========================================================================

/// BH-PA-009: Too many helper args rejected at boundary.
#[test]
fn blackhat_pa_009_too_many_helper_args_at_boundary() {
    let result = parse("contains(1, 2, 3, 4, 5, 6, 7, 8, 9)");
    assert!(
        matches!(result, Err(ExprError::TooManyHelperArgs { len: 9, max: 8 })),
        "BH-PA-009: 9 args should hit TooManyHelperArgs"
    );
}

/// BH-PA-010: Unknown helper within binary expression.
#[test]
fn blackhat_pa_010_unknown_helper_in_binary_expr() {
    let result = parse("bad_func(1) and true");
    assert!(
        matches!(result, Err(ExprError::UnknownHelper { .. })),
        "BH-PA-010: unknown helper should be rejected"
    );
}

/// BH-PA-011: Deeply nested binary left-recursion stays within depth.
#[test]
fn blackhat_pa_011_deep_binary_chain_no_overflow() -> crate::ExprResult<()> {
    let depth = usize::from(crate::parser::MAX_DEPTH);
    let parts: Vec<String> = (0..=depth).map(|i| i.to_string()).collect();
    let source = parts.join(" + ");
    let expr = parse(&source)?;
    let (op, _, _) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    Ok(())
}

/// BH-PA-012: Empty string literal parses correctly.
#[test]
fn blackhat_pa_012_empty_string_literal() -> crate::ExprResult<()> {
    let expr = parse("\"\"")?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::Text(Box::from(""))));
    Ok(())
}

/// BH-PA-013: Helper with deeply nested expression argument.
#[test]
fn blackhat_pa_013_helper_with_deep_arg_expr() -> crate::ExprResult<()> {
    let depth = usize::from(crate::parser::MAX_DEPTH).saturating_sub(3);
    let open = "(".repeat(depth);
    let close = ")".repeat(depth);
    let source = format!("exists({open}1 + 2{close})");
    let expr = parse(&source)?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Exists);
    assert_eq!(args.len(), 1);
    Ok(())
}

/// BH-PA-014: Multi-digit integer edge case.
#[test]
fn blackhat_pa_014_multi_digit_integers() -> crate::ExprResult<()> {
    let expr = parse("100 + 9999")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(100)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(9999)));
    Ok(())
}

/// BH-PA-015: Negation of zero.
#[test]
fn blackhat_pa_015_negation_of_zero() -> crate::ExprResult<()> {
    let expr = parse("-0")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(0)));
    Ok(())
}
