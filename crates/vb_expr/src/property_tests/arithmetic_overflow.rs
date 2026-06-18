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

// Property tests for arithmetic_overflow (AO) — vb_expr
// Tests: All eval_*_op functions use checked arithmetic and return Err on overflow.
// Coverage: AO-1..AO-13 from test-plan §1.8.

use crate::eval::{eval_binary_op, eval_unary_op};
use crate::lexer::{BinaryOp, UnaryOp};
use vb_core::SlotValue;
use vb_core::value::FiniteF64;

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// AO-1: i64::MAX + 1 → IntegerOverflow
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_add_i64_max_plus_one_returns_overflow(_unit in Just(())) {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }

    #[test]
    fn ao_add_i64_min_minus_one_returns_overflow(_unit in Just(())) {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}

// ---------------------------------------------------------------------------
// AO-2: i64::MIN - 1 → IntegerOverflow
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_sub_i64_min_minus_one_returns_overflow(_unit in Just(())) {
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}

// ---------------------------------------------------------------------------
// AO-3: i64::MAX * 2 → IntegerOverflow
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_mul_i64_max_times_two_returns_overflow(_unit in Just(())) {
        // Use a large value that will overflow when multiplied by 2
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX / 2 + 1), SlotValue::I64(2));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}

// ---------------------------------------------------------------------------
// AO-4: -i64::MIN → IntegerOverflow (negation of MIN is overflow)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_neg_i64_min_returns_overflow(_unit in Just(())) {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}

// ---------------------------------------------------------------------------
// AO-5: x / 0 → DivisionByZero
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_div_by_zero_returns_division_by_zero(val in any::<i64>()) {
        prop_assume!(val != 0);
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(val), SlotValue::I64(0));
        prop_assert!(matches!(result, Err(crate::ExprError::DivisionByZero)));
    }
}

// ---------------------------------------------------------------------------
// AO-6: i64::MIN / -1 → IntegerOverflow
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_div_i64_min_by_neg_one_returns_overflow(_unit in Just(())) {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}

// ---------------------------------------------------------------------------
// AO-6b: active eval division partitions zero, overflow, and representable pairs
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_eval_division_partition_matches_error_taxonomy(left in any::<i64>(), right in any::<i64>()) {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(left), SlotValue::I64(right));

        if right == 0 {
            prop_assert!(matches!(result, Err(crate::ExprError::DivisionByZero)));
        } else if left == i64::MIN && right == -1 {
            prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
        } else {
            match left.checked_div(right) {
                Some(expected) => prop_assert_eq!(result, Ok(SlotValue::I64(expected))),
                None => prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow))),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AO-7: f64 zero negation
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_neg_f64_zero_neg_zero(_unit in Just(())) {
        let neg_zero = FiniteF64::new(-0.0).expect("-0.0 is finite");
        let pos_zero = FiniteF64::new(0.0).expect("0.0 is finite");
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(neg_zero));
        prop_assert_eq!(result, Ok(SlotValue::F64(pos_zero)));
    }
}

// ---------------------------------------------------------------------------
// AO-8: Overflow in binary op wrapper (eval_binary_op)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ao_binary_op_add_i64_overflow_returns_error(_unit in Just(())) {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::I64(i64::MAX),
            SlotValue::I64(1)
        );
        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }

    #[test]
    fn ao_binary_op_div_by_zero_returns_error(val in any::<i64>()) {
        prop_assume!(val != 0);
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::I64(val),
            SlotValue::I64(0)
        );
        prop_assert!(matches!(result, Err(crate::ExprError::DivisionByZero)));
    }
}
