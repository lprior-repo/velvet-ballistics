
    // 6. Nested expressions (parenthesized, deeply nested)

    #[test]
    fn parenthesized_expression_lowers_identically() -> Result<(), String> {
        let (ops_unparen, constants_unparen, max_unparen) = lower("1 + 2")?;
        let (ops_paren, constants_paren, max_paren) = lower("(1 + 2)")?;
        adv_ensure(
            ops_unparen == ops_paren,
            "parenthesized ops should match unparenthesized",
        )?;
        adv_ensure(
            constants_unparen == constants_paren,
            "parenthesized constants should match unparenthesized",
        )?;
        adv_ensure(
            max_unparen == max_paren,
            "parenthesized max_stack should match unparenthesized",
        )?;
        Ok(())
    }

    #[test]
    fn nested_parentheses_preserve_precedence() -> Result<(), String> {
        // ((1 + 2)) should be the same as 1 + 2
        let (ops, _constants, _max_stack) = lower("((1 + 2))")?;
        adv_ensure(ops.len() == 3, "double-parenthesized 1+2 should be 3 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Add), "should end with Add")?;
        Ok(())
    }

    #[test]
    fn deeply_nested_unary_not_lowers_correctly() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("not not true")?;
        adv_ensure(ops.len() == 3, "not not true should be 3 ops")?;
        adv_ensure(
            matches!(ops.get(1), Some(ExprOp::Not)),
            "second op should be Not",
        )?;
        adv_ensure(
            matches!(ops.get(2), Some(ExprOp::Not)),
            "third op should be Not",
        )?;
        Ok(())
    }

    #[test]
    fn deeply_nested_mixed_arithmetic_and_negation() -> Result<(), String> {
        let (ops, _constants, max_stack) = lower("-(1 + -(2 * 3))")?;
        adv_ensure(
            max_stack >= 3,
            "nested negation and arithmetic should need stack >= 3",
        )?;
        // Verify the expression compiles without error and has reasonable ops
        adv_ensure(
            ops.len() >= 7,
            "complex nested expression should have many ops",
        )?;
        Ok(())
    }

    // 7. BinaryOp edge cases (division, multiplication, subtraction)

    #[test]
    fn division_lowers_to_div_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("10 / 2")?;
        adv_ensure(
            ops.len() == 3,
            "division should be 3 ops (LoadConst, LoadConst, Div)",
        )?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Div), "should end with Div op")?;
        Ok(())
    }

    #[test]
    fn multiplication_lowers_to_mul_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("3 * 4")?;
        adv_ensure(ops.len() == 3, "multiplication should be 3 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Mul), "should end with Mul op")?;
        Ok(())
    }

    #[test]
    fn subtraction_lowers_to_sub_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("10 - 3")?;
        adv_ensure(ops.len() == 3, "subtraction should be 3 ops")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Sub), "should end with Sub op")?;
        Ok(())
    }

    #[test]
    fn subtraction_with_addition_left_associative() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("10 - 3 + 2")?;
        // left assoc: (10 - 3) + 2 => LoadConst, LoadConst, Sub, LoadConst, Add = 5 ops
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        adv_ensure(
            matches!(ops.get(2), Some(ExprOp::Sub)),
            "3rd op should be Sub",
        )?;
        adv_ensure(
            matches!(ops.get(4), Some(ExprOp::Add)),
            "5th op should be Add",
        )?;
        Ok(())
    }

    #[test]
    fn division_and_multiplication_left_associative() -> Result<(), String> {
        let (ops, constants, _max_stack) = lower("12 / 3 * 2")?;
        adv_ensure(constants.len() == 3, "should have 3 constants")?;
        // (12 / 3) * 2 => LoadConst, LoadConst, Div, LoadConst, Mul = 5 ops
        adv_ensure(ops.len() == 5, "should have 5 ops")?;
        adv_ensure(
            matches!(ops.get(2), Some(ExprOp::Div)),
            "Div should be at index 2",
        )?;
        adv_ensure(
            matches!(ops.get(4), Some(ExprOp::Mul)),
            "Mul should be at index 4",
        )?;
        Ok(())
    }

    // 8. Comparison operators (Gt, Gte, Lt, Lte, NotEq, And, Or)

    #[test]
    fn greater_than_lowers_to_gt_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("5 > 3")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Gt), "should end with Gt")?;
        Ok(())
    }

    #[test]
    fn greater_than_or_equal_lowers_to_gte_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("5 >= 3")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Gte), "should end with Gte")?;
        Ok(())
    }

    #[test]
    fn less_than_lowers_to_lt_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("3 < 5")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Lt), "should end with Lt")?;
        Ok(())
    }

    #[test]
    fn less_than_or_equal_lowers_to_lte_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("3 <= 5")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Lte), "should end with Lte")?;
        Ok(())
    }

    #[test]
    fn not_equal_lowers_to_noteq_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("1 != 2")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::NotEq), "should end with NotEq")?;
        Ok(())
    }

    #[test]
    fn and_operator_lowers_to_and_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("true and false")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::And), "should end with And")?;
        Ok(())
    }

    #[test]
    fn or_operator_lowers_to_or_op() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("true or false")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Or), "should end with Or")?;
        Ok(())
    }

    #[test]
    fn chained_comparison_operators_lowers_with_precedence() -> Result<(), String> {
        // 1 < 2 and 3 > 0 or 4 >= 4
        // Precedence: comparison > and > or
        // => ((1 < 2) and (3 > 0)) or (4 >= 4)
        let (ops, _constants, _max_stack) = lower("1 < 2 and 3 > 0 or 4 >= 4")?;
        let last = ops.last().ok_or("missing last op")?;
        adv_ensure(matches!(last, ExprOp::Or), "root should be Or")?;
        // The ops before the Or should include Lt, And, Gt, Gte
        let has_lt = ops.iter().any(|op| matches!(op, ExprOp::Lt));
        let has_and = ops.iter().any(|op| matches!(op, ExprOp::And));
        let has_gt = ops.iter().any(|op| matches!(op, ExprOp::Gt));
        let has_gte = ops.iter().any(|op| matches!(op, ExprOp::Gte));
        adv_ensure(has_lt, "should contain Lt op")?;
        adv_ensure(has_and, "should contain And op")?;
        adv_ensure(has_gt, "should contain Gt op")?;
        adv_ensure(has_gte, "should contain Gte op")?;
        Ok(())
    }

    #[test]
    fn equality_and_inequality_left_associative() -> Result<(), String> {
        // == and != have same precedence (left-assoc)
        let (ops, _constants, _max_stack) = lower("1 == 2 != 3")?;
        // (1 == 2) != 3 => LoadConst, LoadConst, Eq, LoadConst, NotEq = 5 ops
        adv_ensure(ops.len() == 5, "should have 5 ops for chained equality")?;
        adv_ensure(
            matches!(ops.get(2), Some(ExprOp::Eq)),
            "Eq should be at index 2",
        )?;
        adv_ensure(
            matches!(ops.get(4), Some(ExprOp::NotEq)),
            "NotEq should be at index 4",
        )?;
        Ok(())
    }

    // 9. Helper arity validation for all helpers

    #[test]
    fn exists_with_one_arg_succeeds() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("exists(1)")?;
        adv_ensure(ops.len() == 2, "exists(1) should be load + Exists = 2 ops")?;
        adv_ensure(
            matches!(ops.last(), Some(ExprOp::Exists)),
            "should end with Exists op",
        )
    }

    #[test]
    fn exists_with_zero_args_rejected() -> Result<(), String> {
        let error = adv_lower_error("exists()")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "exists",
                    expected: 1,
                    actual: 0
                }
            ),
            "exists() should fail with arity mismatch",
        )
    }

    #[test]
    fn exists_with_two_args_rejected() -> Result<(), String> {
        let error = adv_lower_error("exists(1, 2)")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "exists",
                    expected: 1,
                    actual: 2
                }
            ),
            "exists(1, 2) should fail with arity mismatch",
        )
    }

    #[test]
    fn sum_with_one_arg_succeeds() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("sum(1)")?;
        adv_ensure(
            matches!(ops.last(), Some(ExprOp::Sum)),
            "should end with Sum op",
        )
    }

    #[test]
    fn sum_with_zero_args_rejected() -> Result<(), String> {
        let error = adv_lower_error("sum()")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "sum",
                    expected: 1,
                    actual: 0
                }
            ),
            "sum() should fail with arity mismatch",
        )
    }

    #[test]
    fn merge_with_two_args_succeeds() -> Result<(), String> {
        let (ops, _constants, _max_stack) = lower("merge(1, 2)")?;
        adv_ensure(
            matches!(ops.last(), Some(ExprOp::Merge)),
            "should end with Merge op",
        )
    }

