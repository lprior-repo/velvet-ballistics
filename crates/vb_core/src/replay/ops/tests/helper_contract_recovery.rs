fn helper_contract_results(
    op: ExprOp,
    store: &mut ValueStore,
    operands: &[(SlotValue, Taint)],
) -> Result<
    (
        Result<(SlotValue, Taint), crate::EngineError>,
        Result<(SlotValue, Taint), ReplayError>,
    ),
    CoreError,
> {
    let mut ops: Vec<_> = operands
        .iter()
        .enumerate()
        .map(|(index, _)| helper_contract_slot(index).map(ExprOp::LoadSlot))
        .collect::<Result<Vec<_>, CoreError>>()?;
    ops.push(op);
    let expression = make_expr_program(ops)?;
    let plan = make_plan(
        vec![
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        ],
        vec![],
        vec![expression],
        vec![],
    )?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 8)?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    for (index, (value, taint)) in operands.iter().enumerate() {
        run.write_slot_with_taint(helper_contract_slot(index)?, *value, *taint)?;
        stack
            .push_tainted(*value, *taint)
            .map_err(replay_err_to_core)?;
    }
    let before = store.checkpoint_v1();
    let forward = crate::engine::eval_expr_with_store(&plan, &run, store, ExprIdx::new(0));
    let mut replay_taint = Taint::Clean;
    let replay = eval_replay_op(&plan, &run, store, op, &mut stack, &mut replay_taint)
        .and_then(|()| stack.pop_tainted());
    if let Ok((_, taint)) = &replay {
        assert_eq!(*taint, replay_taint);
    }
    assert_eq!(store.checkpoint_v1(), before);
    Ok((forward, replay))
}

fn helper_contract_slot(index: usize) -> Result<SlotIdx, CoreError> {
    u16::try_from(index)
        .map(SlotIdx::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "helper test operand slot",
        })
}

#[test]
fn recovered_has_object_keys_match_forward_values_and_operand_taints() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let present = store.insert_symbol("present")?;
    let missing = store.insert_symbol("missing")?;
    let object = store.insert_object(Box::from([crate::value_store::ObjectField::with_taint(
        present,
        SlotValue::I64(9),
        Taint::Secret,
    )]))?;
    for (key, expected) in [(present, true), (missing, false)] {
        for operand_taint in [Taint::Clean, Taint::Secret, Taint::Random] {
            let operands = [
                (SlotValue::Object(object), operand_taint),
                (SlotValue::Symbol(key), Taint::Clean),
            ];
            let (forward, replay) = helper_contract_results(ExprOp::Has, &mut store, &operands)?;
            assert_eq!(forward, Ok((SlotValue::Bool(expected), operand_taint)));
            assert_eq!(replay, Ok((SlotValue::Bool(expected), operand_taint)));
        }
    }
    Ok(())
}

#[test]
fn recovered_has_rejects_dangling_key_object_and_wrong_list_operand() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("key")?;
    let object = store.insert_object(Vec::new().into_boxed_slice())?;
    let list = store.insert_list(Box::from([SlotValue::Symbol(key)]))?;
    let invalid_key = SymbolId::new(u32::MAX);
    let invalid_object = crate::ObjectId::new(u32::MAX);
    for (left, right, expected_forward) in [
        (
            SlotValue::Object(object),
            SlotValue::Symbol(invalid_key),
            crate::EngineError::SymbolOutOfBounds {
                symbol: invalid_key,
            },
        ),
        (
            SlotValue::Object(invalid_object),
            SlotValue::Symbol(key),
            crate::EngineError::ObjectOutOfBounds {
                object: invalid_object,
            },
        ),
        (
            SlotValue::List(list),
            SlotValue::Symbol(key),
            crate::EngineError::TypeMismatch {
                expected: "object",
                found: "list",
            },
        ),
        (
            SlotValue::Object(object),
            SlotValue::I64(1),
            crate::EngineError::TypeMismatch {
                expected: "text",
                found: "number",
            },
        ),
    ] {
        let (forward, replay) = helper_contract_results(
            ExprOp::Has,
            &mut store,
            &[(left, Taint::Clean), (right, Taint::Clean)],
        )?;
        assert_eq!(forward, Err(expected_forward));
        assert_eq!(
            replay,
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::new(1)
            })
        );
    }
    Ok(())
}

#[test]
fn recovered_exists_matches_forward_for_every_valid_value_tag() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("")?;
    let list = store.insert_list(Vec::new().into_boxed_slice())?;
    let object = store.insert_object(Vec::new().into_boxed_slice())?;
    let blob = store.insert_blob(Vec::new())?;
    for value in [
        SlotValue::Null,
        SlotValue::Bool(false),
        SlotValue::I64(0),
        SlotValue::F64(FiniteF64::new(0.0)?),
        SlotValue::Symbol(symbol),
        SlotValue::List(list),
        SlotValue::Object(object),
        SlotValue::Blob(blob),
    ] {
        let expected = !matches!(value, SlotValue::Null);
        let (forward, replay) =
            helper_contract_results(ExprOp::Exists, &mut store, &[(value, Taint::Secret)])?;
        assert_eq!(forward, Ok((SlotValue::Bool(expected), Taint::Secret)));
        assert_eq!(replay, Ok((SlotValue::Bool(expected), Taint::Secret)));
    }
    Ok(())
}

#[test]
fn recovered_length_matches_forward_null_text_list_and_object() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = store.insert_symbol("hello")?;
    let list = store.insert_list(Box::from([SlotValue::Bool(true), SlotValue::Null]))?;
    let object = store.insert_object(Box::from([crate::value_store::ObjectField::clean(
        text,
        SlotValue::Bool(true),
    )]))?;
    for (value, expected) in [
        (SlotValue::Null, 0),
        (SlotValue::Symbol(text), 5),
        (SlotValue::List(list), 2),
        (SlotValue::Object(object), 1),
    ] {
        let (forward, replay) =
            helper_contract_results(ExprOp::Length, &mut store, &[(value, Taint::Secret)])?;
        assert_eq!(forward, Ok((SlotValue::I64(expected), Taint::Secret)));
        assert_eq!(replay, Ok((SlotValue::I64(expected), Taint::Secret)));
    }
    Ok(())
}

#[test]
fn recovered_length_keeps_scalar_and_dangling_handle_errors() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let symbol = SymbolId::new(u32::MAX);
    let list = ListId::new(u32::MAX);
    let object = crate::ObjectId::new(u32::MAX);
    for (value, expected_forward) in [
        (
            SlotValue::I64(1),
            crate::EngineError::TypeMismatch {
                expected: "text, list, or object",
                found: "number",
            },
        ),
        (
            SlotValue::Symbol(symbol),
            crate::EngineError::SymbolOutOfBounds { symbol },
        ),
        (
            SlotValue::List(list),
            crate::EngineError::ListOutOfBounds { list },
        ),
        (
            SlotValue::Object(object),
            crate::EngineError::ObjectOutOfBounds { object },
        ),
    ] {
        let (forward, replay) =
            helper_contract_results(ExprOp::Length, &mut store, &[(value, Taint::Clean)])?;
        assert_eq!(forward, Err(expected_forward));
        assert_eq!(
            replay,
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::new(1)
            })
        );
    }
    Ok(())
}
