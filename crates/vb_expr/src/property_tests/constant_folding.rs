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
    unused_variables,
)]

// Property tests for constant_folding (CF) — vb_expr
// Tests: const_fold_expr returns correct Option<ConstValue> for all literal expressions.
// Coverage: CF-1..CF-18 from test-plan §1.1.

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::{ExprAst, ExprLiteral};
use vb_core::ConstValue;
use vb_core::value::FiniteF64;

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// CF-1..CF-5: Literal folding
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_literal_bool_true_folds_to_some_bool_true(_unit in Just(())) {
        let lit = ExprLiteral::Bool(true);
        let result = crate::bytecode::const_fold_expr(&ExprAst::Literal(lit));
        prop_assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn cf_literal_bool_false_folds_to_some_bool_false(_unit in Just(())) {
        let lit = ExprLiteral::Bool(false);
        let result = crate::bytecode::const_fold_expr(&ExprAst::Literal(lit));
        prop_assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn cf_literal_null_folds_to_some_null(_unit in Just(())) {
        let lit = ExprLiteral::Null;
        let result = crate::bytecode::const_fold_expr(&ExprAst::Literal(lit));
        prop_assert_eq!(result, Some(ConstValue::Null));
    }
}

proptest! {
    #[test]
    fn cf_literal_i64_folds_to_some_i64(val in any::<i64>()) {
        let ast = ExprAst::Literal(ExprLiteral::I64(val));
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::I64(val)));
    }
}

// ---------------------------------------------------------------------------
// CF-6: not true = false, not false = true
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_not_true_folds_to_false(_unit in Just(())) {
        let ast = ExprAst::Unary { op: UnaryOp::Not, expr: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn cf_not_false_folds_to_true(_unit in Just(())) {
        let ast = ExprAst::Unary { op: UnaryOp::Not, expr: Box::new(ExprAst::Literal(ExprLiteral::Bool(false))) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(true)));
    }
}

// ---------------------------------------------------------------------------
// CF-7..CF-8: Reference and helper do not fold
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_reference_does_not_fold(name in "[a-z][a-z0-9_]{1,20}") {
        let ast = ExprAst::Reference(format!("${}", name).into());
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, None);
    }
}

// ---------------------------------------------------------------------------
// CF-9..CF-10: Boolean short-circuit folding
// ---------------------------------------------------------------------------

#[test]
fn cf_and_true_true_folds_to_true() {
    let ast = ExprAst::Binary {
        op: BinaryOp::And,
        left: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
    };
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, Some(ConstValue::Bool(true)));
}

#[test]
fn cf_and_false_any_folds_to_false() {
    let ast = ExprAst::Binary {
        op: BinaryOp::And,
        left: Box::new(ExprAst::Literal(ExprLiteral::Bool(false))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
    };
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, Some(ConstValue::Bool(false)));
}

#[test]
fn cf_or_false_false_folds_to_false() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Or,
        left: Box::new(ExprAst::Literal(ExprLiteral::Bool(false))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(false))),
    };
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, Some(ConstValue::Bool(false)));
}

#[test]
fn cf_or_true_any_folds_to_true() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Or,
        left: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(false))),
    };
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, Some(ConstValue::Bool(true)));
}

// ---------------------------------------------------------------------------
// CF-11..CF-14: Arithmetic folding (Add, Sub, Mul, Div)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_add_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Add, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        match a.checked_add(b) {
            Some(sum) => prop_assert_eq!(result, Some(ConstValue::I64(sum))),
            None => prop_assert_eq!(result, None),
        }
    }

    #[test]
    fn cf_sub_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Sub, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        match a.checked_sub(b) {
            Some(diff) => prop_assert_eq!(result, Some(ConstValue::I64(diff))),
            None => prop_assert_eq!(result, None),
        }
    }

    #[test]
    fn cf_mul_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Mul, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        match a.checked_mul(b) {
            Some(prod) => prop_assert_eq!(result, Some(ConstValue::I64(prod))),
            None => prop_assert_eq!(result, None),
        }
    }

    #[test]
    fn cf_div_i64_folds_correctly(a: i64, b: i64) {
        prop_assume!(b != 0);
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Div, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        match a.checked_div(b) {
            Some(quot) => prop_assert_eq!(result, Some(ConstValue::I64(quot))),
            None => prop_assert_eq!(result, None),
        }
    }
}

// ---------------------------------------------------------------------------
// CF-15..CF-20: Comparison folding (Eq, NotEq, Lt, Lte, Gt, Gte)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_eq_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Eq, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a == b)));
    }

    #[test]
    fn cf_noteq_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::NotEq, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a != b)));
    }

    #[test]
    fn cf_lt_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Lt, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a < b)));
    }

    #[test]
    fn cf_lte_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Lte, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a <= b)));
    }

    #[test]
    fn cf_gt_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Gt, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a > b)));
    }

    #[test]
    fn cf_gte_i64_folds_correctly(a: i64, b: i64) {
        let left = ExprAst::Literal(ExprLiteral::I64(a));
        let right = ExprAst::Literal(ExprLiteral::I64(b));
        let ast = ExprAst::Binary { op: BinaryOp::Gte, left: Box::new(left), right: Box::new(right) };
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, Some(ConstValue::Bool(a >= b)));
    }
}

// ---------------------------------------------------------------------------
// CF-21: Negation folding
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_neg_i64_folds_correctly(val in any::<i64>()) {
        let inner = ExprAst::Literal(ExprLiteral::I64(val));
        let ast = ExprAst::Unary { op: UnaryOp::Neg, expr: Box::new(inner) };
        let result = crate::bytecode::const_fold_expr(&ast);
        match val.checked_neg() {
            Some(negated) => prop_assert_eq!(result, Some(ConstValue::I64(negated))),
            None => prop_assert_eq!(result, None),
        }
    }
}

// ---------------------------------------------------------------------------
// CF-22..CF-23: i64::MIN / -1 and -i64::MIN overflow (special cases)
// ---------------------------------------------------------------------------

#[test]
fn cf_div_i64_min_by_neg_one_returns_none() {
    let left = ExprAst::Literal(ExprLiteral::I64(i64::MIN));
    let right = ExprAst::Literal(ExprLiteral::I64(-1));
    let ast = ExprAst::Binary {
        op: BinaryOp::Div,
        left: Box::new(left),
        right: Box::new(right),
    };
    // i64::MIN / -1 overflows
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, None);
}

#[test]
fn cf_neg_i64_min_returns_none() {
    let inner = ExprAst::Literal(ExprLiteral::I64(i64::MIN));
    let ast = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(inner),
    };
    // -i64::MIN overflows
    let result = crate::bytecode::const_fold_expr(&ast);
    assert_eq!(result, None);
}

// ---------------------------------------------------------------------------
// CF-24: Non-i64 operands do not fold arithmetic
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_add_f64_does_not_fold(_unit in Just(())) {
        let f64_val = FiniteF64::new(1.5).expect("valid finite f64");
        let left = ExprAst::Literal(ExprLiteral::F64(f64_val));
        let right = ExprAst::Literal(ExprLiteral::I64(2));
        let ast = ExprAst::Binary { op: BinaryOp::Add, left: Box::new(left), right: Box::new(right) };
        // Mixed types do not fold
        let result = crate::bytecode::const_fold_expr(&ast);
        prop_assert_eq!(result, None);
    }
}

#[test]
fn cf_neg_f64_does_not_fold() {
    // F64 negation is not constant-folded (fold_unary only handles I64)
    let f64_val = FiniteF64::new(1.5).expect("valid finite f64");
    let inner = ExprAst::Literal(ExprLiteral::F64(f64_val));
    let ast = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(inner),
    };
    let result = crate::bytecode::const_fold_expr(&ast);
    // fold_unary only handles Neg on I64, not F64
    assert_eq!(result, None);
}

// ---------------------------------------------------------------------------
// CF-25: Nested expressions fold correctly
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cf_nested_arithmetic_folds_correctly(a: i64, b: i64, c: i64) {
        // (a + b) * c
        let left = ExprAst::Binary {
            op: BinaryOp::Add,
            left: Box::new(ExprAst::Literal(ExprLiteral::I64(a))),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(b))),
        };
        let ast = ExprAst::Binary {
            op: BinaryOp::Mul,
            left: Box::new(left),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(c))),
        };
        let result = crate::bytecode::const_fold_expr(&ast);
        match a.checked_add(b).and_then(|sum| sum.checked_mul(c)) {
            Some(val) => prop_assert_eq!(result, Some(ConstValue::I64(val))),
            None => prop_assert_eq!(result, None),
        }
    }
}
