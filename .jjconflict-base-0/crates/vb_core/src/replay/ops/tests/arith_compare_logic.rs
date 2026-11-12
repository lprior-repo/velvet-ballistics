// ---- Arithmetic ops ----

#[test]
fn eval_add_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(7)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(17) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 + 7 should be 17".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_add_overflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MAX + i64::MAX must overflow"
    );
    Ok(())
}

#[test]
fn eval_sub_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(100))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(37)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(63) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "100 - 37 should be 63".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_sub_underflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MIN - 1 must underflow"
    );
    Ok(())
}

#[test]
fn eval_mul_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(6)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(7)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "6 * 7 should be 42".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_mul_overflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MAX * 2 must overflow"
    );
    Ok(())
}

#[test]
fn eval_div_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(6)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "42 / 6 should be 7".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_div_by_zero_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "division by zero must fail"
    );
    Ok(())
}

#[test]
fn eval_add_underflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(-1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MIN + (-1) must overflow"
    );
    Ok(())
}

#[test]
fn eval_sub_negative_result() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(-7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 - 10 should be -7".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_mul_zero() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(0) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX * 0 should be 0".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_div_negative() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(-10))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(-3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "-10 / 3 should be -3 (truncating toward zero)".into(),
        });
    }
    Ok(())
}

// ---- Comparison ops ----

#[test]
fn eval_gt_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 > 5 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_gt_false_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 > 5 should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_lt_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(8)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 < 8 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_lt_false_when_greater() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 < 5 should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_gte_true_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 >= 5 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_gte_false_when_less() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 >= 10 should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_lte_true_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 <= 5 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_lte_false_when_greater() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 <= 3 should be false".into(),
        });
    }
    Ok(())
}

// ---- Boolean ops ----

#[test]
fn eval_eq_same_i64() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "42 == 42 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_eq_different_types_returns_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(0) == Bool(false) should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_eq_null_null_returns_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "Null == Null should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_not_eq_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::NotEq,
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "1 != 2 should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_and_true_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "true && true should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_and_true_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "true && false should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_or_false_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "false || true should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_or_false_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "false || false should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_not_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "!true should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_not_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "!false should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_and_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "And with non-bool left must fail"
    );
    Ok(())
}

#[test]
fn eval_or_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Or with non-bool right must fail"
    );
    Ok(())
}

#[test]
fn eval_not_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Not on non-bool must fail"
    );
    Ok(())
}

