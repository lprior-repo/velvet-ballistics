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
