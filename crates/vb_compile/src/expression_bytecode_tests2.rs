        adv_ensure(
            result.is_err(),
            "constant pool overflow (65536 existing + 1 new) should produce an error",
        )
    }

    #[test]
    fn helper_zero_args_rejected_with_arity_mismatch() -> Result<(), String> {
        let error = adv_lower_error("contains()")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "contains",
                    expected: 2,
                    actual: 0
                }
            ),
            "contains() did not produce arity mismatch",
        )
    }

    #[test]
    fn helper_too_many_args_rejected_with_arity_mismatch() -> Result<(), String> {
        let error = adv_lower_error("append_if(1, 2)")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "append_if",
                    expected: 3,
                    actual: 2
                }
            ),
            "append_if(1, 2) did not produce arity mismatch",
        )
    }

    #[test]
    fn slot_accessor_with_non_numeric_root_rejected() -> Result<(), String> {
        let expr = parse_expression("$slot.abc").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
        adv_ensure(
            matches!(
                result,
                Err(CompileError::UnknownReferenceName { kind: "slot", .. })
            ),
            "non-numeric slot index did not produce slot reference error",
        )
    }

    #[test]
    fn unknown_reference_root_rejected_in_slot_accessor_path() -> Result<(), String> {
        let expr = parse_expression("$unknown.5").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
        adv_ensure(
            matches!(result, Err(CompileError::UnknownReferenceRoot { root, .. }) if root.as_ref() == "unknown"),
            "unknown root did not produce UnknownReferenceRoot",
        )
    }

    #[test]
    fn deeply_nested_arithmetic_produces_valid_bytecode() -> Result<(), String> {
        // Build a left-associative chain: 1 + 2 + 3 + 4 + 5
        let (ops, constants, max_stack) = lower("1 + 2 + 3 + 4 + 5")?;
        adv_ensure(constants.len() == 5, "should have 5 constants")?;
        adv_ensure(ops.len() == 9, "should have 5 loads + 4 adds = 9 ops")?;
        adv_ensure(
            max_stack >= 2,
            "max_stack should be at least 2 for left-assoc chain",
        )?;
        Ok(())
    }

    #[test]
    fn nested_negation_produces_correct_bytecode() -> Result<(), String> {
        // --5 should produce: LoadConst(0), LoadConst(0), LoadConst(5), Sub, Sub
        let (ops, _constants, _max_stack) = lower("--5")?;
        adv_ensure(ops.len() == 5, "nested negation should produce 5 ops")?;
        // Check last two ops are Sub
        let fourth = ops.get(3).ok_or("missing 4th op")?;
        let fifth = ops.get(4).ok_or("missing 5th op")?;
        adv_ensure(matches!(fourth, ExprOp::Sub), "4th op should be Sub")?;
        adv_ensure(matches!(fifth, ExprOp::Sub), "5th op should be Sub")?;
        Ok(())
    }

    // ── Edge-case expression bytecode tests ──────────────────────────────────

    // 1. Empty string constant expressions

    #[test]
    fn empty_string_literal_rejected_as_unsupported() -> Result<(), String> {
        let error = adv_lower_error("\"\"")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLoweringUnsupported { ref feature }
                    if feature.as_ref() == "text constants"
            ),
            "empty string literal did not produce text constants diagnostic",
        )
    }

    // 2. Non-empty string constants (rejected as ExpressionLoweringUnsupported)

    #[test]
    fn nonempty_string_literal_rejected_as_unsupported() -> Result<(), String> {
        let error = adv_lower_error("\"hello world\"")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLoweringUnsupported { ref feature }
                    if feature.as_ref() == "text constants"
            ),
            "non-empty string literal did not produce text constants diagnostic",
        )
    }

    #[test]
    fn string_in_helper_call_rejected_as_unsupported() -> Result<(), String> {
        // contains($slot.0, "needle") - first arg is a reference (rejected),
        // but the string arg alone would also fail if lowered first
        let error = adv_lower_error("contains(\"a\", \"b\")")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLoweringUnsupported { ref feature }
                    if feature.as_ref() == "text constants"
            ),
            "string arguments in helper should be rejected as text constants",
        )
    }

    // 3. Large integer constants (near i64::MAX, i64::MIN, zero)

    #[test]
    fn zero_integer_constant_lowers_correctly() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("0")?;
        adv_ensure(
            constants == vec![ConstValue::I64(0)],
            "zero should produce I64(0) constant",
        )?;
        adv_ensure(ops.len() == 1, "single zero should produce one op")?;
        adv_ensure(
            matches!(ops.first(), Some(ExprOp::LoadConst(_))),
            "zero should produce LoadConst op",
        )
    }

    #[test]
    fn near_max_integer_constant_lowers_correctly() -> Result<(), String> {
        let source = i64::MAX.to_string();
        let (ops, constants, _max_stack) = lower(&source)?;
        adv_ensure(
            constants == vec![ConstValue::I64(i64::MAX)],
            "i64::MAX should produce correct constant",
        )?;
        adv_ensure(
            matches!(ops.first(), Some(ExprOp::LoadConst(_))),
            "i64::MAX should produce LoadConst op",
        )
    }

    #[test]
    fn near_min_integer_constant_lowers_correctly() -> Result<(), String> {
        // i64::MIN = -9223372036854775808 cannot be parsed as a literal because
        // the lexer treats the minus as unary negation and 9223372036854775808
        // overflows i64::MAX. Instead verify a large negative constant is lowered
        // through the negation path correctly.
        let (ops, constants, _max_stack) = lower("-9999999999")?;
        adv_ensure(
            constants == vec![ConstValue::I64(0), ConstValue::I64(9999999999)],
            "large negative should produce 0 and absolute value constants",
        )?;
        adv_ensure(ops.len() == 3, "negation should produce 3 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Sub), "should end with Sub")?;
        Ok(())
    }

    #[test]
    fn negative_one_integer_constant_lowers_correctly() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("-1")?;
        adv_ensure(
            constants == vec![ConstValue::I64(0), ConstValue::I64(1)],
            "negation of 1 should produce 0 and 1 constants",
        )?;
        adv_ensure(ops.len() == 3, "negation should produce 3 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Sub), "negation should end with Sub")?;
        Ok(())
    }

    #[test]
    fn large_integer_in_binary_expression() -> Result<(), String> {
        let source = format!("{} + {}", i64::MAX - 1, 1);
        let (ops, constants, _max_stack) = lower(&source)?;
        adv_ensure(constants.len() == 2, "should have 2 constants")?;
        adv_ensure(
            constants.first() == Some(&ConstValue::I64(i64::MAX - 1)),
            "first constant should be i64::MAX - 1",
        )?;
        adv_ensure(
            constants.get(1) == Some(&ConstValue::I64(1)),
            "second constant should be 1",
        )?;
        adv_ensure(ops.len() == 3, "should have 2 loads + 1 add = 3 ops")?;
        Ok(())
    }

    // 4. Boolean constant expressions (true, false)

    #[test]
    fn true_boolean_lowers_to_const() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("true")?;
        adv_ensure(
            constants == vec![ConstValue::Bool(true)],
            "true should produce Bool(true) constant",
        )?;
        adv_ensure(ops.len() == 1, "true should produce one op")?;
        adv_ensure(
            matches!(ops.first(), Some(ExprOp::LoadConst(_))),
            "true should produce LoadConst op",
        )
    }

    #[test]
    fn false_boolean_lowers_to_const() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("false")?;
        adv_ensure(
            constants == vec![ConstValue::Bool(false)],
            "false should produce Bool(false) constant",
        )?;
        adv_ensure(ops.len() == 1, "false should produce one op")?;
        adv_ensure(
            matches!(ops.first(), Some(ExprOp::LoadConst(_))),
            "false should produce LoadConst op",
        )
    }

    #[test]
    fn boolean_equality_expression_lowers_correctly() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("true == false")?;
        adv_ensure(
            constants == vec![ConstValue::Bool(true), ConstValue::Bool(false)],
            "true == false should produce two boolean constants",
        )?;
        adv_ensure(
            ops.len() == 3,
            "should have 3 ops (LoadConst, LoadConst, Eq)",
        )?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Eq), "should end with Eq op")?;
        Ok(())
    }

    // 5. Null constant expressions

    #[test]
    fn null_constant_lowers_to_const() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("null")?;
        adv_ensure(
            constants == vec![ConstValue::Null],
            "null should produce Null constant",
        )?;
        adv_ensure(ops.len() == 1, "null should produce one op")?;
        adv_ensure(
            matches!(ops.first(), Some(ExprOp::LoadConst(_))),
            "null should produce LoadConst op",
        )
    }

    #[test]
    fn null_equality_expression_lowers_correctly() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("null == null")?;
        adv_ensure(
            constants == vec![ConstValue::Null, ConstValue::Null],
            "null == null should produce two null constants",
        )?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Eq), "should end with Eq")?;
        Ok(())
    }

    #[test]
    fn null_inequality_expression_lowers_correctly() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("null != 0")?;
        adv_ensure(
            constants == vec![ConstValue::Null, ConstValue::I64(0)],
            "null != 0 should produce null and zero constants",
        )?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::NotEq), "should end with NotEq")?;
        Ok(())
    }
