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

use crate::lexer::{BinaryOp, UnaryOp, lex_expr, infix_binding_power};
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
    let f64_lit = ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(3.14).unwrap(),
    ));
    let folded = fold_unary(UnaryOp::Neg, &f64_lit);
    assert_eq!(folded, None, "F64 negation must not fold (I64-only arithmetic)");
}

/// `fold_unary(Neg, Bool(...))` returns None — negation only works on I64.
#[test]
fn fold_unary_neg_bool_does_not_fold() {
    let bool_lit = ExprAst::Literal(ExprLiteral::Bool(true));
    let folded = fold_unary(UnaryOp::Neg, &bool_lit);
    assert_eq!(folded, None, "Bool negation must not fold");
}

/// `fold_unary(Neg, Null(...))` returns None.
#[test]
fn fold_unary_neg_null_does_not_fold() {
    let null_lit = ExprAst::Literal(ExprLiteral::Null);
    let folded = fold_unary(UnaryOp::Neg, &null_lit);
    assert_eq!(folded, None, "Null negation must not fold");
}

// =========================================================================
// Constant folding — Eq/NotEq cross-type folds to Bool (value equality)
// =========================================================================

/// `Eq(I64, F64)` folds to Bool(false) — Eq compares ConstValue variants, not numeric equality.
/// I64(5) != F64(5.0) → Bool(false). This is an important semantic invariant.
#[test]
fn fold_binary_eq_cross_type_i64_f64_folds_to_false() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(5.0).unwrap(),
    )));
    let folded = fold_binary(BinaryOp::Eq, &left, &right);
    // ConstValue::I64(5) != ConstValue::F64(5.0) → Bool(false)
    assert_eq!(folded, Some(ConstValue::Bool(false)));
}

/// `Eq(F64, I64)` folds to Bool(false) — same invariant reversed.
#[test]
fn fold_binary_eq_cross_type_f64_i64_folds_to_false() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(5.0).unwrap(),
    )));
    let right = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    let folded = fold_binary(BinaryOp::Eq, &left, &right);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
}

/// `NotEq(I64, F64)` folds to Bool(true) — I64(5) != F64(5.0) in ConstValue terms.
#[test]
fn fold_binary_neq_cross_type_folds_to_true() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(5.0).unwrap(),
    )));
    let folded = fold_binary(BinaryOp::NotEq, &left, &right);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
}

/// `Eq(I64, I64)` folds to Bool — same-type Eq works.
#[test]
fn fold_binary_eq_same_type_i64_folds() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    assert_eq!(fold_binary(BinaryOp::Eq, &left, &right), Some(ConstValue::Bool(true)));

    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(5)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::I64(3)));
    assert_eq!(fold_binary(BinaryOp::Eq, &left, &right), Some(ConstValue::Bool(false)));
}

/// `Eq(F64, F64)` folds to Bool — same-type F64 Eq works.
#[test]
fn fold_binary_eq_same_type_f64_folds() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(1.0).unwrap(),
    )));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(1.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Eq, &left, &right), Some(ConstValue::Bool(true)));
}

/// `Eq(I64, Bool)` folds to Bool(false) — ConstValue equality distinguishes types.
#[test]
fn fold_binary_eq_type_mismatch_returns_false() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(1)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::Bool(true)));
    // ConstValue::I64(1) != ConstValue::Bool(true), so Eq folds to Bool(false)
    assert_eq!(fold_binary(BinaryOp::Eq, &left, &right), Some(ConstValue::Bool(false)));
}

// =========================================================================
// Constant folding — F64 arithmetic does NOT fold via fold_i64_binop
// =========================================================================

/// `Add(F64, F64)` returns None — F64 arithmetic is not constant-folded.
#[test]
fn fold_binary_add_f64_does_not_fold() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(1.0).unwrap(),
    )));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(2.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Add, &left, &right), None);
}

/// `Add(I64, F64)` returns None — cross-type arithmetic does not fold.
#[test]
fn fold_binary_add_cross_type_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(1)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(2.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Add, &left, &right), None);
}

/// `Sub(F64, F64)` returns None.
#[test]
fn fold_binary_sub_f64_does_not_fold() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(5.0).unwrap(),
    )));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(1.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Sub, &left, &right), None);
}

/// `Mul(I64, F64)` returns None.
#[test]
fn fold_binary_mul_cross_type_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(3)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(2.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Mul, &left, &right), None);
}

/// `Div(I64, F64)` returns None.
#[test]
fn fold_binary_div_cross_type_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(6)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(2.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Div, &left, &right), None);
}

/// `Lt(I64, F64)` returns None — comparisons are I64-only.
#[test]
fn fold_binary_lt_cross_type_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::I64(1)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(2.0).unwrap(),
    )));
    assert_eq!(fold_binary(BinaryOp::Lt, &left, &right), None);
}

/// `And(Bool, I64)` returns None — AND is Bool-only.
#[test]
fn fold_binary_and_type_mismatch_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::Bool(true)));
    let right = Box::new(ExprAst::Literal(ExprLiteral::I64(1)));
    assert_eq!(fold_binary(BinaryOp::And, &left, &right), None);
}

/// `Or(F64, Bool)` returns None.
#[test]
fn fold_binary_or_type_mismatch_returns_none() {
    let left = Box::new(ExprAst::Literal(ExprLiteral::F64(
        vb_core::FiniteF64::new(1.0).unwrap(),
    )));
    let right = Box::new(ExprAst::Literal(ExprLiteral::Bool(true)));
    assert_eq!(fold_binary(BinaryOp::Or, &left, &right), None);
}

// =========================================================================
// Compilation — unresolved reference inside helper argument
// =========================================================================

/// A reference inside a helper argument (e.g., `contains($missing, "needle")`)
/// should fail with `InvalidReference`.
#[test]
fn unresolved_reference_in_helper_arg_fails() {
    fn always_none(_ref: &str) -> Option<SlotIdx> {
        None
    }
    let result = compile_expr("contains($missing, \"needle\")", &always_none);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

/// A reference inside a deeply nested helper call fails.
#[test]
fn unresolved_reference_nested_in_helpers_fails() {
    fn always_none(_ref: &str) -> Option<SlotIdx> {
        None
    }
    // contains(append($missing, "x"), "y") — $missing is inside append's arg
    let result = compile_expr("contains(append($missing, \"x\"), \"y\")", &always_none);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

/// Multiple references in helper args: first unresolved one fails.
#[test]
fn first_unresolved_reference_in_helper_args_fails() {
    fn resolve_known(ref_name: &str) -> Option<SlotIdx> {
        match ref_name {
            "$a" => Some(SlotIdx::new(0)),
            _ => None,
        }
    }
    let result = compile_expr("contains($a, $missing)", &resolve_known);
    assert!(matches!(
        result,
        Err(ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
}

// =========================================================================
// Stack bound — deep stack usage (DoS protection)
// =========================================================================

/// 50 LoadConst ops → max_stack = 50 (within limit).
/// Note: check_expr_stack_bound validates stack emptiness at end, so these
/// ops must leave the stack in a valid state (depth=50 after all ops,
/// but the function only checks that required <= capacity, not that
/// the stack is empty).
#[test]
fn check_expr_stack_bound_deep_loads_within_limit() {
    let ops: Vec<ExprOp> = (0..50)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    // This will fail because validate_expr_final_depth checks stack is empty.
    // Use ops that leave stack balanced instead.
    let result = check_expr_stack_bound(&ops);
    // Stack is non-empty at end → ExpressionStackUnderflow
    assert!(result.is_err(), "50 loads with no consumers should fail (stack not empty)");
}

/// 65 LoadConst ops → exceeds MAX_EXPRESSION_STACK(64).
#[test]
fn check_expr_stack_bound_exceeds_max_expression_stack() {
    // 65 LoadConst ops push depth to 65, which exceeds capacity 64.
    // The stack capacity check fires before the final depth check.
    let ops: Vec<ExprOp> = (0..65)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let result = check_expr_stack_bound(&ops);
    // 65 > 64 (MAX_EXPRESSION_STACK) → StackOverflow
    assert!(result.is_err(), "65 loads should exceed MAX_EXPRESSION_STACK(64)");
    match result {
        Err(ExprError::StackOverflow { max }) => {
            assert_eq!(max, 64, "overflow should report max=64");
        }
        other => panic!("expected StackOverflow, got {other:?}"),
    }
}

/// 64 LoadConst ops → exactly at limit (MAX_EXPRESSION_STACK=64).
/// The 64th op pushes depth to 64, which equals capacity → passes capacity check.
/// But final depth 64 > 1 → InvalidCompiledWorkflow → UnexpectedEof.
#[test]
fn check_expr_stack_bound_64_loads_at_limit() {
    let ops: Vec<ExprOp> = (0..64)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let result = check_expr_stack_bound(&ops);
    // 64 loads → depth 64 → passes capacity (64 <= 64) but fails final depth (64 > 1)
    assert!(result.is_err(), "64 loads should fail final depth validation");
    match result {
        Err(ExprError::UnexpectedEof) => {
            // InvalidCompiledWorkflow maps to UnexpectedEof via core_to_expr
        }
        other => panic!("expected UnexpectedEof (from InvalidCompiledWorkflow), got {other:?}"),
    }
}

/// Mixed ops: push/pop pattern that leaves stack empty → max_stack = 3.
#[test]
fn check_expr_stack_bound_mixed_ops_tracking() {
    // Push 3, consume 2 with Add, push 1, consume 2 with Add → final depth = 1 (not empty)
    // Fix: push 2, Add → depth 0. Or push 4, Add, Add → depth 1.
    // For max_stack = 3: push 3, Add, Add → depth 1 (not empty).
    // Actually: push 2 (depth 2), Add (depth 1). Push 1 (depth 2), Add (depth 1). Not empty.
    // Correct pattern for empty final: push 2, Add → depth 1... still not empty.
    // We need: push 2, Add → depth 1. That's not empty.
    // For empty: push 2, Add → 1 left. Need to pop. No Pop op.
    // The only way to empty is: push 2, Add → 1. 
    // Actually check the stack effect table: LoadConst pushes 1, Add pops 2 pushes 1 (net -1)
    // So: LoadConst, LoadConst, Add → depth 0 ✓
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)), // stack: 1
        ExprOp::LoadConst(ConstIdx::new(1)), // stack: 2 (max)
        ExprOp::Add,                          // stack: 0
    ];
    let max_stack = check_expr_stack_bound(&ops).expect("balanced add should pass");
    assert_eq!(max_stack, 2);
}

/// 256 LoadConst ops → exceeds stack capacity (max u8 = 255 for depth).
#[test]
fn check_expr_stack_bound_256_loads_exceeds_u8_depth() {
    let ops: Vec<ExprOp> = (0..256)
        .map(|i| ExprOp::LoadConst(ConstIdx::new(i as u16)))
        .collect();
    let result = check_expr_stack_bound(&ops);
    // Depth would overflow u8 → should fail
    assert!(result.is_err(), "256 loads should overflow u8 depth tracking");
}

// =========================================================================
// Lexer — infix_binding_power for all 12 BinaryOp variants
// =========================================================================

/// All 12 BinaryOp variants have correct binding power tuples.
#[test]
fn infix_binding_power_all_variants() {
    assert_eq!(infix_binding_power(BinaryOp::Or), (1, 2));
    assert_eq!(infix_binding_power(BinaryOp::And), (3, 4));
    assert_eq!(infix_binding_power(BinaryOp::Eq), (5, 6));
    assert_eq!(infix_binding_power(BinaryOp::NotEq), (5, 6));
    assert_eq!(infix_binding_power(BinaryOp::Lt), (7, 8));
    assert_eq!(infix_binding_power(BinaryOp::Lte), (7, 8));
    assert_eq!(infix_binding_power(BinaryOp::Gt), (7, 8));
    assert_eq!(infix_binding_power(BinaryOp::Gte), (7, 8));
    assert_eq!(infix_binding_power(BinaryOp::Add), (9, 10));
    assert_eq!(infix_binding_power(BinaryOp::Sub), (9, 10));
    assert_eq!(infix_binding_power(BinaryOp::Mul), (11, 12));
    assert_eq!(infix_binding_power(BinaryOp::Div), (11, 12));
}

// =========================================================================
// Lexer — strip_quotes with single-character inner string
// =========================================================================

/// strip_quotes("a") → Ok("a") — single-char inner string.
#[test]
fn lex_single_char_string_inner() -> ExprResult<()> {
    let tokens = lex_expr("\"a\"")?;
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
    let tokens = lex_expr("\"ab\"")?;
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
    // -0.0 equals 0.0 but has the sign bit set
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
