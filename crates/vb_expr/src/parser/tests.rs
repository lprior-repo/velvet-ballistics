#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons)]

#![forbid(unsafe_code)]
//! BDD parser tests.

#[allow(unused_imports)]
use crate::ExprError;
use crate::lexer::{BinaryOp, UnaryOp, lex_expr};
#[allow(unused_imports)]
use crate::parser::{ExprAst, ExprHelper, ExprLiteral, parse_expr};

mod adversarial;

#[allow(dead_code)]
fn parse(source: &str) -> crate::ExprResult<ExprAst> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)
}

#[test]
fn parses_addition_with_multiplication_precedence() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3")?;
    let (op, _, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    let (right_op, _, _) = as_binary(right)?;
    assert_eq!(right_op, BinaryOp::Mul);
    Ok(())
}

#[test]
fn parses_left_associative_subtraction() -> crate::ExprResult<()> {
    let expr = parse("1 - 2 - 3")?;
    let (op, left, _) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Sub);
    let (left_op, _, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::Sub);
    Ok(())
}

#[test]
fn parses_not_and_or_precedence() -> crate::ExprResult<()> {
    let expr = parse("not $a and $b or $c")?;
    let (op, left, _) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    let (left_op, not_expr, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::And);
    let (not_op, _) = as_unary(not_expr)?;
    assert_eq!(not_op, UnaryOp::Not);
    Ok(())
}

#[test]
fn parses_helper_call() -> crate::ExprResult<()> {
    let expr = parse("contains($tags, \"urgent\")")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn rejects_unknown_helper() {
    let result = parse("unknown_func(1)");
    assert!(matches!(result, Err(ExprError::UnknownHelper { .. })));
}

#[test]
fn rejects_wrong_arity() {
    let result = parse("contains(1)");
    assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
}

#[test]
fn rejects_parse_depth() {
    let open = "(".repeat(usize::from(crate::parser::MAX_DEPTH).saturating_add(2));
    let close = ")".repeat(usize::from(crate::parser::MAX_DEPTH).saturating_add(2));
    let source = format!("{open}true{close}");
    let result = parse(&source);
    assert!(matches!(result, Err(ExprError::ParseDepthExceeded { .. })));
}

// --- BDD parser tests ---

#[test]
fn parse_expr_parses_simple_addition() -> crate::ExprResult<()> {
    let expr = parse("5 + 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
    Ok(())
}

#[test]
fn parse_expr_parses_operator_precedence_correctly() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
    let (inner_op, _, _) = as_binary(right)?;
    assert_eq!(inner_op, BinaryOp::Mul);
    Ok(())
}

#[test]
fn parse_expr_parses_parenthesized_grouping() -> crate::ExprResult<()> {
    let expr = parse("(1 + 2) * 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Mul);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
    let (inner_op, _, _) = as_binary(left)?;
    assert_eq!(inner_op, BinaryOp::Add);
    Ok(())
}

#[test]
fn parse_expr_parses_unary_negation() -> crate::ExprResult<()> {
    let expr = parse("-5")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(5)));
    Ok(())
}

#[test]
fn parse_expr_parses_boolean_not() -> crate::ExprResult<()> {
    let expr = parse("not true")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Not);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

#[test]
fn parse_expr_parses_comparison_operators() -> crate::ExprResult<()> {
    let expr = parse("5 == 5")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Eq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(5)));

    let expr_ne = parse("5 != 3")?;
    let (op_ne, _, _) = as_binary(&expr_ne)?;
    assert_eq!(op_ne, BinaryOp::NotEq);

    let expr_lt = parse("1 < 2")?;
    let (op_lt, _, _) = as_binary(&expr_lt)?;
    assert_eq!(op_lt, BinaryOp::Lt);

    let expr_gt = parse("2 > 1")?;
    let (op_gt, _, _) = as_binary(&expr_gt)?;
    assert_eq!(op_gt, BinaryOp::Gt);

    let expr_lte = parse("1 <= 2")?;
    let (op_lte, _, _) = as_binary(&expr_lte)?;
    assert_eq!(op_lte, BinaryOp::Lte);

    let expr_gte = parse("2 >= 1")?;
    let (op_gte, _, _) = as_binary(&expr_gte)?;
    assert_eq!(op_gte, BinaryOp::Gte);
    Ok(())
}

#[test]
fn parse_expr_parses_logical_and_or() -> crate::ExprResult<()> {
    let expr = parse("true and false or true")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Bool(true)));
    let (left_op, _, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::And);
    Ok(())
}

#[test]
fn parse_expr_parses_helper_call_with_arguments() -> crate::ExprResult<()> {
    let expr = parse("contains($x, $y)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    assert_eq!(args.first(), Some(&ExprAst::Reference(Box::from("$x"))));
    assert_eq!(args.get(1), Some(&ExprAst::Reference(Box::from("$y"))));
    Ok(())
}

#[test]
fn parse_expr_parses_variable_reference() -> crate::ExprResult<()> {
    let expr = parse("$data.field")?;
    assert_eq!(expr, ExprAst::Reference(Box::from("$data.field")));
    Ok(())
}

// --- extended BDD: literal types ---

#[test]
fn parse_expr_parses_null_literal() -> crate::ExprResult<()> {
    let expr = parse("null")?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::Null));
    Ok(())
}

#[test]
fn parse_expr_parses_boolean_literal_true_and_false() -> crate::ExprResult<()> {
    assert_eq!(parse("true")?, ExprAst::Literal(ExprLiteral::Bool(true)));
    assert_eq!(parse("false")?, ExprAst::Literal(ExprLiteral::Bool(false)));
    Ok(())
}

#[test]
fn parse_expr_parses_integer_literals() -> crate::ExprResult<()> {
    assert_eq!(parse("0")?, ExprAst::Literal(ExprLiteral::I64(0)));
    assert_eq!(parse("42")?, ExprAst::Literal(ExprLiteral::I64(42)));
    let neg_expr = parse("-7")?;
    let (op, inner) = as_unary(&neg_expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(7)));
    Ok(())
}

#[test]
fn parse_expr_parses_text_literal() -> crate::ExprResult<()> {
    let expr = parse("\"hello world\"")?;
    assert_eq!(
        expr,
        ExprAst::Literal(ExprLiteral::Text(Box::from("hello world")))
    );
    Ok(())
}

#[test]
fn parse_expr_parses_text_literal_empty_string() -> crate::ExprResult<()> {
    let expr = parse("\"\"")?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::Text(Box::from(""))));
    Ok(())
}

// --- extended BDD: binary operators ---

#[test]
fn parse_expr_parses_subtraction() -> crate::ExprResult<()> {
    let expr = parse("10 - 4")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Sub);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(10)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(4)));
    Ok(())
}

#[test]
fn parse_expr_parses_multiplication() -> crate::ExprResult<()> {
    let expr = parse("6 * 7")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Mul);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(6)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(7)));
    Ok(())
}

#[test]
fn parse_expr_parses_division() -> crate::ExprResult<()> {
    let expr = parse("8 / 2")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Div);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(8)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(2)));
    Ok(())
}

#[test]
fn parse_expr_parses_and_without_or() -> crate::ExprResult<()> {
    let expr = parse("true and false")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::And);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::Bool(true)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Bool(false)));
    Ok(())
}

// --- extended BDD: precedence chains ---

#[test]
fn parse_expr_mul_binds_tighter_than_add_on_right() -> crate::ExprResult<()> {
    let expr = parse("2 * 3 + 4")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(4)));
    let (inner_op, _, _) = as_binary(left)?;
    assert_eq!(inner_op, BinaryOp::Mul);
    Ok(())
}

#[test]
fn parse_expr_div_binds_tighter_than_sub() -> crate::ExprResult<()> {
    let expr = parse("10 - 6 / 2")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Sub);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(10)));
    let (inner_op, _, _) = as_binary(right)?;
    assert_eq!(inner_op, BinaryOp::Div);
    Ok(())
}

#[test]
fn parse_expr_comparison_binds_tighter_than_and() -> crate::ExprResult<()> {
    let expr = parse("1 < 2 and 3 > 2")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::And);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::Lt);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::Gt);
    Ok(())
}

#[test]
fn parse_expr_eq_binds_tighter_than_or() -> crate::ExprResult<()> {
    let expr = parse("1 == 1 or 2 == 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::Eq);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::Eq);
    Ok(())
}

#[test]
fn parse_expr_add_binds_tighter_than_lt() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 < 5")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Lt);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(5)));
    let (inner_op, _, _) = as_binary(left)?;
    assert_eq!(inner_op, BinaryOp::Add);
    Ok(())
}

#[test]
fn parse_expr_full_precedence_tower_is_respected() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3 == 7 and 8 / 2 > 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::And);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::Eq);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::Gt);
    Ok(())
}

// --- extended BDD: parentheses ---

#[test]
fn parse_expr_single_parenthesized_literal() -> crate::ExprResult<()> {
    let expr = parse("(42)")?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::I64(42)));
    Ok(())
}

#[test]
fn parse_expr_double_parenthesized_literal() -> crate::ExprResult<()> {
    let expr = parse("((99))")?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::I64(99)));
    Ok(())
}

#[test]
fn parse_expr_parens_override_precedence_in_middle() -> crate::ExprResult<()> {
    let expr = parse("1 * (2 + 3)")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Mul);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
    let (inner_op, _, _) = as_binary(right)?;
    assert_eq!(inner_op, BinaryOp::Add);
    Ok(())
}

#[test]
fn parse_expr_nested_parens_with_binary_inside() -> crate::ExprResult<()> {
    let expr = parse("((5 * 2) + (3 / 1))")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    let (lop, _, _) = as_binary(left)?;
    assert_eq!(lop, BinaryOp::Mul);
    let (rop, _, _) = as_binary(right)?;
    assert_eq!(rop, BinaryOp::Div);
    Ok(())
}

// --- extended BDD: unary operators ---

#[test]
fn parse_expr_not_on_reference() -> crate::ExprResult<()> {
    let expr = parse("not $flag")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Not);
    assert_eq!(*inner, ExprAst::Reference(Box::from("$flag")));
    Ok(())
}

#[test]
fn parse_expr_neg_on_parenthesized_expr() -> crate::ExprResult<()> {
    let expr = parse("-(1 + 2)")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    let (inner_op, _, _) = as_binary(inner)?;
    assert_eq!(inner_op, BinaryOp::Add);
    Ok(())
}

#[test]
fn parse_expr_not_on_not_is_double_negation() -> crate::ExprResult<()> {
    let expr = parse("not not false")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Not);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Not);
    assert_eq!(*inner2, ExprAst::Literal(ExprLiteral::Bool(false)));
    Ok(())
}

#[test]
fn parse_expr_neg_on_int_creates_unary() -> crate::ExprResult<()> {
    let expr = parse("-42")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(42)));
    Ok(())
}

// --- extended BDD: mixed unary + binary ---

#[test]
fn parse_expr_unary_not_with_binary_and() -> crate::ExprResult<()> {
    let expr = parse("not $a and $b")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::And);
    assert_eq!(*right, ExprAst::Reference(Box::from("$b")));
    let (unary_op, _) = as_unary(left)?;
    assert_eq!(unary_op, UnaryOp::Not);
    Ok(())
}

#[test]
fn parse_expr_unary_neg_with_binary_mul() -> crate::ExprResult<()> {
    let expr = parse("-3 * 4")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    let (bin_op, left, right) = as_binary(inner)?;
    assert_eq!(bin_op, BinaryOp::Mul);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(3)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(4)));
    Ok(())
}

// --- extended BDD: helper calls ---

#[test]
fn parse_expr_helper_exists_arity_1() -> crate::ExprResult<()> {
    let expr = parse("exists($x)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Exists);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_length_arity_1() -> crate::ExprResult<()> {
    let expr = parse("length($x)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Length);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_empty_arity_1() -> crate::ExprResult<()> {
    let expr = parse("empty($x)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Empty);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_sum_arity_1() -> crate::ExprResult<()> {
    let expr = parse("sum($items)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Sum);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_count_arity_1() -> crate::ExprResult<()> {
    let expr = parse("count($items)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Count);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_unique_arity_1() -> crate::ExprResult<()> {
    let expr = parse("unique($items)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Unique);
    assert_eq!(args.len(), 1);
    Ok(())
}

#[test]
fn parse_expr_helper_merge_arity_2() -> crate::ExprResult<()> {
    let expr = parse("merge($a, $b)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Merge);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn parse_expr_helper_append_arity_2() -> crate::ExprResult<()> {
    let expr = parse("append($list, 1)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Append);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn parse_expr_helper_append_if_arity_3() -> crate::ExprResult<()> {
    let expr = parse("append_if($list, 1, true)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::AppendIf);
    assert_eq!(args.len(), 3);
    Ok(())
}

#[test]
fn parse_expr_helper_has_arity_2() -> crate::ExprResult<()> {
    let expr = parse("has($obj, $key)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Has);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn parse_expr_helper_starts_with_arity_2() -> crate::ExprResult<()> {
    let expr = parse("starts_with($s, \"hi\")")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::StartsWith);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn parse_expr_helper_ends_with_arity_2() -> crate::ExprResult<()> {
    let expr = parse("ends_with($s, \"lo\")")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::EndsWith);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn parse_expr_helper_with_zero_args_is_rejected() -> crate::ExprResult<()> {
    let result = parse("exists()");
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        panic!("expected HelperArityMismatch");
    };
    assert_eq!(helper, "exists");
    assert_eq!(expected, 1);
    assert_eq!(actual, 0);
    Ok(())
}

#[test]
fn parse_expr_helper_with_extra_args_is_rejected() -> crate::ExprResult<()> {
    let result = parse("exists(1, 2)");
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        panic!("expected HelperArityMismatch");
    };
    assert_eq!(helper, "exists");
    assert_eq!(expected, 1);
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn parse_expr_helper_call_with_expr_args() -> crate::ExprResult<()> {
    let expr = parse("contains(1 + 2, 3 * 4)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    let (a0_op, _, _) = as_binary(&args[0])?;
    assert_eq!(a0_op, BinaryOp::Add);
    let (a1_op, _, _) = as_binary(&args[1])?;
    assert_eq!(a1_op, BinaryOp::Mul);
    Ok(())
}

// --- extended BDD: error cases ---

#[test]
fn parse_expr_rejects_bare_identifier_without_parens() {
    let result = parse("foobar");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { ref token }) if token.contains("unknown identifier"))
    );
}

#[test]
fn parse_expr_rejects_unclosed_helper_paren() {
    let result = parse("contains(1, 2");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
}

#[test]
fn parse_expr_rejects_comma_after_last_helper_arg() {
    let result = parse("contains(1,)");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
}

#[test]
fn parse_expr_rejects_only_operator() {
    let result = parse("+");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
}

#[test]
fn parse_expr_rejects_stray_dollar() {
    let result = parse("$");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
}

#[test]
fn parse_expr_rejects_leading_operator_before_literal() -> crate::ExprResult<()> {
    let result = parse("* 5");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
    Ok(())
}

#[test]
fn parse_expr_rejects_consecutive_binary_operators() -> crate::ExprResult<()> {
    let result = parse("1 + * 2");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
    Ok(())
}

#[test]
fn parse_expr_rejects_triple_operator_chain() -> crate::ExprResult<()> {
    let result = parse("1 + - * 2");
    assert!(matches!(result, Err(ExprError::UnexpectedToken { .. })));
    Ok(())
}

// --- F64 literal parser tests ---

#[test]
fn parse_expr_parses_float_literal() -> crate::ExprResult<()> {
    let expr = parse("3.14")?;
    let finite = vb_core::FiniteF64::new(3.14)?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::F64(finite)));
    Ok(())
}

#[test]
fn parse_expr_parses_float_literal_with_zero_integer_part() -> crate::ExprResult<()> {
    let expr = parse("0.5")?;
    let finite = vb_core::FiniteF64::new(0.5)?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::F64(finite)));
    Ok(())
}

#[test]
fn parse_expr_parses_float_literal_large_value() -> crate::ExprResult<()> {
    let expr = parse("999.999")?;
    let finite = vb_core::FiniteF64::new(999.999)?;
    assert_eq!(expr, ExprAst::Literal(ExprLiteral::F64(finite)));
    Ok(())
}

#[test]
fn parse_expr_returns_error_for_empty_input() -> crate::ExprResult<()> {
    let tokens = lex_expr("")?;
    let result = parse_expr(&tokens);
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("End"),
        "token should contain 'End', got: {token}"
    );
    Ok(())
}

#[test]
fn parse_expr_returns_unknown_helper_for_bad_helper() -> crate::ExprResult<()> {
    let result = parse("bogus_func(1)");
    let Err(ExprError::UnknownHelper { helper }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnknownHelper".into(),
        });
    };
    assert_eq!(helper, "bogus_func");
    Ok(())
}

#[test]
fn parse_expr_returns_wrong_arity_error_for_contains_with_one_arg() -> crate::ExprResult<()> {
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

#[test]
fn parse_expr_returns_error_for_missing_right_paren() -> crate::ExprResult<()> {
    let result = parse("(1 + 2");
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("right parenthesis"),
        "token should mention right parenthesis, got: {token}"
    );
    Ok(())
}

#[allow(dead_code)]
fn as_binary(expr: &ExprAst) -> crate::ExprResult<(BinaryOp, &ExprAst, &ExprAst)> {
    match expr {
        ExprAst::Binary { op, left, right } => Ok((*op, left, right)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected binary, got {other:?}"),
        }),
    }
}

#[allow(dead_code)]
fn as_unary(expr: &ExprAst) -> crate::ExprResult<(UnaryOp, &ExprAst)> {
    match expr {
        ExprAst::Unary { op, expr } => Ok((*op, expr)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected unary, got {other:?}"),
        }),
    }
}

#[allow(dead_code)]
fn as_helper(expr: &ExprAst) -> crate::ExprResult<(ExprHelper, &[ExprAst])> {
    match expr {
        ExprAst::Helper { name, args } => Ok((*name, args)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected helper, got {other:?}"),
        }),
    }
}
