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

#[test]
fn merge_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("merge(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "merge",
                expected: 2,
                actual: 1
            }
        ),
        "merge(1) should fail with arity mismatch",
    )
}

#[test]
fn merge_with_three_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("merge(1, 2, 3)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "merge",
                expected: 2,
                actual: 3
            }
        ),
        "merge(1, 2, 3) should fail with arity mismatch",
    )
}

#[test]
fn append_if_with_three_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("append_if(1, 2, 3)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::AppendIf)),
        "should end with AppendIf op",
    )
}

#[test]
fn append_if_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("append_if(1, 2)")?;
    // This is already tested above, but verifying with the adv_ensure pattern
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append_if",
                expected: 3,
                actual: 2
            }
        ),
        "append_if(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn append_if_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("append_if(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append_if",
                expected: 3,
                actual: 1
            }
        ),
        "append_if(1) should fail with arity mismatch",
    )
}

#[test]
fn contains_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("contains(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Contains)),
        "should end with Contains op",
    )
}

#[test]
fn contains_with_three_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("contains(1, 2, 3)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "contains",
                expected: 2,
                actual: 3
            }
        ),
        "contains(1, 2, 3) should fail with arity mismatch",
    )
}

#[test]
fn length_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("length(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Length)),
        "should end with Length op",
    )
}

#[test]
fn length_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("length(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "length",
                expected: 1,
                actual: 2
            }
        ),
        "length(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn unique_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("unique(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Unique)),
        "should end with Unique op",
    )
}

#[test]
fn unique_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("unique(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "unique",
                expected: 1,
                actual: 2
            }
        ),
        "unique(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn count_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("count(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Count)),
        "should end with Count op",
    )
}

#[test]
fn count_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("count()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "count",
                expected: 1,
                actual: 0
            }
        ),
        "count() should fail with arity mismatch",
    )
}

#[test]
fn empty_helper_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("empty(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Empty)),
        "should end with Empty op",
    )
}

#[test]
fn empty_helper_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("empty()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "empty",
                expected: 1,
                actual: 0
            }
        ),
        "empty() should fail with arity mismatch",
    )
}

#[test]
fn empty_helper_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("empty(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "empty",
                expected: 1,
                actual: 2
            }
        ),
        "empty(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn append_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("append(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Append)),
        "should end with Append op",
    )
}

#[test]
fn append_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("append(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append",
                expected: 2,
                actual: 1
            }
        ),
        "append(1) should fail with arity mismatch",
    )
}

#[test]
fn starts_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("starts_with(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::StartsWith)),
        "should end with StartsWith op",
    )
}

#[test]
fn starts_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("starts_with(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "starts_with",
                expected: 2,
                actual: 1
            }
        ),
        "starts_with(1) should fail with arity mismatch",
    )
}

#[test]
fn ends_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("ends_with(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::EndsWith)),
        "should end with EndsWith op",
    )
}

#[test]
fn has_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("has(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Has)),
        "should end with Has op",
    )
}

#[test]
fn has_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("has(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "has",
                expected: 2,
                actual: 1
            }
        ),
        "has(1) should fail with arity mismatch",
    )
}

// Additional edge-case: helper with nested expression argument

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

