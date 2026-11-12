// ---- ReplayExprStack lifecycle ----

#[test]
fn expr_stack_push_pop_roundtrip() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(3).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;

    let second = stack.pop().map_err(replay_err_to_core)?;
    let first = stack.pop().map_err(replay_err_to_core)?;

    if second != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "second pop should be Bool(true)".into(),
        });
    }
    if first != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "first pop should be I64(42)".into(),
        });
    }
    Ok(())
}

#[test]
fn expr_stack_pop_empty_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let result = stack.pop();
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "popping from empty stack must fail with ExpressionEvalFailed"
    );
    Ok(())
}

#[test]
fn expr_stack_push_overflow_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(1).map_err(replay_err_to_core)?;
    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    let overflow_result = stack.push(SlotValue::Null);
    assert!(
        matches!(overflow_result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "push overflow must fail with ExpressionEvalFailed"
    );
    Ok(())
}

#[test]
fn expr_stack_max_capacity_boundary() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(MAX_EXPRESSION_STACK).map_err(replay_err_to_core)?;
    let values: [i64; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    ];
    values
        .iter()
        .try_for_each(|&i| stack.push(SlotValue::I64(i)).map_err(replay_err_to_core))?;
    let overflow_result = stack.push(SlotValue::Null);
    assert!(
        matches!(overflow_result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "push beyond capacity must fail with ExpressionEvalFailed"
    );
    Ok(())
}

// ---- pop_pair / pop_i64_pair ----

#[test]
fn pop_pair_returns_left_then_right() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(20)).map_err(replay_err_to_core)?;

    let (left, right) = pop_pair(&mut stack).map_err(replay_err_to_core)?;
    if left != SlotValue::I64(10) || right != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "pop_pair order wrong".into(),
        });
    }
    Ok(())
}

#[test]
fn pop_i64_pair_succeeds_for_i64_values() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;

    let (left, right) = pop_i64_pair(&mut stack).map_err(replay_err_to_core)?;
    if left != 5 || right != 3 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "pop_i64_pair values wrong".into(),
        });
    }
    Ok(())
}

#[test]
fn pop_i64_pair_rejects_non_i64_left() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;

    let result = pop_i64_pair(&mut stack);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "non-i64 left operand must fail"
    );
    Ok(())
}

#[test]
fn pop_i64_pair_rejects_non_i64_right() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;

    let result = pop_i64_pair(&mut stack);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "non-i64 right operand must fail"
    );
    Ok(())
}

#[test]
fn pop_pair_underflow_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = pop_pair(&mut stack);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO
        })
    );
    Ok(())
}

