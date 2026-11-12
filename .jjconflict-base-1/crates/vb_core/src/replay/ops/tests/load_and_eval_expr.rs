// ---- LoadSlot / LoadConst ----

#[test]
fn eval_load_slot_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded slot should be I64(99)".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_slot_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(50)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::SlotNotAvailable { .. })),
        "loading out-of-bounds slot must fail"
    );
    Ok(())
}

#[test]
fn eval_load_slot_propagates_taint() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "taint should be Secret after loading tainted slot".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_i64() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::I64(77)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(77) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be I64(77)".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::Bool(true)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be Bool(true)".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_null() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::Null])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Null {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be Null".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::I64(1)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(5)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "loading out-of-bounds constant must fail"
    );
    Ok(())
}

// ---- LoadAccessor ----

#[test]
fn eval_load_accessor_object_field() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::clean(
        field_sym,
        SlotValue::I64(42),
    )];
    let obj_handle = store.insert_object(fields.into_boxed_slice())?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj_handle))?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accessor should resolve object field to I64(42)".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_list_index() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let items = vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)];
    let list_handle = store.insert_list(items.into_boxed_slice())?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(1)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list_handle))?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accessor should resolve list[1] to I64(20)".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_empty_path_returns_root() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "empty accessor path should return root value".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(99)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "out-of-bounds accessor must fail"
    );
    Ok(())
}

// ---- Helper ops with wrong operand types ----

#[test]
fn eval_helper_op_with_wrong_types_returns_eval_failed() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::Contains,
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Contains on non-symbols must fail evaluation, not panic"
    );
    Ok(())
}

// ---- eval_expr_for_replay integration ----

#[test]
fn eval_expr_for_replay_addition() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(15))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(27))?;
    let mut store = ValueStore::new();

    let (value, _taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    if value != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "15 + 27 should be 42".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_expr_for_replay_comparison_chain() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::LoadConst(ConstIdx::new(3)),
        ExprOp::Lt,
        ExprOp::And,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![
            ConstValue::I64(10),
            ConstValue::I64(5),
            ConstValue::I64(3),
            ConstValue::I64(8),
        ],
        vec![expr],
        vec![],
    )?;

    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let (value, _taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    if value != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "(10 > 5) && (3 < 8) should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn eval_expr_for_replay_with_uninitialized_slot() -> Result<(), CoreError> {
    // Expression loads from a slot that was never written
    let expr = make_expr_program(vec![ExprOp::LoadSlot(SlotIdx::new(0))])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    // Slot 0 is never written -- read should fail
    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let result = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0));
    assert!(
        result.is_err(),
        "expression loading uninitialized slot must fail"
    );
    Ok(())
}

#[test]
fn eval_expr_for_replay_out_of_bounds_expr() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let result = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(99));
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "out-of-bounds expression must fail"
    );
    Ok(())
}

