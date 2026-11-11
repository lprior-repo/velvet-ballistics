struct ContainsReplayCases {
    list: ListId,
    empty: ListId,
    singleton: ListId,
    symbol_list: ListId,
    float_list: ListId,
    first_symbol: SymbolId,
    second_symbol: SymbolId,
}

fn contains_replay_cases(store: &mut ValueStore) -> Result<ContainsReplayCases, CoreError> {
    let list = store.insert_list(vec![SlotValue::I64(7), SlotValue::I64(11)].into_boxed_slice())?;
    let empty = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    let singleton = store.insert_list(vec![SlotValue::I64(11)].into_boxed_slice())?;
    let first_symbol = store.insert_symbol("same")?;
    let second_symbol = store.insert_symbol("same")?;
    let symbol_list =
        store.insert_list(vec![SlotValue::Symbol(first_symbol)].into_boxed_slice())?;
    let float = FiniteF64::new(2.5)?;
    let float_list = store.insert_list(vec![SlotValue::F64(float)].into_boxed_slice())?;
    Ok(ContainsReplayCases {
        list,
        empty,
        singleton,
        symbol_list,
        float_list,
        first_symbol,
        second_symbol,
    })
}

fn contains_replay_at_step_one(
    store: &mut ValueStore,
    receiver: SlotValue,
    needle: SlotValue,
) -> Result<Result<SlotValue, ReplayError>, CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 0)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(receiver).map_err(replay_err_to_core)?;
    stack.push(needle).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    match eval_replay_op(&plan, &run, store, ExprOp::Contains, &mut stack, &mut taint) {
        Err(error) => Ok(Err(error)),
        Ok(()) => stack.pop().map(Ok).map_err(replay_err_to_core),
    }
}

#[test]
fn contains_replay_existing_i64_true() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.list), SlotValue::I64(11)]
        )?,
        SlotValue::Bool(true)
    );
    Ok(())
}

#[test]
fn contains_replay_absent_i64_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.list), SlotValue::I64(13)]
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}

#[test]
fn contains_replay_empty_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.empty), SlotValue::I64(7)]
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}

#[test]
fn contains_replay_other_list_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.singleton), SlotValue::I64(7)]
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}

#[test]
fn contains_replay_invalid_list_receiver_step_one() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let invalid = ListId::new(u32::MAX);
    assert_eq!(
        contains_replay_at_step_one(&mut store, SlotValue::List(invalid), SlotValue::I64(7))?,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn contains_replay_null_receiver_step_one() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    assert_eq!(
        contains_replay_at_step_one(&mut store, SlotValue::Null, SlotValue::I64(7))?,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn contains_replay_symbol_receiver_step_one() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("receiver")?;
    assert_eq!(
        contains_replay_at_step_one(&mut store, SlotValue::Symbol(symbol), SlotValue::I64(7))?,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn contains_replay_invalid_needle_handle_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[
                SlotValue::List(cases.list),
                SlotValue::List(ListId::new(u32::MAX))
            ],
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}

#[test]
fn contains_replay_equal_text_distinct_symbol_ids_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_ne!(cases.first_symbol, cases.second_symbol);
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[
                SlotValue::List(cases.symbol_list),
                SlotValue::Symbol(cases.second_symbol)
            ],
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}

#[test]
fn contains_replay_same_symbol_id_true() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[
                SlotValue::List(cases.symbol_list),
                SlotValue::Symbol(cases.first_symbol)
            ],
        )?,
        SlotValue::Bool(true)
    );
    Ok(())
}

#[test]
fn contains_replay_same_f64_tag_true() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    let needle = SlotValue::F64(FiniteF64::new(2.5)?);
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.float_list), needle]
        )?,
        SlotValue::Bool(true)
    );
    Ok(())
}

#[test]
fn contains_replay_i64_f64_cross_tag_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_replay_cases(&mut store)?;
    let needle = SlotValue::F64(FiniteF64::new(7.0)?);
    assert_eq!(
        eval_stack_helper(
            ExprOp::Contains,
            &mut store,
            &[SlotValue::List(cases.list), needle]
        )?,
        SlotValue::Bool(false)
    );
    Ok(())
}
