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
