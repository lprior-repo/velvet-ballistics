        Ok(())
    }

    #[test]
    fn accessor_with_single_segment_creates_one_path_entry() -> Result<(), String> {
        let (ops, _constants, accessors) = lower_with_accessors("$slots.5.0")?;
        adv_ensure(ops.len() == 1, "should be single LoadAccessor")?;
        let accessor = accessors.first().ok_or("missing accessor")?;
        adv_ensure(accessor.root == SlotIdx::new(5), "root should be slot 5")?;
        adv_ensure(accessor.path.len() == 1, "should have 1 segment")?;
        match accessor.path.first() {
            Some(PathSegment::Index(0)) => Ok(()),
            other => Err(format!("expected Index(0), got {other:?}")),
        }
    }

    // ── Edge-case: multiple expressions sharing a constant pool ──────────────

    #[test]
    fn two_expressions_share_constant_pool_independently() -> Result<(), String> {
        // Lower two separate expressions into the same constants vec
        let expr1 = parse_expression("1 + 2").map_err(|e| e.to_string())?;
        let expr2 = parse_expression("3 + 4").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let _prog1 = compile_expr_to_bytecode(&expr1, &mut constants).map_err(|e| e.to_string())?;
        adv_ensure(
            constants.len() == 2,
            "first expression should add 2 constants",
        )?;
        let _prog2 = compile_expr_to_bytecode(&expr2, &mut constants).map_err(|e| e.to_string())?;
        adv_ensure(
            constants.len() == 4,
            "second expression should add 2 more constants",
        )?;
        adv_ensure(
            constants
                == vec![
                    ConstValue::I64(1),
                    ConstValue::I64(2),
                    ConstValue::I64(3),
                    ConstValue::I64(4),
                ],
            "constants should be [1, 2, 3, 4]",
        )?;
        Ok(())
    }

    #[test]
    fn expression_with_max_constants_near_overflow_boundary() -> Result<(), String> {
        // Fill constants to u16::MAX - 1 and verify the expression still compiles
        let expr = parse_expression("1").map_err(|e| e.to_string())?;
        let fill_count = usize::from(u16::MAX) - 1;
        let mut constants = Vec::with_capacity(fill_count + 1);
        for i in 0..fill_count {
            let value = i64::try_from(i).map_err(|error| error.to_string())?;
            constants.push(ConstValue::I64(value));
        }
        // constants has 65534 entries; pushing one more should succeed (65535 < 65536)
        let result = compile_expr_to_bytecode(&expr, &mut constants);
        adv_ensure(
            result.is_ok(),
            "should succeed with u16::MAX - 1 existing constants",
        )?;
        adv_ensure(
            constants.len() == fill_count + 1,
            "should have one more constant",
        )?;
        Ok(())
    }

    // ── Edge-case: chained comparisons left-associativity ───────────────────

    #[test]
    fn chained_lt_operators_left_associative() -> Result<(), String> {
        // 1 < 2 < 3 => (1 < 2) < 3
        let (ops, constants, _max_stack) = lower("1 < 2 < 3")?;
        adv_ensure(constants.len() == 3, "should have 3 constants")?;
        adv_ensure(ops.len() == 5, "should have 5 ops (3 loads + 2 Lt)")?;
        let lt_count = ops.iter().filter(|op| matches!(op, ExprOp::Lt)).count();
        adv_ensure(lt_count == 2, "should have 2 Lt ops")?;
        Ok(())
    }

    #[test]
    fn chained_gte_operators_left_associative() -> Result<(), String> {
        // 5 >= 3 >= 1 => (5 >= 3) >= 1
        let (ops, constants, _max_stack) = lower("5 >= 3 >= 1")?;
        adv_ensure(constants.len() == 3, "should have 3 constants")?;
        let gte_count = ops.iter().filter(|op| matches!(op, ExprOp::Gte)).count();
        adv_ensure(gte_count == 2, "should have 2 Gte ops")?;
        Ok(())
    }

    #[test]
    fn mixed_add_mul_left_assoc_same_precedence() -> Result<(), String> {
        // 1 + 2 - 3 => left-assoc: (1 + 2) - 3
        let (ops, _constants, _max_stack) = lower("1 + 2 - 3")?;
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        // First binary op should be Add, second should be Sub
        let first_bin = ops
            .iter()
            .find(|op| matches!(op, ExprOp::Add | ExprOp::Sub))
            .ok_or("missing first binary op")?;
        adv_ensure(
            matches!(first_bin, ExprOp::Add),
            "first binary op should be Add",
        )?;
        Ok(())
    }

    // ── Edge-case: negation of helper result ────────────────────────────────

    #[test]
    fn negation_of_helper_result_produces_sub() -> Result<(), String> {
        // -length(1) => Const 0, Const 1, Length, Sub
        let (ops, constants, _max_stack) = lower("-length(1)")?;
        adv_ensure(
            constants.first() == Some(&ConstValue::I64(0)),
            "first constant should be 0 for negation",
        )?;
        let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
        let has_sub = ops.iter().any(|op| matches!(op, ExprOp::Sub));
        adv_ensure(has_length, "should contain Length")?;
        adv_ensure(has_sub, "should contain Sub for negation")?;
        let sub_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Sub))
            .ok_or("no Sub")?;
        let length_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Length))
            .ok_or("no Length")?;
        adv_ensure(length_pos < sub_pos, "Length should come before Sub")?;
        Ok(())
    }

    #[test]
    fn not_of_not_of_boolean() -> Result<(), String> {
        // not not false => LoadConst(false), Not, Not
        let (ops, constants, _max_stack) = lower("not not false")?;
        adv_ensure(
            constants == vec![ConstValue::Bool(false)],
            "should have Bool(false)",
        )?;
        adv_ensure(ops.len() == 3, "should have 3 ops")?;
        let not_count = ops.iter().filter(|op| matches!(op, ExprOp::Not)).count();
        adv_ensure(not_count == 2, "should have 2 Not ops")?;
        Ok(())
    }

    // ── Edge-case: max stack tracking ───────────────────────────────────────

    #[test]
    fn max_stack_one_for_simple_load() -> Result<(), String> {
        let (_, _, max_stack) = lower("42")?;
        adv_ensure(max_stack == 1, "single load should have max_stack 1")
    }

    #[test]
    fn max_stack_two_for_binary_op() -> Result<(), String> {
        let (_, _, max_stack) = lower("1 + 2")?;
        adv_ensure(max_stack >= 2, "binary op should have max_stack >= 2")
    }

    #[test]
    fn max_stack_increases_with_complexity() -> Result<(), String> {
        let (_, _, ms_simple) = lower("1 + 2")?;
        let (_, _, ms_complex) = lower("1 + 2 * 3")?;
        adv_ensure(
            ms_complex >= ms_simple,
            "more complex expression should have >= max_stack",
        )
    }
}
