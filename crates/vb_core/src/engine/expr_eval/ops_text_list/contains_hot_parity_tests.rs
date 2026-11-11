struct ContainsHotCases {
    list: ListId,
    empty: ListId,
    singleton: ListId,
    symbol_list: ListId,
    float_list: ListId,
    first_symbol: SymbolId,
    second_symbol: SymbolId,
}

fn contains_hot_cases(
    store: &mut ValueStore,
) -> Result<ContainsHotCases, crate::errors::CoreError> {
    let list = store.insert_list(vec![SlotValue::I64(7), SlotValue::I64(11)].into_boxed_slice())?;
    let empty = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    let singleton = store.insert_list(vec![SlotValue::I64(11)].into_boxed_slice())?;
    let first_symbol = store.insert_symbol("same")?;
    let second_symbol = store.insert_symbol("same")?;
    let symbol_list =
        store.insert_list(vec![SlotValue::Symbol(first_symbol)].into_boxed_slice())?;
    let float = crate::value::FiniteF64::new(2.5)?;
    let float_list = store.insert_list(vec![SlotValue::F64(float)].into_boxed_slice())?;
    Ok(ContainsHotCases {
        list,
        empty,
        singleton,
        symbol_list,
        float_list,
        first_symbol,
        second_symbol,
    })
}

fn contains_hot_program() -> Result<ExprProgram, crate::errors::CoreError> {
    ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Contains,
        ]
        .into_boxed_slice(),
    )
}

fn contains_hot_nodes() -> Box<[crate::workflow::CompiledNode]> {
    vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(0)),
        None,
    )]
    .into_boxed_slice()
}

fn contains_hot_plan() -> Result<CompiledWorkflow, crate::errors::CoreError> {
    let expression = contains_hot_program()?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "contains_hot_test".into(),
        digest: WorkflowDigest::from_bytes([0xA5; 32]),
        nodes: contains_hot_nodes(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| crate::errors::CoreError::InvalidCompiledWorkflow {
        reason: "Contains hot test workflow validation failed",
    })
}

fn contains_hot_eval(
    store: &mut ValueStore,
    receiver: SlotValue,
    needle: SlotValue,
) -> Result<Result<(SlotValue, Taint), EngineError>, crate::errors::CoreError> {
    let workflow = contains_hot_plan()?;
    let mut run = crate::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2)?;
    run.write_slot(SlotIdx::new(0), receiver)?;
    run.write_slot(SlotIdx::new(1), needle)?;
    Ok(crate::engine::expr_eval::eval_expr_with_store(
        &workflow,
        &run,
        store,
        ExprIdx::new(0),
    ))
}

#[test]
fn contains_hot_existing_i64_true() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(cases.list), SlotValue::I64(11))?,
        Ok((SlotValue::Bool(true), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_absent_i64_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(cases.list), SlotValue::I64(13))?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_empty_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(cases.empty), SlotValue::I64(7))?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_other_list_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(
            &mut store,
            SlotValue::List(cases.singleton),
            SlotValue::I64(7)
        )?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_invalid_list_receiver_error() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let invalid = ListId::new(u32::MAX);
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(invalid), SlotValue::I64(7))?,
        Err(EngineError::ListOutOfBounds { list: invalid })
    );
    Ok(())
}

#[test]
fn contains_hot_null_receiver_error() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::Null, SlotValue::I64(7))?,
        Err(EngineError::TypeMismatch {
            expected: "list",
            found: "null"
        })
    );
    Ok(())
}

#[test]
fn contains_hot_symbol_receiver_error() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("receiver")?;
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::Symbol(symbol), SlotValue::I64(7))?,
        Err(EngineError::TypeMismatch {
            expected: "list",
            found: "symbol"
        })
    );
    Ok(())
}

#[test]
fn contains_hot_invalid_needle_handle_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(
            &mut store,
            SlotValue::List(cases.list),
            SlotValue::List(ListId::new(u32::MAX)),
        )?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_equal_text_distinct_symbol_ids_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_ne!(cases.first_symbol, cases.second_symbol);
    assert_eq!(
        contains_hot_eval(
            &mut store,
            SlotValue::List(cases.symbol_list),
            SlotValue::Symbol(cases.second_symbol),
        )?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_same_symbol_id_true() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    assert_eq!(
        contains_hot_eval(
            &mut store,
            SlotValue::List(cases.symbol_list),
            SlotValue::Symbol(cases.first_symbol),
        )?,
        Ok((SlotValue::Bool(true), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_same_f64_tag_true() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    let needle = SlotValue::F64(crate::value::FiniteF64::new(2.5)?);
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(cases.float_list), needle)?,
        Ok((SlotValue::Bool(true), Taint::Clean))
    );
    Ok(())
}

#[test]
fn contains_hot_i64_f64_cross_tag_false() -> Result<(), crate::errors::CoreError> {
    let mut store = ValueStore::new();
    let cases = contains_hot_cases(&mut store)?;
    let needle = SlotValue::F64(crate::value::FiniteF64::new(7.0)?);
    assert_eq!(
        contains_hot_eval(&mut store, SlotValue::List(cases.list), needle)?,
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    Ok(())
}
