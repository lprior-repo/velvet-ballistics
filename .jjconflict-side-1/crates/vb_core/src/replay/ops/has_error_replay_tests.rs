#[test]
fn replay_has_wrong_receiver_with_valid_symbol_maps_failure_to_current_step()
-> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let key = insert_text(&mut store, "key")?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Symbol(key))
        .map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn replay_has_wrong_key_with_valid_object_maps_failure_to_current_step() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let object = store.insert_object(Vec::<crate::value_store::ObjectField>::new())?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Object(object))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn replay_has_both_operands_wrong_maps_failure_to_current_step() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn replay_has_invalid_object_with_valid_symbol_maps_failure_to_current_step()
-> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let key = insert_text(&mut store, "key")?;
    let bad_object = crate::ids::ObjectId::new(u32::MAX);
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Object(bad_object))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Symbol(key))
        .map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn replay_has_valid_empty_object_with_invalid_symbol_maps_failure_to_current_step()
-> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let object = store.insert_object(Vec::<crate::value_store::ObjectField>::new())?;
    let bad_symbol = crate::ids::SymbolId::new(u32::MAX);
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Object(object))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Symbol(bad_symbol))
        .map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn replay_has_both_typed_handles_invalid_maps_failure_to_current_step() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut store = ValueStore::new();
    let bad_object = crate::ids::ObjectId::new(u32::MAX);
    let bad_symbol = crate::ids::SymbolId::new(u32::MAX);
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Object(bad_object))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Symbol(bad_symbol))
        .map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}
