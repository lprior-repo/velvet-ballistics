// Regression property tests for builtin_eval overflow semantics.

use crate::lexer::BinaryOp;
use proptest::prelude::*;
use vb_core::SlotValue;

proptest! {
    #[test]
    fn proptest_builtin_eval_min_div_neg_one_returns_integer_overflow(_unit in Just(())) {
        let result = crate::builtin_eval::eval_binary_op(
            BinaryOp::Div,
            SlotValue::I64(i64::MIN),
            SlotValue::I64(-1),
        );

        prop_assert!(matches!(result, Err(crate::ExprError::IntegerOverflow)));
    }
}
