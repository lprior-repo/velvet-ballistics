
    #[test]
    fn helper_with_nested_expression_argument() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("contains(1 + 2, 3)")?;
        adv_ensure(
            constants.len() == 3,
            "nested expression in helper should produce 3 constants",
        )?;
        adv_ensure(
            matches!(ops.last(), Some(ExprOp::Contains)),
            "should end with Contains",
        )?;
        Ok(())
    }

    // Additional edge-case: helper within a binary expression

    #[test]
    fn helper_result_used_in_binary_expression() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("length(1) + count(2)")?;
        // LoadConst, Length, LoadConst, Count, Add = 5 ops
        adv_ensure(ops.len() == 5, "helper in binary should have 5 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Add), "should end with Add")?;
        let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
        let has_count = ops.iter().any(|op| matches!(op, ExprOp::Count));
        adv_ensure(has_length, "should contain Length op")?;
        adv_ensure(has_count, "should contain Count op")?;
        Ok(())
    }

    // Additional edge-case: not operator applied to helper result

    #[test]
    fn not_applied_to_helper_result() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("not contains(1, 2)")?;
        adv_ensure(
            matches!(ops.last(), Some(ExprOp::Not)),
            "should end with Not",
        )?;
        let has_contains = ops.iter().any(|op| matches!(op, ExprOp::Contains));
        adv_ensure(has_contains, "should contain Contains op")?;
        Ok(())
    }

    // ── Edge-case: deeply nested expressions ────────────────────────────────

    #[test]
    fn deeply_nested_binary_tree_produces_valid_bytecode() -> Result<(), String> {
        // Build a balanced binary tree: ((1+2)+(3+4))+((5+6)+(7+8))
        let (ops, constants, max_stack) = lower("((1 + 2) + (3 + 4)) + ((5 + 6) + (7 + 8))")?;
        adv_ensure(constants.len() == 8, "should have 8 constants")?;
        // 8 LoadConst + 7 Add = 15 ops
        adv_ensure(ops.len() == 15, "should have 15 ops")?;
        let add_count = ops.iter().filter(|op| matches!(op, ExprOp::Add)).count();
        adv_ensure(add_count == 7, "should have 7 Add ops")?;
        adv_ensure(
            max_stack >= 4,
            "max_stack should be at least 4 for balanced tree",
        )?;
        Ok(())
    }

    #[test]
    fn deeply_nested_left_chain_arithmetic() -> Result<(), String> {
        // Left-deep chain of 20 additions: 1+2+3+...+20
        let parts: Vec<String> = (1..=20i64).map(|i| i.to_string()).collect();
        let expr = parts.join(" + ");
        let (ops, constants, max_stack) = lower(&expr)?;
        adv_ensure(constants.len() == 20, "should have 20 constants")?;
        // 20 loads + 19 adds = 39 ops
        adv_ensure(ops.len() == 39, "should have 39 ops")?;
        adv_ensure(
            max_stack >= 2,
            "left-deep chain should need at least 2 stack slots",
        )?;
        Ok(())
    }

    #[test]
    fn deeply_nested_mixed_and_or_precedence() -> Result<(), String> {
        // a or b and c or d and e => (a or (b and c)) or (d and e)
        let (ops, constants, _max_stack) = lower("true or false and true or false and true")?;
        // Constants: true, false, true, false, true = 5
        adv_ensure(constants.len() == 5, "should have 5 constants")?;
        let and_count = ops.iter().filter(|op| matches!(op, ExprOp::And)).count();
        let or_count = ops.iter().filter(|op| matches!(op, ExprOp::Or)).count();
        adv_ensure(and_count == 2, "should have 2 And ops")?;
        adv_ensure(or_count == 2, "should have 2 Or ops")?;
        // Root should be Or (left-assoc)
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Or), "root should be Or")?;
        Ok(())
    }

    #[test]
    fn deeply_nested_helper_inside_binary() -> Result<(), String> {
        // contains(length(1) == 0, true)
        // Wait, contains takes 2 args not a binary expr.
        // Let's use: length(1) == 0 and empty(1)
        let (ops, _constants, _max_stack) = lower("length(1) == 0 and empty(1)")?;
        let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
        let has_empty = ops.iter().any(|op| matches!(op, ExprOp::Empty));
        let has_eq = ops.iter().any(|op| matches!(op, ExprOp::Eq));
        let has_and = ops.iter().any(|op| matches!(op, ExprOp::And));
        adv_ensure(has_length, "should contain Length")?;
        adv_ensure(has_empty, "should contain Empty")?;
        adv_ensure(has_eq, "should contain Eq")?;
        adv_ensure(has_and, "should contain And")?;
        Ok(())
    }

    // ── Edge-case: operator precedence boundary conditions ──────────────────

    #[test]
    fn mul_has_higher_precedence_than_add() -> Result<(), String> {
        // 2 + 3 * 4 => 2, 3, 4, Mul, Add
        let (ops, constants, _max_stack) = lower("2 + 3 * 4")?;
        adv_ensure(constants.len() == 3, "should have 3 constants")?;
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        // Mul should come before Add
        let mul_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Mul))
            .ok_or("no Mul")?;
        let add_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Add))
            .ok_or("no Add")?;
        adv_ensure(mul_pos < add_pos, "Mul should come before Add in postfix")?;
        Ok(())
    }

    #[test]
    fn div_has_higher_precedence_than_sub() -> Result<(), String> {
        // 10 - 6 / 2 => 10, 6, 2, Div, Sub
        let (ops, _constants, _max_stack) = lower("10 - 6 / 2")?;
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        let div_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Div))
            .ok_or("no Div")?;
        let sub_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Sub))
            .ok_or("no Sub")?;
        adv_ensure(div_pos < sub_pos, "Div should come before Sub in postfix")?;
        Ok(())
    }

    #[test]
    fn and_has_higher_precedence_than_or() -> Result<(), String> {
        // true or false and true => true, false, true, And, Or
        let (ops, _constants, _max_stack) = lower("true or false and true")?;
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        let and_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::And))
            .ok_or("no And")?;
        let or_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Or))
            .ok_or("no Or")?;
        adv_ensure(and_pos < or_pos, "And should come before Or in postfix")?;
        Ok(())
    }

    #[test]
    fn comparison_has_higher_precedence_than_and() -> Result<(), String> {
        // 1 < 2 and 3 > 0 => 1, 2, Lt, 3, 0, Gt, And
        let (ops, _constants, _max_stack) = lower("1 < 2 and 3 > 0")?;
        let lt_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Lt))
            .ok_or("no Lt")?;
        let gt_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Gt))
            .ok_or("no Gt")?;
        let and_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::And))
            .ok_or("no And")?;
        adv_ensure(lt_pos < and_pos, "Lt should come before And")?;
        adv_ensure(gt_pos < and_pos, "Gt should come before And")?;
        Ok(())
    }

    #[test]
    fn equality_has_higher_precedence_than_and() -> Result<(), String> {
        // a == b and c != d => a, b, Eq, c, d, NotEq, And
        let (ops, _constants, _max_stack) = lower("1 == 2 and 3 != 4")?;
        adv_ensure(ops.len() == 7, "should have 7 ops")?;
        let eq_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Eq))
            .ok_or("no Eq")?;
        let noteq_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::NotEq))
            .ok_or("no NotEq")?;
        let and_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::And))
            .ok_or("no And")?;
        adv_ensure(eq_pos < and_pos, "Eq should come before And")?;
        adv_ensure(noteq_pos < and_pos, "NotEq should come before And")?;
        Ok(())
    }

    #[test]
    fn parentheses_override_precedence() -> Result<(), String> {
        // (1 + 2) * 3 => 1, 2, Add, 3, Mul (vs without parens: 1, 2, 3, Mul, Add)
        let (ops, constants, _max_stack) = lower("(1 + 2) * 3")?;
        adv_ensure(constants.len() == 3, "should have 3 constants")?;
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        let add_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Add))
            .ok_or("no Add")?;
        let mul_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Mul))
            .ok_or("no Mul")?;
        adv_ensure(add_pos < mul_pos, "Add should come before Mul with parens")?;
        Ok(())
    }

    #[test]
    fn nested_parens_override_all_precedence() -> Result<(), String> {
        // ((1 + 2) * (3 - 4)) / 5
        let (ops, constants, _max_stack) = lower("((1 + 2) * (3 - 4)) / 5")?;
        adv_ensure(constants.len() == 5, "should have 5 constants")?;
        // Add, Sub should come before Mul, Mul before Div
        let add_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Add))
            .ok_or("no Add")?;
        let sub_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Sub))
            .ok_or("no Sub")?;
        let mul_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Mul))
            .ok_or("no Mul")?;
        let div_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Div))
            .ok_or("no Div")?;
        adv_ensure(add_pos < mul_pos, "Add should come before Mul")?;
        adv_ensure(sub_pos < mul_pos, "Sub should come before Mul")?;
        adv_ensure(mul_pos < div_pos, "Mul should come before Div")?;
        Ok(())
    }

    #[test]
    fn unary_negation_has_highest_precedence() -> Result<(), String> {
        // -1 + 2 => the negation is applied to 1 first
        let (ops, constants, _max_stack) = lower("-1 + 2")?;
        // Const 0, Const 1, Sub, Const 2, Add
        adv_ensure(
            constants == vec![ConstValue::I64(0), ConstValue::I64(1), ConstValue::I64(2)],
            "negation constants should be 0, 1, 2",
        )?;
        let sub_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Sub))
            .ok_or("no Sub")?;
        let add_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Add))
            .ok_or("no Add")?;
        adv_ensure(sub_pos < add_pos, "Sub (negation) should come before Add")?;
        Ok(())
    }

    #[test]
    fn not_has_higher_precedence_than_comparison() -> Result<(), String> {
        // not true == false => (not true) == false
        let (ops, _constants, _max_stack) = lower("not true == false")?;
        let not_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Not))
            .ok_or("no Not")?;
        let eq_pos = ops
            .iter()
            .position(|op| matches!(op, ExprOp::Eq))
            .ok_or("no Eq")?;
        adv_ensure(not_pos < eq_pos, "Not should come before Eq")?;
        Ok(())
    }

    // ── Edge-case: helper function boundary conditions ──────────────────────

    #[test]
    fn helper_with_negated_argument() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("exists(-1)")?;
        // Const 0, Const 1, Sub, Exists
        adv_ensure(constants.len() == 2, "should have 2 constants (0 and 1)")?;
