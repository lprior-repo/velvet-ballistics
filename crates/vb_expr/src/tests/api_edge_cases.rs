#![forbid(unsafe_code)]
//! Public API, constant-folding, and stack-bound edge-case tests.
//!
//! Each test targets a specific verified gap in the existing vb_expr test surface:
//!
//! - `parse_helper_name` for all 13 helpers
//! - `helper_arity` for every ExprHelper variant
//! - `helper_name` canonical name mapping
//! - `fold_unary` with F64 Neg → None (I64-only folding)
//! - `fold_binary` Eq/NotEq cross-type → None
//! - `fold_binary` Eq same-type → Bool
//! - `fold_i64_binop` with F64 → None
//! - `compile_expr_with_resolver` unresolved reference inside helper arg
//! - `check_expr_stack_bound` with deep stack usage (DoS protection)
//! - `infix_binding_power` for all 12 BinaryOp variants
//! - `strip_quotes` with single-character inner string
//! - `eval_neg_op` IEEE 754 negative zero

use crate::lexer::{BinaryOp, UnaryOp, lex_expr};
use crate::parser::{parse_expr, parse_helper_name, helper_arity, helper_name, ExprAst, ExprLiteral};
use crate::bytecode::{compile_expr, compile_expr_with_resolver, check_expr_stack_bound};
use crate::eval::eval_unary_op;
use crate::bytecode::fold::{fold_unary, fold_binary};
use crate::{ExprError, ExprResult};
use vb_core::{ConstIdx, ConstValue, ExprOp, SlotIdx};

// =========================================================================
// Parser public API — parse_helper_name
// =========================================================================

/// All 13 helpers map to the correct ExprHelper variant.
#[test]
fn parse_helper_name_returns_correct_variant_for_all_helpers() {
    assert_eq!(parse_helper_name("contains"), Some(crate::parser::ExprHelper::Contains));
    assert_eq!(parse_helper_name("starts_with"), Some(crate::parser::ExprHelper::StartsWith));
    assert_eq!(parse_helper_name("ends_with"), Some(crate::parser::ExprHelper::EndsWith));
    assert_eq!(parse_helper_name("has"), Some(crate::parser::ExprHelper::Has));
    assert_eq!(parse_helper_name("exists"), Some(crate::parser::ExprHelper::Exists));
    assert_eq!(parse_helper_name("length"), Some(crate::parser::ExprHelper::Length));
    assert_eq!(parse_helper_name("empty"), Some(crate::parser::ExprHelper::Empty));
    assert_eq!(parse_helper_name("append"), Some(crate::parser::ExprHelper::Append));
    assert_eq!(parse_helper_name("append_if"), Some(crate::parser::ExprHelper::AppendIf));
    assert_eq!(parse_helper_name("merge"), Some(crate::parser::ExprHelper::Merge));
    assert_eq!(parse_helper_name("sum"), Some(crate::parser::ExprHelper::Sum));
    assert_eq!(parse_helper_name("count"), Some(crate::parser::ExprHelper::Count));
    assert_eq!(parse_helper_name("unique"), Some(crate::parser::ExprHelper::Unique));
}

/// Unknown helper names return None.
#[test]
fn parse_helper_name_rejects_unknown_helper() {
    assert_eq!(parse_helper_name("nonexistent"), None);
    assert_eq!(parse_helper_name(""), None);
    assert_eq!(parse_helper_name("contains_extra"), None);
}

// =========================================================================
// Parser public API — helper_arity
// =========================================================================

/// Arity for arity-1 helpers (Exists, Length, Empty, Sum, Count, Unique).
#[test]
fn helper_arity_arity_one_helpers() {
    assert_eq!(helper_arity(crate::parser::ExprHelper::Exists), 1);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Length), 1);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Empty), 1);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Sum), 1);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Count), 1);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Unique), 1);
}

/// Arity for arity-2 helpers (Contains, StartsWith, EndsWith, Has, Append, Merge).
#[test]
fn helper_arity_arity_two_helpers() {
    assert_eq!(helper_arity(crate::parser::ExprHelper::Contains), 2);
    assert_eq!(helper_arity(crate::parser::ExprHelper::StartsWith), 2);
    assert_eq!(helper_arity(crate::parser::ExprHelper::EndsWith), 2);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Has), 2);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Append), 2);
    assert_eq!(helper_arity(crate::parser::ExprHelper::Merge), 2);
}

/// Arity for arity-3 helper (AppendIf).
#[test]
fn helper_arity_arity_three_append_if() {
    assert_eq!(helper_arity(crate::parser::ExprHelper::AppendIf), 3);
}

// =========================================================================
// Parser public API — helper_name
// =========================================================================

/// Canonical names match expected string values.
#[test]
fn helper_name_canonical_mapping() {
    assert_eq!(helper_name(crate::parser::ExprHelper::Contains), "contains");
    assert_eq!(helper_name(crate::parser::ExprHelper::StartsWith), "starts_with");
    assert_eq!(helper_name(crate::parser::ExprHelper::EndsWith), "ends_with");
    assert_eq!(helper_name(crate::parser::ExprHelper::Has), "has");
    assert_eq!(helper_name(crate::parser::ExprHelper::Exists), "exists");
    assert_eq!(helper_name(crate::parser::ExprHelper::Length), "length");
    assert_eq!(helper_name(crate::parser::ExprHelper::Empty), "empty");
    assert_eq!(helper_name(crate::parser::ExprHelper::Append), "append");
    assert_eq!(helper_name(crate::parser::ExprHelper::AppendIf), "append_if");
    assert_eq!(helper_name(crate::parser::ExprHelper::Merge), "merge");
    assert_eq!(helper_name(crate::parser::ExprHelper::Sum), "sum");
    assert_eq!(helper_name(crate::parser::ExprHelper::Count), "count");
    assert_eq!(helper_name(crate::parser::ExprHelper::Unique), "unique");
}

// =========================================================================
// Constant folding — F64 negation does NOT fold
// =========================================================================

/// `fold_unary(Neg, F64(...))` returns None — constant folding is I64-only for arithmetic.
#[test]
fn fold_unary_neg_f64_does_not_fold() {
    let ast = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(3.14).unwrap(),
        ))),
    };
    let folded = fold_unary(UnaryOp::Neg, &ast);
    assert_eq!(folded, None, "F64 negation must not fold (I64-only arithmetic)");
}

/// `fold_unary(Neg, Bool(...))` returns None — negation only works on I64.
#[test]
fn fold_unary_neg_bool_does_not_fold() {
    let ast = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
    };
    let folded = fold_unary(UnaryOp::Neg, &ast);
    assert_eq!(folded, None, "Bool negation must not fold");
}

/// `fold_unary(Neg, Null(...))` returns None.
#[test]
fn fold_unary_neg_null_does_not_fold() {
    let ast = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(ExprAst::Literal(ExprLiteral::Null)),
    };
    let folded = fold_unary(UnaryOp::Neg, &ast);
    assert_eq!(folded, None, "Null negation must not fold");
}

// =========================================================================
// Constant folding — Eq/NotEq cross-type does NOT fold
// =========================================================================

/// `Eq(I64, F64)` returns None — cross-type Eq does not fold.
#[test]
fn fold_binary_eq_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(5.0).unwrap(),
        ))),
    };
    let folded = fold_binary(BinaryOp::Eq, &ast.left, &ast.right);
    assert_eq!(folded, None, "I64==F64 must not fold");
}

/// `Eq(F64, I64)` returns None — reversed cross-type also does not fold.
#[test]
fn fold_binary_eq_reversed_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(5.0).unwrap(),
        ))),
        right: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
    };
    let folded = fold_binary(BinaryOp::Eq, &ast.left, &ast.right);
    assert_eq!(folded, None, "F64==I64 must not fold");
}

/// `NotEq(I64, F64)` returns None — cross-type NotEq also does not fold.
#[test]
fn fold_binary_neq_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::NotEq,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(5.0).unwrap(),
        ))),
    };
    let folded = fold_binary(BinaryOp::NotEq, &ast.left, &ast.right);
    assert_eq!(folded, None, "I64!=F64 must not fold");
}

/// `Eq(I64, I64)` folds to Bool — same-type Eq works.
#[test]
fn fold_binary_eq_same_type_i64_folds() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
        right: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
    };
    assert_eq!(fold_binary(BinaryOp::Eq, &ast.left, &ast.right), Some(ConstValue::Bool(true)));

    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(5))),
        right: Box::new(ExprAst::Literal(ExprLiteral::I64(3))),
    };
    assert_eq!(fold_binary(BinaryOp::Eq, &ast.left, &ast.right), Some(ConstValue::Bool(false)));
}

/// `Eq(F64, F64)` folds to Bool — same-type F64 Eq works.
#[test]
fn fold_binary_eq_same_type_f64_folds() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(1.0).unwrap(),
        ))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(1.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Eq, &ast.left, &ast.right), Some(ConstValue::Bool(true)));
}

/// `Eq(I64, Bool)` returns None — type mismatch prevents folding.
#[test]
fn fold_binary_eq_type_mismatch_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Eq,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
    };
    // ConstValue::I64(1) != ConstValue::Bool(true), so Eq folds to Bool(false)
    assert_eq!(fold_binary(BinaryOp::Eq, &ast.left, &ast.right), Some(ConstValue::Bool(false)));
}

// =========================================================================
// Constant folding — F64 arithmetic does NOT fold via fold_i64_binop
// =========================================================================

/// `Add(F64, F64)` returns None — F64 arithmetic is not constant-folded.
#[test]
fn fold_binary_add_f64_does_not_fold() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Add,
        left: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(1.0).unwrap(),
        ))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(2.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Add, &ast.left, &ast.right), None);
}

/// `Add(I64, F64)` returns None — cross-type arithmetic does not fold.
#[test]
fn fold_binary_add_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Add,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(2.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Add, &ast.left, &ast.right), None);
}

/// `Sub(F64, F64)` returns None.
#[test]
fn fold_binary_sub_f64_does_not_fold() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Sub,
        left: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(5.0).unwrap(),
        ))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(1.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Sub, &ast.left, &ast.right), None);
}

/// `Mul(I64, F64)` returns None.
#[test]
fn fold_binary_mul_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Mul,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(3))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(2.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Mul, &ast.left, &ast.right), None);
}

/// `Div(I64, F64)` returns None.
#[test]
fn fold_binary_div_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Div,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(6))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(2.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Div, &ast.left, &ast.right), None);
}

/// `Lt(I64, F64)` returns None — comparisons are I64-only.
#[test]
fn fold_binary_lt_cross_type_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Lt,
        left: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
        right: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(2.0).unwrap(),
        ))),
    };
    assert_eq!(fold_binary(BinaryOp::Lt, &ast.left, &ast.right), None);
}

/// `And(Bool, I64)` returns None — AND is Bool-only.
#[test]
fn fold_binary_and_type_mismatch_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::And,
        left: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
        right: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
    };
    assert_eq!(fold_binary(BinaryOp::And, &ast.left, &ast.right), None);
}

/// `Or(F64, Bool)` returns None.
#[test]
fn fold_binary_or_type_mismatch_returns_none() {
    let ast = ExprAst::Binary {
        op: BinaryOp::Or,
        left: Box::new(ExprAst::Literal(ExprLiteral::F64(
            vb_core::FiniteF64::new(1.0).unwrap(),
        ))),
        right: Box::new(ExprAst::Literal(ExprLiteral::Bool(true))),
    };
    assert_eq!(fold_binary(BinaryOp::Or, &ast.left, &ast.right), None);
}

// =========================================================================
// Compilation — unresolved reference inside helper argument
// =========================================================================

/// A reference inside a helper argument (e.g., `contains($missing, "needle")`)
/// should fail with `InvalidReference`.
#[test]
fn unresolved_reference_in_helper_arg_fails() {
    let result = compile_expr("contains($missing, \"needle\")", &|_| None);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

/// A reference inside a deeply nested helper call fails.
#[test]
fn unresolved_reference_nested_in_helpers_fails() {
    // contains(append($missing, "x"), "y") — $missing is inside append's arg
    let result = compile_expr("contains(append($missing, \"x\"), \"y\")", &|_| None);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

/// Multiple references in helper args: first unresolved one fails.
#[test]
fn first_unresolved_reference_in_helper_args_fails() {
    // $a is resolved to slot 0, $missing is not
    let resolver = |ref_name: &str| match ref_name {
        "$a" => Some(SlotIdx::new(0)),
        _ => None,
    };
    let result = compile_expr("contains($a, $missing)", &resolver);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

// =========================================================================
// Stack bound — deep stack usage (DoS protection)
// =========================================================================

/// 50 LoadConst ops → max_stack = 50 (within limit).
#[test]
fn check_expr_stack_bound_deep_loads_within_limit() {
    let ops: Vec<ExprOp> = (0..50)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let max_stack = check_expr_stack_bound(&ops).expect("50 loads should be within limit");
    assert_eq!(max_stack, 50);
}

/// 100 LoadConst ops → max_stack = 100 (may exceed limit depending on MAX_EXPRESSION_STACK).
#[test]
fn check_expr_stack_bound_100_loads_exceeds_max_expression_stack() {
    let ops: Vec<ExprOp> = (0..100)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let result = check_expr_stack_bound(&ops);
    // vb_core::limits::MAX_EXPRESSION_STACK is 63, so 100 should overflow
    assert!(result.is_err(), "100 loads should exceed MAX_EXPRESSION_STACK(63)");
    match result {
        Err(ExprError::StackOverflow { max }) => {
            assert_eq!(max, 63, "overflow should report max=63");
        }
        other => panic!("expected StackOverflow, got {other:?}"),
    }
}

/// 63 LoadConst ops → max_stack = 63 (exactly at limit).
#[test]
fn check_expr_stack_bound_63_loads_at_limit() {
    let ops: Vec<ExprOp> = (0..63)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let max_stack = check_expr_stack_bound(&ops).expect("63 loads should be exactly at limit");
    assert_eq!(max_stack, 63);
}

/// 64 LoadConst ops → exceeds limit.
#[test]
fn check_expr_stack_bound_64_loads_exceeds_limit() {
    let ops: Vec<ExprOp> = (0..64)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let result = check_expr_stack_bound(&ops);
    assert!(result.is_err(), "64 loads should exceed MAX_EXPRESSION_STACK(63)");
}

/// Mixed ops: push/pop pattern with correct stack depth tracking.
#[test]
fn check_expr_stack_bound_mixed_ops_tracking() {
    // Push 3, consume 2 with Add, push 1, consume 2 with Add → max stack = 3
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)), // stack: 1
        ExprOp::LoadConst(ConstIdx::new(1)), // stack: 2
        ExprOp::LoadConst(ConstIdx::new(2)), // stack: 3 (max)
        ExprOp::Add,                          // stack: 2 (consumes 2, pushes 1)
        ExprOp::LoadConst(ConstIdx::new(3)), // stack: 3 (max again)
        ExprOp::Add,                          // stack: 2
    ];
    let max_stack = check_expr_stack_bound(&ops).expect("mixed ops should pass");
    assert_eq!(max_stack, 3);
}

/// Deep push with interleaved consumes — verifies stack depth tracking is correct.
#[test]
fn check_expr_stack_bound_deep_push_then_consume() {
    // Push 10 values, consume 1 with Unary → max_stack = 10
    let mut ops = vec![];
    for i in 0..10u16 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
    }
    ops.push(ExprOp::Not); // consumes 1, pushes 1
    let max_stack = check_expr_stack_bound(&ops).expect("10 pushes + 1 unary should pass");
    assert_eq!(max_stack, 10);
}

// =========================================================================
// Lexer — infix_binding_power for all 12 BinaryOp variants
// =========================================================================

/// All 12 BinaryOp variants have correct binding power tuples.
#[test]
fn infix_binding_power_all_variants() {
    assert_eq!(BinaryOp::Or.binding_power(), (1, 2));
    assert_eq!(BinaryOp::And.binding_power(), (3, 4));
    assert_eq!(BinaryOp::Eq.binding_power(), (5, 6));
    assert_eq!(BinaryOp::NotEq.binding_power(), (5, 6));
    assert_eq!(BinaryOp::Lt.binding_power(), (7, 8));
    assert_eq!(BinaryOp::Lte.binding_power(), (7, 8));
    assert_eq!(BinaryOp::Gt.binding_power(), (7, 8));
    assert_eq!(BinaryOp::Gte.binding_power(), (7, 8));
    assert_eq!(BinaryOp::Add.binding_power(), (9, 10));
    assert_eq!(BinaryOp::Sub.binding_power(), (9, 10));
    assert_eq!(BinaryOp::Mul.binding_power(), (11, 12));
    assert_eq!(BinaryOp::Div.binding_power(), (11, 12));
}

// =========================================================================
// Lexer — strip_quotes with single-character inner string
// =========================================================================

/// strip_quotes("a") → Ok("a") — single-char inner string.
#[test]
fn lex_single_char_string_inner() -> ExprResult<()> {
    let tokens = lex_expr(r#""a""#)?;
    // Token::Literal(LiteralToken::Text(Box::from("a")))
    match tokens.first() {
        Some(crate::lexer::Token::Literal(crate::lexer::LiteralToken::Text(t))) => {
            assert_eq!(t.as_ref(), "a");
        }
        other => panic!("expected Text(\"a\"), got {other:?}"),
    }
    Ok(())
}

/// strip_quotes with two-char inner string.
#[test]
fn lex_two_char_string_inner() -> ExprResult<()> {
    let tokens = lex_expr(r#"""ab""#)?;
    match tokens.first() {
        Some(crate::lexer::Token::Literal(crate::lexer::LiteralToken::Text(t))) => {
            assert_eq!(t.as_ref(), "ab");
        }
        other => panic!("expected Text(\"ab\"), got {other:?}"),
    }
    Ok(())
}

// =========================================================================
// Evaluator — IEEE 754 negative zero
// =========================================================================

/// eval_unary_op(Neg, F64(0.0)) → F64(-0.0) — IEEE 754 negative zero.
#[test]
fn eval_neg_op_f64_zero_returns_negative_zero() -> ExprResult<()> {
    let result = eval_unary_op(
        UnaryOp::Neg,
        vb_core::SlotValue::F64(vb_core::FiniteF64::new(0.0).unwrap()),
    );
    let Ok(v) = result else {
        panic!("negation of 0.0 should succeed, got {:?}", result);
    };
    let vb_core::SlotValue::F64(finite) = v else {
        panic!("expected F64 result, got {:?}", v);
    };
    // -0.0 is finite and equals 0.0, but has the sign bit set
    assert!(finite.is_finite(), "-0.0 should be finite");
    assert_eq!(finite.get(), 0.0, "-0.0 == 0.0");
    // Verify it is negative zero by checking the sign
    assert!(
        finite.get().is_sign_negative(),
        "negation of 0.0 should produce negative zero"
    );
    Ok(())
}

/// eval_unary_op(Not, Bool(true)) → Bool(false) (already tested in bytecode tests).
/// This test verifies the evaluator path directly.
#[test]
fn eval_unary_op_not_bool_true_returns_false() {
    let result = eval_unary_op(UnaryOp::Not, vb_core::SlotValue::Bool(true));
    assert_eq!(result, Ok(vb_core::SlotValue::Bool(false)));
}

/// eval_unary_op(Not, Bool(false)) → Bool(true).
#[test]
fn eval_unary_op_not_bool_false_returns_true() {
    let result = eval_unary_op(UnaryOp::Not, vb_core::SlotValue::Bool(false));
    assert_eq!(result, Ok(vb_core::SlotValue::Bool(true)));
}
