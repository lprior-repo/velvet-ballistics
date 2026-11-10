    // ===== Mixed I64/F64 promotion tests (vb-cofe1) =====

    #[test]
    fn eval_binary_op_mixed_sub_mul_div_promote_to_f64() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::I64(10),
            SlotValue::F64(make_f64(0.5)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(9.5)));
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(2.5)),
            SlotValue::I64(4),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(10.0)));
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::I64(1),
            SlotValue::F64(make_f64(2.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(0.5)));
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(7.5)),
            SlotValue::I64(2),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(3.75)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_mixed_div_by_zero_returns_nonfinite_float() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::I64(1),
            SlotValue::F64(make_f64(0.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for I64 / F64(0.0)".into(),
            });
        };
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::I64(0),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for F64 / I64(0)".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_mixed_comparisons_promote_operands() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lt,
            SlotValue::I64(3),
            SlotValue::F64(make_f64(3.5)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        let result = eval_binary_op(
            BinaryOp::Gt,
            SlotValue::I64(10),
            SlotValue::F64(make_f64(9.5)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        let result = eval_binary_op(
            BinaryOp::Lte,
            SlotValue::F64(make_f64(-0.5)),
            SlotValue::I64(0),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_mixed_sources_typecheck_then_evaluate() -> ExprResult<()> {
        let cases: [(&str, crate::typecheck::ExprType, SlotValue); 7] = [
            (
                "1 + 2.5",
                crate::typecheck::ExprType::F64,
                SlotValue::F64(make_f64(3.5)),
            ),
            (
                "10 - 0.5",
                crate::typecheck::ExprType::F64,
                SlotValue::F64(make_f64(9.5)),
            ),
            (
                "3 * 1.5",
                crate::typecheck::ExprType::F64,
                SlotValue::F64(make_f64(4.5)),
            ),
            (
                "7.5 / 2.5",
                crate::typecheck::ExprType::F64,
                SlotValue::F64(make_f64(3.0)),
            ),
            (
                "5 > 2.5",
                crate::typecheck::ExprType::Bool,
                SlotValue::Bool(true),
            ),
            (
                "2.5 < 5",
                crate::typecheck::ExprType::Bool,
                SlotValue::Bool(true),
            ),
            ("2 + 3", crate::typecheck::ExprType::I64, SlotValue::I64(5)),
        ];
        for (source, expected_type, expected_value) in cases {
            let tokens = crate::lexer::lex_expr(source)?;
            let ast = crate::parser::parse_expr(&tokens)?;
            let inferred =
                crate::typecheck::typecheck_expr(&ast, &crate::typecheck::TypeContext::new())?;
            assert_eq!(inferred, expected_type);
            let mut constants = Vec::new();
            let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
            let result = eval_expr_program(&program, &[], &constants)?;
            assert_eq!(result, expected_value);
        }
        Ok(())
    }

