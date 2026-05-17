// Property tests for eval_bounds (BE) — vb_expr
// Tests: Stack overflow, program index bounds, and expression stack enforcement.
// Coverage: BE-1..BE-8 from test-plan §1.5.

use crate::eval::{eval_binary_op, eval_unary_op};
use crate::lexer::{BinaryOp, UnaryOp};
use vb_core::value::FiniteF64;
use vb_core::{ExprOp, SlotValue};

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// BE-1..BE-4: Stack overflow — check_expr_stack_bound enforces MAX_EXPRESSION_STACK.
// ---------------------------------------------------------------------------

#[test]
fn be_check_expr_stack_bound_rejects_100_loads_no_pop() {
    // 100 LoadConst ops with no pops: max stack depth = 100.
    // MAX_EXPRESSION_STACK = 64, so this must be rejected (also invalid final depth).
    let ops: Vec<ExprOp> = (0..100)
        .map(|i| ExprOp::LoadConst(vb_core::ConstIdx::new(i as u16 % 256)))
        .collect();
    let result = crate::bytecode::check_expr_stack_bound(&ops);
    // This should fail because either the stack overflows OR final depth != 1
    assert!(
        result.is_err(),
        "100 LoadConst ops with no pops must be rejected"
    );
}

#[test]
fn be_check_expr_stack_bound_accepts_proper_program_depth_2() {
    // Build program: LoadConst(0), LoadConst(1), Add
    // Stack: LoadConst(0) -> depth 1, LoadConst(1) -> depth 2, Add -> depth 1
    // Max stack depth = 2, final depth = 1. This is valid.
    let ops = vec![
        ExprOp::LoadConst(vb_core::ConstIdx::new(0)),
        ExprOp::LoadConst(vb_core::ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = crate::bytecode::check_expr_stack_bound(&ops);
    assert!(
        result.is_ok(),
        "proper program with max depth 2 should be accepted"
    );
}

#[test]
fn be_check_expr_stack_bound_rejects_too_many_ops() {
    // Verify that an ops sequence with more than MAX_EXPRESSION_OPS is rejected.
    // MAX_EXPRESSION_OPS = 256 (from bytecode/mod.rs MAX_OPS).
    // We'll create 300 ops that are all LoadConst - this exceeds both
    // MAX_EXPRESSION_OPS and MAX_EXPRESSION_STACK.
    let ops: Vec<ExprOp> = (0..300)
        .map(|i| ExprOp::LoadConst(vb_core::ConstIdx::new(i as u16 % 256)))
        .collect();
    let result = crate::bytecode::check_expr_stack_bound(&ops);
    assert!(
        result.is_err(),
        "300 ops must be rejected (exceeds MAX_EXPRESSION_OPS)"
    );
}

// ---------------------------------------------------------------------------
// BE-5..BE-6: Unary ops do not increase stack depth
// ---------------------------------------------------------------------------

#[test]
fn be_not_operation_depth_is_one() {
    // LoadConst, Not — stack: [val] → [result], depth = 1 throughout.
    let ops = vec![ExprOp::LoadConst(vb_core::ConstIdx::new(0)), ExprOp::Not];
    let result = crate::bytecode::check_expr_stack_bound(&ops);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// BE-7..BE-8: Division does not change stack depth behavior
// ---------------------------------------------------------------------------

#[test]
fn be_division_depth_is_two() {
    // LoadConst, LoadConst, Div — stack depth = 2 throughout.
    let ops = vec![
        ExprOp::LoadConst(vb_core::ConstIdx::new(10)),
        ExprOp::LoadConst(vb_core::ConstIdx::new(2)),
        ExprOp::Div,
    ];
    let result = crate::bytecode::check_expr_stack_bound(&ops);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// BE-9: eval_binary_op with mixed I64/F64 types returns TypeMismatch
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn be_eval_add_i64_plus_f64_returns_type_mismatch(i in any::<i64>()) {
        let f64_val = FiniteF64::new(1.5).expect("valid finite f64");
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::I64(i),
            SlotValue::F64(f64_val),
        );
        let is_type_mismatch = matches!(result, Err(crate::ExprError::TypeMismatch { .. }));
        prop_assert!(is_type_mismatch);
    }

    #[test]
    fn be_eval_sub_f64_minus_i64_returns_type_mismatch(i in any::<i64>()) {
        let f64_val = FiniteF64::new(1.5).expect("valid finite f64");
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(f64_val),
            SlotValue::I64(i),
        );
        let is_type_mismatch = matches!(result, Err(crate::ExprError::TypeMismatch { .. }));
        prop_assert!(is_type_mismatch);
    }
}

// ---------------------------------------------------------------------------
// BE-10: eval_unary_op neg on bool returns TypeMismatch
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn be_eval_neg_bool_returns_type_mismatch(val in proptest::bool::ANY) {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::Bool(val));
        let is_type_mismatch = matches!(result, Err(crate::ExprError::TypeMismatch { .. }));
        prop_assert!(is_type_mismatch);
    }
}

// ---------------------------------------------------------------------------
// BE-11: eval_binary_op and/or on non-bool returns TypeMismatch
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn be_eval_and_with_non_bool_left_returns_type_mismatch(val in any::<i64>()) {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::I64(val),
            SlotValue::Bool(true),
        );
        let is_type_mismatch = matches!(result, Err(crate::ExprError::TypeMismatch { .. }));
        prop_assert!(is_type_mismatch);
    }

    #[test]
    fn be_eval_or_with_non_bool_right_returns_type_mismatch(val in any::<i64>()) {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::Bool(true),
            SlotValue::I64(val),
        );
        let is_type_mismatch = matches!(result, Err(crate::ExprError::TypeMismatch { .. }));
        prop_assert!(is_type_mismatch);
    }
}

// ---------------------------------------------------------------------------
// BE-12: Comparison ops work across all i64 values (exhaustive sample)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn be_eval_gt_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::Gt, SlotValue::I64(a), SlotValue::I64(b))
            .expect("gt should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a > b),
            _other => prop_assert!(false),
        }
    }

    #[test]
    fn be_eval_gte_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::Gte, SlotValue::I64(a), SlotValue::I64(b))
            .expect("gte should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a >= b),
            _other => prop_assert!(false),
        }
    }

    #[test]
    fn be_eval_lt_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(a), SlotValue::I64(b))
            .expect("lt should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a < b),
            _other => prop_assert!(false),
        }
    }

    #[test]
    fn be_eval_lte_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::Lte, SlotValue::I64(a), SlotValue::I64(b))
            .expect("lte should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a <= b),
            _other => prop_assert!(false),
        }
    }

    #[test]
    fn be_eval_eq_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(a), SlotValue::I64(b))
            .expect("eq should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a == b),
            _other => prop_assert!(false),
        }
    }

    #[test]
    fn be_eval_noteq_i64_is_correct(a: i64, b: i64) {
        let result = eval_binary_op(BinaryOp::NotEq, SlotValue::I64(a), SlotValue::I64(b))
            .expect("noteq should not overflow");
        match result {
            SlotValue::Bool(b_vec) => prop_assert_eq!(b_vec, a != b),
            _other => prop_assert!(false),
        }
    }
}
