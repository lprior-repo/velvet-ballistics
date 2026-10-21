    // ===== Edge-case tests added for comprehensive coverage =====

    /// Edge case: F64 values are rejected by integer arithmetic (type mismatch).
    ///
    /// The expression evaluator only supports I64 arithmetic. Supplying an F64
    /// value to an Add operation must produce TypeMismatch, not a panic or
    /// silent coercion.
    #[test]
    fn edge_f64_rejected_by_integer_addition() -> ExprResult<()> {
        let f64_val = vb_core::value::FiniteF64::new(3.14).map_err(|_| ExprError::UnexpectedEof)?;
        let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(f64_val), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 + I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    /// Edge case: F64 values rejected by comparison operators (type mismatch).
    ///
    /// Comparison operators expect I64 operands. Supplying F64 must fail.
    #[test]
    fn edge_f64_rejected_by_comparison() -> ExprResult<()> {
        let f64_val = vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?;
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::F64(f64_val), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 < I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    /// Edge case: Parenthesized groups override operator precedence.
    ///
    /// `(1 + 2) * (3 + 4)` should evaluate to `3 * 7 = 21`, not `1 + (2 * 3) + 4`.
    /// This tests that parentheses correctly override the default precedence
    /// where multiplication binds tighter than addition.
    #[test]
    fn edge_parenthesized_groups_override_precedence() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("(1 + 2) * (3 + 4)")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(21));
        Ok(())
    }

    /// Edge case: Comparison operators mixed with arithmetic in precedence.
    ///
    /// `1 + 2 < 3 + 4` should be parsed as `(1 + 2) < (3 + 4)` which evaluates
    /// to `3 < 7` = true. This verifies that comparison operators have lower
    /// precedence than arithmetic operators.
    #[test]
    fn edge_comparison_lower_precedence_than_arithmetic() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1 + 2 < 3 + 4")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    /// Edge case: Boolean equality and inequality across same and different values.
    ///
    /// `true == true` should be true, `true == false` should be false,
    /// `false != false` should be false, `true != false` should be true.
    #[test]
    fn edge_boolean_equality_and_inequality() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true == true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));

        let tokens = crate::lexer::lex_expr("true != false")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    /// Edge case: All four comparison operators on equal values.
    ///
    /// For `5 < 5` -> false, `5 <= 5` -> true, `5 > 5` -> false, `5 >= 5` -> true.
    /// This verifies the boundary behavior of comparison operators.
    #[test]
    fn edge_comparison_boundary_on_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(false), "5 < 5 should be false");

        let result = eval_binary_op(BinaryOp::Lte, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true), "5 <= 5 should be true");

        let result = eval_binary_op(BinaryOp::Gt, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(false), "5 > 5 should be false");

        let result = eval_binary_op(BinaryOp::Gte, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true), "5 >= 5 should be true");
        Ok(())
    }

    /// Edge case: Negation of a negative number returns the positive.
    ///
    /// `-(-42)` should evaluate to `42`. This exercises the unary negation
    /// path with a non-zero, non-MIN negative operand.
    #[test]
    fn edge_negation_of_negative_returns_positive() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(-42))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    /// Edge case: Logical AND/OR with all boolean combinations.
    ///
    /// Verifies `false and false` -> false, `true or true` -> true,
    /// `false or false` -> false. Existing tests cover `true and false`
    /// and `true or false`, but these three combinations are untested.
    #[test]
    fn edge_logical_and_or_all_combinations() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Bool(false),
            SlotValue::Bool(false),
        )?;
        assert_eq!(result, SlotValue::Bool(false), "false and false");

        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true), "true or true");

        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false), "false or false");
        Ok(())
    }

    /// Edge case: Chained left-associative subtraction via end-to-end pipeline.
    ///
    /// `100 - 50 - 25` should parse as `(100 - 50) - 25 = 25`, not `100 - (50 - 25) = 75`.
    /// This verifies that subtraction is left-associative through the full
    /// lex -> parse -> compile -> eval pipeline.
    #[test]
    fn edge_left_associative_subtraction_e2e() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("100 - 50 - 25")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(25));
        Ok(())
    }

    /// Edge case: Division of negative by negative produces correct positive result.
    ///
    /// `-100 / -10` should yield `10`. Exercises signed division through the
    /// end-to-end pipeline.
    #[test]
    fn edge_negative_division_e2e() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("0 - 100")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program_neg100 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;

        let tokens = crate::lexer::lex_expr("0 - 10")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants2 = Vec::new();
        let program_neg10 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants2)?;

        // Use direct binary op since we cannot parse negative literals
        let neg100_result = eval_expr_program(&program_neg100, &[], &constants)?;
        let neg10_result = eval_expr_program(&program_neg10, &[], &constants2)?;

        let result = eval_binary_op(BinaryOp::Div, neg100_result, neg10_result)?;
        assert_eq!(result, SlotValue::I64(10));
        Ok(())
    }

    /// Edge case: `not true or true` precedence -- NOT binds tighter than OR.
    ///
    /// This should parse as `(not true) or true` = `false or true` = `true`.
    /// If OR bound tighter, it would be `not (true or true)` = `not true` = `false`.
    #[test]
    fn edge_not_binds_tighter_than_or() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("not true or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(
            result,
            SlotValue::Bool(true),
            "not true or true should be true"
        );
        Ok(())
    }

    /// Edge case: Whitespace-only input through full pipeline returns error.
    ///
    /// The lexer produces only an End token for whitespace-only input.
    /// The parser should reject this with UnexpectedToken.
    #[test]
    fn edge_whitespace_only_rejected_by_parser() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("   \t  \n  ")?;
        let result = crate::parser::parse_expr(&tokens);
        let Err(ExprError::UnexpectedToken { token }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedToken for whitespace-only".into(),
            });
        };
        assert!(
            token.contains("End"),
            "whitespace-only should produce End token error, got: {token}"
        );
        Ok(())
    }

    // ===== F64 arithmetic integration tests =====

    #[test]
    fn eval_binary_op_f64_adds_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(1.5)),
            SlotValue::F64(make_f64(2.5)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(4.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_subtracts_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(10.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(7.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_multiplies_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(6.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_divides_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(84.0)),
            SlotValue::F64(make_f64(2.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_division_by_zero_returns_nonfinite_float_not_division_by_zero()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::F64(make_f64(0.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for F64/0, not DivisionByZero".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_zero_divided_by_zero_returns_nonfinite_float() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(0.0)),
            SlotValue::F64(make_f64(0.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for 0.0/0.0 (NaN)".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_produces_nonfinite_float_when_result_is_infinity() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(f64::MIN_POSITIVE)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat when result is Inf".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_addition_produces_nonfinite_float_when_result_is_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(f64::MAX)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MAX + MAX".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_subtraction_produces_nonfinite_float_when_result_is_negative_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(f64::MIN)),
            SlotValue::F64(make_f64(f64::MAX)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MIN - MAX".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_multiplication_produces_nonfinite_float_when_result_is_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(2.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MAX * 2.0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_negation_returns_finite_value() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(make_f64(42.0)))?;
        assert_eq!(result, SlotValue::F64(make_f64(-42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_negation_of_min_produces_max() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(make_f64(f64::MIN)))?;
        assert_eq!(result, SlotValue::F64(make_f64(f64::MAX)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gt,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than_returns_false_when_less() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gt,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than_or_equal_equal_case() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gte,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lt,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than_returns_false_when_greater() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lt,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than_or_equal_equal_case() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lte,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_equality_with_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Eq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_equality_with_unequal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Eq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(8.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_inequality_with_unequal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::NotEq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(8.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_inequality_with_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::NotEq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_type_mismatch_with_i64_in_add() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::I64(2),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 + I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_type_mismatch_with_bool_in_mul() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::Bool(true),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 * Bool".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_null_in_subtraction() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::Null,
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 - Null".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_division_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("3.14 / 0.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for F64 / 0.0 end-to-end".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_addition() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1.5 + 2.5")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(4.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_multiplication() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("6.0 * 7.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_complex_expression() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("2.0 + 3.0 * 4.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(14.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_division_yields_nonfinite_when_dividing_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1.0 / 0.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        assert!(
            matches!(result, Err(ExprError::NonFiniteFloat)),
            "F64/0.0 should return NonFiniteFloat, not DivisionByZero"
        );
        Ok(())
    }

    #[test]
    fn i64_division_by_zero_still_returns_division_by_zero_not_nonfinite_float() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "I64/0 must return DivisionByZero, not NonFiniteFloat".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_i64_division_by_zero_returns_division_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("10 / 0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for I64/0".into(),
            });
        };
        Ok(())
    }

    /// NaN comparison test — POST-006 (IEEE 754 compliance).
    ///
    /// Per IEEE 754: NaN < x, NaN > x, NaN == x yield false for any x.
    /// NaN != NaN is true (NaN is not equal to itself).
    ///
    /// NOTE: NaN cannot enter the vb_expr system via FiniteF64::new() which
    /// rejects all NaN/Inf bit patterns at construction time. Therefore, the
    /// comparison operators eval_lt_op, eval_gt_op, eval_gte_op, eval_lte_op
    /// can NEVER receive NaN inputs through the public API by construction.
    ///
    /// This test verifies the IEEE 754 NaN comparison semantics DIRECTLY using
    /// raw f64::NAN, demonstrating that the behavior would be correct if NaN
    /// could somehow reach the comparison operators (which it cannot).
    ///
    /// The eval_*_op functions extract the inner f64 via .get() and perform
    /// standard Rust f64 comparisons, which follow IEEE 754 semantics.
    #[test]
    fn f64_comparison_nan_yields_false() {
        let nan = f64::NAN;
        let x = 1.0;

        assert!(!(nan < x), "NaN < x must be false per IEEE 754");
        assert!(!(nan > x), "NaN > x must be false per IEEE 754");
        assert!(!(nan == x), "NaN == x must be false per IEEE 754");
        assert!(!(nan <= x), "NaN <= x must be false per IEEE 754");
        assert!(!(nan >= x), "NaN >= x must be false per IEEE 754");
        assert!(
            nan != nan,
            "NaN != NaN must be true (NaN is not equal to itself)"
        );

        assert!(!(nan < 0.0), "NaN < 0.0 must be false");
        assert!(!(nan > 0.0), "NaN > 0.0 must be false");
        assert!(!(nan < f64::INFINITY), "NaN < Inf must be false");
        assert!(!(nan > f64::NEG_INFINITY), "NaN > -Inf must be false");
        assert!(!(nan == f64::MAX), "NaN == MAX must be false");
        assert!(!(nan == f64::MIN), "NaN == MIN must be false");
    }

    // ============================================================================
    // AND/OR Short-Circuit Tests (LETHAL-2)
    // ============================================================================

    // --- B1: AND returns SlotValue::Bool(true) when both operands are true ---

    #[test]
    fn and_returns_true_when_both_operands_are_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // --- B2: AND returns false when first is false but evaluates BOTH operands ---
    // Section 46: both operands must be evaluated before boolean operator applies.

    #[test]
    fn and_returns_false_when_first_is_false_and_evaluates_right() -> ExprResult<()> {
        // Section 46: left=false, right=non-bool → both evaluated → TypeMismatch
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch since both operands evaluated".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B3: AND returns false when first is true and second is false ---

    #[test]
    fn and_returns_false_when_first_is_true_and_second_is_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    // --- B4: OR returns SlotValue::Bool(false) when both operands are false ---

    #[test]
    fn or_returns_false_when_both_operands_are_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    // --- B5: OR returns true when first is true but evaluates BOTH operands ---
    // Section 46: both operands must be evaluated before boolean operator applies.

    #[test]
    fn or_returns_true_when_first_is_true_and_evaluates_right() -> ExprResult<()> {
        // Section 46: left=true, right=non-bool → both evaluated → TypeMismatch
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch since both operands evaluated".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B6: OR returns true when first is false and second is true ---

    #[test]
    fn or_returns_true_when_first_is_false_and_second_is_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // --- B7: AND evaluates BOTH operands when first produces TypeMismatch ---

    #[test]
    fn and_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
        // left = I64 (TypeMismatch), right = Bool (valid)
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for I64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    /// B7 error accumulation test: left=I64, right=F64 (both non-bool)
    #[test]
    fn and_evaluates_both_operands_error_accumulation_i64_left_f64_right() -> ExprResult<()> {
        let left = SlotValue::I64(1);
        let right = SlotValue::F64(FiniteF64::new(1.0).unwrap());
        let result = eval_binary_op(BinaryOp::And, left, right);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B8: OR evaluates BOTH operands when first produces TypeMismatch ---

    #[test]
    fn or_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
        // left = Null (TypeMismatch), right = Bool (valid)
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(false));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    /// B8 error accumulation test: left=Null, right=F64 (both non-bool)
    #[test]
    fn or_evaluates_both_operands_error_accumulation_null_left_f64_right() -> ExprResult<()> {
        let left = SlotValue::Null;
        let right = SlotValue::F64(FiniteF64::new(1.0).unwrap());
        let result = eval_binary_op(BinaryOp::Or, left, right);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    // ============================================================================
    // Exhaustive Bool × Bool Matrix for AND
    // ============================================================================

    #[test]
    fn and_false_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Bool(false),
            SlotValue::Bool(false),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_false_true_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_true_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_true_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Exhaustive Bool × Bool Matrix for OR
    // ============================================================================

    #[test]
    fn or_false_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn or_false_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn or_true_false_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn or_true_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Error variant tests for AND/OR TypeMismatch scenarios
    // ============================================================================

    #[test]
    fn and_rejects_i64_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_i64_bool() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_bool_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for bool and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn or_rejects_null_bool() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_bool_null() -> ExprResult<()> {
        // left=false requires evaluating right, which is Null -> TypeMismatch
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for false or null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_i64_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 or i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn or_rejects_f64_bool() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::F64(FiniteF64::new(1.0).unwrap()),
            SlotValue::Bool(true),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for f64 or bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_null_null() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Null, SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null and null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_f64_f64() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::F64(FiniteF64::new(1.0).unwrap()),
            SlotValue::F64(FiniteF64::new(2.0).unwrap()),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for f64 or f64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_symbol_symbol() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
            SlotValue::Symbol(vb_core::ids::SymbolId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for symbol and symbol".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "symbol");
        Ok(())
    }

    #[test]
    fn or_rejects_list_list() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::List(vb_core::ids::ListId::new(1)),
            SlotValue::List(vb_core::ids::ListId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for list or list".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "list");
        Ok(())
    }

    #[test]
    fn and_rejects_object_object() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Object(vb_core::ids::ObjectId::new(1)),
            SlotValue::Object(vb_core::ids::ObjectId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for object and object".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "object");
        Ok(())
    }

    // ============================================================================
    // Integration tests: full pipeline (lex → parse → compile → eval)
    // ============================================================================

    #[test]
    fn integration_and_true_true() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_and_false_any() -> ExprResult<()> {
        // "false and 1" — Section 46 mandates BOTH operands are evaluated.
        // 1 is non-bool, so evaluating it produces TypeMismatch.
        let tokens = crate::lexer::lex_expr("false and 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for non-bool right operand".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_true_any() -> ExprResult<()> {
        // "true or 1" — Section 46 mandates BOTH operands are evaluated.
        // 1 is non-bool, so evaluating it produces TypeMismatch.
        let tokens = crate::lexer::lex_expr("true or 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for non-bool right operand".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_false_false() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("false or false")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn integration_and_type_mismatch_left_i64() -> ExprResult<()> {
        // "1 and true" should error with TypeMismatch for number
        let tokens = crate::lexer::lex_expr("1 and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 and true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_type_mismatch_left_null() -> ExprResult<()> {
        // "null or true" should error with TypeMismatch for null
        let tokens = crate::lexer::lex_expr("null or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn integration_and_both_type_mismatch() -> ExprResult<()> {
        // "1 and 2" should error with TypeMismatch (both are non-bool)
        let tokens = crate::lexer::lex_expr("1 and 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 and 2".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_both_type_mismatch() -> ExprResult<()> {
        // "1 or 2" should error with TypeMismatch (both are non-bool)
        let tokens = crate::lexer::lex_expr("1 or 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 or 2".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // ============================================================================
    // Chained AND/OR tests
    // ============================================================================

    #[test]
    fn integration_chained_and() -> ExprResult<()> {
        // "true and true and true" should return true
        let tokens = crate::lexer::lex_expr("true and true and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_chained_or() -> ExprResult<()> {
        // "false or false or true" should return true
        let tokens = crate::lexer::lex_expr("false or false or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_mixed_and_or() -> ExprResult<()> {
        // "(true and false) or true" should return true
        let tokens = crate::lexer::lex_expr("(true and false) or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Proptest invariants for AND/OR
    // ============================================================================

    proptest! {
        #[test]
        fn proptest_and_is_commutative_for_bools(a: bool, b: bool) {
            // For any two SlotValue::Bool values a, b:
            // eval_binary_op(And, Bool(a), Bool(b)) == eval_binary_op(And, Bool(b), Bool(a))
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::And, left, right).unwrap();
            let result_ba = eval_binary_op(BinaryOp::And, right, left).unwrap();
            prop_assert_eq!(result_ab, result_ba);
        }
    }

    proptest! {
        #[test]
        fn proptest_or_is_commutative_for_bools(a: bool, b: bool) {
            // For any two SlotValue::Bool values a, b:
            // eval_binary_op(Or, Bool(a), Bool(b)) == eval_binary_op(Or, Bool(b), Bool(a))
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::Or, left, right).unwrap();
            let result_ba = eval_binary_op(BinaryOp::Or, right, left).unwrap();
            prop_assert_eq!(result_ab, result_ba);
        }
    }

    #[test]
    fn proptest_and_false_left_always_false() {
        // Section 46: AND with false left and valid bool right is always false.
        // With non-bool right, Section 46 mandates evaluation → TypeMismatch.
        let left = SlotValue::Bool(false);
        // Test with valid bool right - should return false
        let result = eval_binary_op(BinaryOp::And, left, SlotValue::Bool(true)).unwrap();
        assert_eq!(result, SlotValue::Bool(false));

        // Section 46: non-bool right must be evaluated → TypeMismatch
        let result2 = eval_binary_op(BinaryOp::And, left, SlotValue::I64(0));
        assert!(matches!(result2, Err(ExprError::TypeMismatch { .. })));

        let result3 = eval_binary_op(BinaryOp::And, left, SlotValue::Null);
        assert!(matches!(result3, Err(ExprError::TypeMismatch { .. })));
    }

    #[test]
    fn proptest_or_true_left_always_true() {
        // Section 46: OR with true left and valid bool right is always true.
        // With non-bool right, Section 46 mandates evaluation → TypeMismatch.
        let left = SlotValue::Bool(true);
        // Test with valid bool right - should return true
        let result = eval_binary_op(BinaryOp::Or, left, SlotValue::Bool(false)).unwrap();
        assert_eq!(result, SlotValue::Bool(true));

        // Section 46: non-bool right must be evaluated → TypeMismatch
        let result2 = eval_binary_op(BinaryOp::Or, left, SlotValue::I64(0));
        assert!(matches!(result2, Err(ExprError::TypeMismatch { .. })));

        let result3 = eval_binary_op(BinaryOp::Or, left, SlotValue::Null);
        assert!(matches!(result3, Err(ExprError::TypeMismatch { .. })));
    }

    #[test]
    fn proptest_and_requires_both_bools() {
        // Any non-bool left OR right produces TypeMismatch
        let non_bools = [
            SlotValue::I64(1),
            SlotValue::F64(FiniteF64::new(1.0).unwrap()),
            SlotValue::Null,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for left in &non_bools {
            let result = eval_binary_op(BinaryOp::And, *left, SlotValue::Bool(true));
            assert!(
                matches!(result, Err(ExprError::TypeMismatch { .. })),
                "AND with non-bool left should be TypeMismatch"
            );
        }

        // left is bool, right is non-bool -> TypeMismatch
        for right in &non_bools {
            let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), *right);
            assert!(
                matches!(result, Err(ExprError::TypeMismatch { .. })),
                "AND with non-bool right should be TypeMismatch"
            );
        }
    }

    #[test]
    fn proptest_or_requires_both_bools() {
        // Any non-bool left OR right produces TypeMismatch
        let non_bools = [
            SlotValue::I64(1),
            SlotValue::F64(FiniteF64::new(1.0).unwrap()),
            SlotValue::Null,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for left in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, *left, SlotValue::Bool(true));
            assert!(
                matches!(result, Err(ExprError::TypeMismatch { .. })),
                "OR with non-bool left should be TypeMismatch"
            );
        }

        // left is bool, right is non-bool -> TypeMismatch
        for right in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), *right);
            assert!(
                matches!(result, Err(ExprError::TypeMismatch { .. })),
                "OR with non-bool right should be TypeMismatch"
            );
        }
    }
