#![forbid(unsafe_code)]
//! Blackhat tests for active expression operator evaluation.
//! These tests document security-relevant behavior in the public evaluator.

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod blackhat_tests {
    use crate::ExprError;
    use crate::eval::{eval_binary_op, eval_unary_op};
    use crate::lexer::{BinaryOp, UnaryOp};
    use vb_core::SlotValue;
    use vb_core::value::FiniteF64;

    /// BH-BE-001: active eval maps i64::MIN / -1 to IntegerOverflow.
    ///
    /// Regression guard for signed division overflow. `checked_div` returns
    /// `None` for both zero divisors and `i64::MIN / -1`, so the divisor must be
    /// classified before mapping the remaining `None` to `IntegerOverflow`.
    #[test]
    fn blackhat_be_001_div_values_reports_min_div_neg_one_as_overflow() {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        assert!(matches!(result, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-002: Public eval_binary_op keeps f64 division support.
    ///
    /// Guard against reintroducing a public narrow duplicate i64-only evaluator.
    #[test]
    fn blackhat_be_002_public_api_preserves_f64_division() -> crate::ExprResult<()> {
        let left = FiniteF64::new(4.0)?;
        let right = FiniteF64::new(2.0)?;
        let expected = FiniteF64::new(2.0)?;
        let result = eval_binary_op(BinaryOp::Div, SlotValue::F64(left), SlotValue::F64(right));
        assert_eq!(result, Ok(SlotValue::F64(expected)));
        Ok(())
    }

    /// BH-BE-003: eval_binary_op addition overflow detection.
    #[test]
    fn blackhat_be_003_add_overflow() {
        let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-004: eval_binary_op subtraction overflow detection.
    #[test]
    fn blackhat_be_004_sub_overflow() {
        let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-005: eval_binary_op multiplication overflow detection.
    #[test]
    fn blackhat_be_005_mul_overflow() {
        let r = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-006: eval_unary_op negation overflow detection.
    #[test]
    fn blackhat_be_006_neg_overflow() {
        let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-007: Division by zero returns correct error.
    #[test]
    fn blackhat_be_007_div_by_zero() {
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(1), SlotValue::I64(0));
        assert!(matches!(r, Err(ExprError::DivisionByZero)));
    }

    /// BH-BE-008: Type confusion rejected for all cross-type operations.
    #[test]
    fn blackhat_be_008_type_confusion() {
        // bool + int
        assert!(matches!(
            eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1)),
            Err(ExprError::TypeMismatch { .. })
        ));
        // int and int
        assert!(matches!(
            eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(0)),
            Err(ExprError::TypeMismatch { .. })
        ));
        // not int
        assert!(matches!(
            eval_unary_op(UnaryOp::Not, SlotValue::I64(1)),
            Err(ExprError::TypeMismatch { .. })
        ));
    }

    /// BH-XO50X-003: division type gate precedes zero and overflow classification.
    #[test]
    fn blackhat_be_019_division_type_gate_precedes_division_taxonomy() {
        assert!(matches!(
            eval_binary_op(BinaryOp::Div, SlotValue::Bool(true), SlotValue::I64(0)),
            Err(ExprError::TypeMismatch { .. })
        ));
        assert!(matches!(
            eval_binary_op(BinaryOp::Div, SlotValue::I64(1), SlotValue::Bool(false)),
            Err(ExprError::TypeMismatch { .. })
        ));
        assert!(matches!(
            eval_binary_op(BinaryOp::Div, SlotValue::Bool(true), SlotValue::I64(-1)),
            Err(ExprError::TypeMismatch { .. })
        ));
        assert!(matches!(
            eval_binary_op(
                BinaryOp::Div,
                SlotValue::I64(i64::MIN),
                SlotValue::Bool(false)
            ),
            Err(ExprError::TypeMismatch { .. })
        ));
    }

    /// BH-BE-009: End-to-end bytecode program with i64::MIN / -1.
    ///
    /// The eval.rs main evaluator correctly returns IntegerOverflow for
    /// this program because eval_div_values checks for zero explicitly.
    #[test]
    fn blackhat_be_009_program_i64_min_div_neg_one() -> crate::ExprResult<()> {
        use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram};

        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)];
        let result = crate::eval::eval_expr_program(&program, &[], &constants);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "BH-BE-009: expected IntegerOverflow".into(),
            });
        };
        Ok(())
    }

    /// BH-BE-010: Stack underflow from empty stack returns error, not panic.
    #[test]
    fn blackhat_be_010_stack_underflow_no_panic() -> crate::ExprResult<()> {
        use vb_core::{ExprOp, ExprProgram};

        let program = ExprProgram {
            ops: vec![ExprOp::Add].into_boxed_slice(),
            max_stack: 0,
        };
        let r = crate::eval::eval_expr_program(&program, &[], &[]);
        let Err(ExprError::StackUnderflow) = r else {
            return Err(ExprError::UnexpectedToken {
                token: "BH-BE-010: expected StackUnderflow".into(),
            });
        };
        Ok(())
    }

    /// BH-BE-011: Out-of-bounds slot and constant access returns error, not panic.
    #[test]
    fn blackhat_be_011_oob_access_no_panic() -> crate::ExprResult<()> {
        use vb_core::{ConstIdx, ExprOp, ExprProgram, SlotIdx};

        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(255))].into_boxed_slice(),
            max_stack: 1,
        };
        let r = crate::eval::eval_expr_program(&program, &[], &[]);
        assert!(r.is_err(), "BH-BE-011a: OOB slot should error");
        let program = ExprProgram {
            ops: vec![ExprOp::LoadConst(ConstIdx::new(255))].into_boxed_slice(),
            max_stack: 1,
        };
        let r = crate::eval::eval_expr_program(&program, &[], &[]);
        assert!(r.is_err(), "BH-BE-011b: OOB const should error");
        Ok(())
    }

    /// BH-BE-012: Cross-type equality does not panic.
    #[test]
    fn blackhat_be_012_cross_type_equality_no_panic() -> crate::ExprResult<()> {
        let r = eval_binary_op(BinaryOp::Eq, SlotValue::Null, SlotValue::I64(1))?;
        assert_eq!(r, SlotValue::Bool(false));
        let r = eval_binary_op(BinaryOp::NotEq, SlotValue::Null, SlotValue::I64(1))?;
        assert_eq!(r, SlotValue::Bool(true));
        let r = eval_binary_op(BinaryOp::Eq, SlotValue::Null, SlotValue::Null)?;
        assert_eq!(r, SlotValue::Bool(true));
        Ok(())
    }

    /// BH-BE-013: Division truncation is correct (toward zero).
    #[test]
    fn blackhat_be_013_division_truncation() -> crate::ExprResult<()> {
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(7), SlotValue::I64(2))?;
        assert_eq!(r, SlotValue::I64(3));
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(-7), SlotValue::I64(2))?;
        assert_eq!(r, SlotValue::I64(-3));
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(7), SlotValue::I64(-2))?;
        assert_eq!(r, SlotValue::I64(-3));
        let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(-7), SlotValue::I64(-2))?;
        assert_eq!(r, SlotValue::I64(3));
        Ok(())
    }

    /// BH-BE-014: End-to-end overflow in nested multiplication.
    #[test]
    fn blackhat_be_014_e2e_overflow_nested() -> crate::ExprResult<()> {
        let source = "1000000 * 1000000 * 1000000 * 10";
        let tokens = crate::lexer::lex_expr(source)?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let r = crate::eval::eval_expr_program(&program, &[], &constants);
        assert!(
            matches!(r, Err(ExprError::IntegerOverflow)),
            "BH-BE-014: deeply nested overflow must be detected"
        );
        Ok(())
    }

    /// BH-BE-015: End-to-end correct precision for large non-overflowing values.
    #[test]
    fn blackhat_be_015_e2e_large_value_no_wrap() -> crate::ExprResult<()> {
        let source = "1000000 * 1000000 * 1000000";
        let tokens = crate::lexer::lex_expr(source)?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let r = crate::eval::eval_expr_program(&program, &[], &constants);
        assert_eq!(r, Ok(SlotValue::I64(1_000_000_000_000_000_000i64)));
        Ok(())
    }

    /// BH-BE-016: Negation of zero and positive values does not overflow.
    #[test]
    fn blackhat_be_016_neg_zero_no_overflow() -> crate::ExprResult<()> {
        let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(0))?;
        assert_eq!(r, SlotValue::I64(0));
        let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
        assert_eq!(r, SlotValue::I64(-42));
        let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(-42))?;
        assert_eq!(r, SlotValue::I64(42));
        Ok(())
    }

    /// BH-BE-017: Addition overflow at both boundaries.
    #[test]
    fn blackhat_be_017_add_both_boundaries() {
        let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
        let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }

    /// BH-BE-018: Subtraction overflow at both boundaries.
    #[test]
    fn blackhat_be_018_sub_both_boundaries() {
        let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
        let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MAX), SlotValue::I64(-1));
        assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    }
}
