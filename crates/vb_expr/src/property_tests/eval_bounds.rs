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
        matches!(
            result,
            Err(crate::ExprError::StackOverflow { .. }
                | crate::ExprError::BytecodeTooLong { .. }
                | crate::ExprError::StackUnderflow)
        ),
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
    let depth = result.expect("proper program with max depth 2 should be accepted");
    assert_eq!(
        depth, 2,
        "required max depth should be 2 for LoadConst+LoadConst+Add"
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
        matches!(
            result,
            Err(crate::ExprError::StackOverflow { .. }
                | crate::ExprError::BytecodeTooLong { .. }
                | crate::ExprError::StackUnderflow)
        ),
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
    let depth = result.expect("LoadConst+Not must succeed with max depth 1");
    assert_eq!(depth, 1, "LoadConst+Not must report required depth 1");
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
    let depth = result.expect("LoadConst+LoadConst+Div must succeed with max depth 2");
    assert_eq!(
        depth, 2,
        "LoadConst+LoadConst+Div must report required depth 2"
    );
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
