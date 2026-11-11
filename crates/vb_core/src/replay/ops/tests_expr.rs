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
            reason: "loaded slot should be I64(99)",
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
            reason: "taint should be Secret after loading tainted slot",
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
            reason: "loaded const should be I64(77)",
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
            reason: "loaded const should be Bool(true)",
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
            reason: "loaded const should be Null",
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
            reason: "accessor should resolve object field to I64(42)",
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
            reason: "accessor should resolve list[1] to I64(20)",
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
            reason: "empty accessor path should return root value",
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
            reason: "15 + 27 should be 42",
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
            reason: "(10 > 5) && (3 < 8) should be true",
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

// =========================================================================
// BLACKHAT security regression tests -- replay expression evaluation
// =========================================================================

// --- FINDING BH-OPS-01: Division of i64::MIN by -1 must overflow ---
//
// checked_div correctly returns None for i64::MIN / -1, but we verify
// the replay path handles this case.

#[test]
fn blackhat_eval_div_min_by_neg_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(-1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-01: i64::MIN / -1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-02: Subtraction of i64::MIN by 1 must underflow ---

#[test]
fn blackhat_eval_sub_min_by_one_returns_error() -> Result<(), CoreError> {
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
        "BLACKHAT BH-OPS-02: i64::MIN - 1 must underflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-03: Addition of i64::MAX + 1 must overflow ---

#[test]
fn blackhat_eval_add_max_plus_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-03: i64::MAX + 1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-04: Multiplication overflow at boundary values ---

#[test]
fn blackhat_eval_mul_boundary_overflow() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-04: i64::MIN * 2 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-05: Taint joins across LoadSlot operations ---
//
// When two slots are loaded (one Clean, one Secret) and added,
// the accumulated taint must be Secret.

#[test]
fn blackhat_taint_joins_across_expression() -> Result<(), CoreError> {
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
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)?;
    let mut store = ValueStore::new();

    let (value, taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    assert_eq!(
        value,
        SlotValue::I64(30),
        "BLACKHAT BH-OPS-05: value must be 30"
    );
    assert_eq!(
        taint,
        Taint::Secret,
        "BLACKHAT BH-OPS-05: taint must be Secret when one operand is Secret"
    );
    Ok(())
}

// --- FINDING BH-OPS-06: Expression stack overflow is caught ---
//
// Pushing more than capacity allows must fail, not corrupt memory.

#[test]
fn blackhat_expression_stack_overflow_is_safe() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(2).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = stack.push(SlotValue::I64(3));
    assert!(
        result.is_err(),
        "BLACKHAT BH-OPS-06: stack overflow must return error, not corrupt"
    );

    // Verify the stack is still consistent after the failed push
    let first = stack.pop().map_err(replay_err_to_core)?;
    assert_eq!(
        first,
        SlotValue::I64(2),
        "BLACKHAT BH-OPS-06: stack must remain consistent after overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-07: Helper ops evaluate like the engine (vb-cofe1) ---
//
// Replay previously rejected every helper opcode. The bead extends replay to
// cover them with engine-matching semantics, so these tests now pin the
// supported behavior. Wrong operand types must still fail evaluation
// (ExpressionEvalFailed), never panic and never surface as Internal.

fn eval_stack_helper(
    op: ExprOp,
    store: &mut ValueStore,
    values: &[SlotValue],
) -> Result<SlotValue, CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    eval_replay_op(&plan, &run, store, op, &mut stack, &mut taint).map_err(replay_err_to_core)?;
    stack.pop().map_err(replay_err_to_core)
}

fn check_helper_type_error(op: ExprOp, values: &[SlotValue]) -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    let result = eval_replay_op(&plan, &run, &mut store, op, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-07: {op:?} on wrong types must fail evaluation, not panic"
    );
    Ok(())
}

fn insert_text(store: &mut ValueStore, text: &str) -> Result<SymbolId, CoreError> {
    store.insert_symbol(text)
}

fn insert_numbers(store: &mut ValueStore, values: &[i64]) -> Result<ListId, CoreError> {
    let items: Vec<SlotValue> = values.iter().map(|&v| SlotValue::I64(v)).collect();
    store.insert_list(items.into_boxed_slice())
}

#[test]
fn blackhat_helper_contains_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(7), SlotValue::I64(11)].into_boxed_slice(),
    )?;
    let found = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(7)],
    )?;
    if found != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should find list member",
        });
    }
    let absent = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(13)],
    )?;
    if absent != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should miss absent list member",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_starts_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let prefix = insert_text(&mut store, "hello")?;
    let result = eval_stack_helper(
        ExprOp::StartsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(prefix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: starts_with should match prefix",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_ends_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let suffix = insert_text(&mut store, "world")?;
    let result = eval_stack_helper(
        ExprOp::EndsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(suffix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: ends_with should match suffix",
        });
    }
    Ok(())
}

fn insert_has_object(
    store: &mut ValueStore,
    key: SymbolId,
) -> Result<crate::ids::ObjectId, CoreError> {
    store
        .insert_object(vec![crate::ObjectField::clean(key, SlotValue::I64(1))].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })
}

fn assert_has_result(
    store: &mut ValueStore,
    object: crate::ids::ObjectId,
    symbol: SymbolId,
    expected: bool,
) -> Result<(), CoreError> {
    let result = eval_stack_helper(
        ExprOp::Has,
        store,
        &[SlotValue::Object(object), SlotValue::Symbol(symbol)],
    )?;
    assert_eq!(
        result,
        SlotValue::Bool(expected),
        "BLACKHAT BH-OPS-07: has membership result"
    );
    Ok(())
}

#[test]
fn blackhat_helper_has_matches_engine_list_membership() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let present_symbol = insert_text(&mut store, "present")?;
    let absent_symbol = insert_text(&mut store, "absent")?;
    let object = insert_has_object(&mut store, present_symbol)?;
    assert_has_result(&mut store, object, present_symbol, true)?;
    assert_has_result(&mut store, object, absent_symbol, false)?;
    Ok(())
}

#[test]
fn blackhat_helper_exists_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key = insert_text(&mut store, "k")?;
    let object = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let present = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Object(object)])?;
    if present != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should hold for non-empty object",
        });
    }
    let null = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Null])?;
    if null != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should fail for null",
        });
    }
    check_helper_type_error(ExprOp::Exists, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_length_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello")?;
    let text_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::Symbol(text)])?;
    if text_len != SlotValue::I64(5) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of text should be 5",
        });
    }
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let list_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::List(list)])?;
    if list_len != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of list should be 3",
        });
    }
    check_helper_type_error(ExprOp::Length, &[SlotValue::Bool(true)])
}

#[test]
fn blackhat_helper_empty_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let null_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::Null])?;
    if null_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty null should be true",
        });
    }
    let empty_list = insert_numbers(&mut store, &[])?;
    let list_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(empty_list)])?;
    if list_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty list should be true",
        });
    }
    let full_list = insert_numbers(&mut store, &[1])?;
    let list_full = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(full_list)])?;
    if list_full != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: non-empty list should be false",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_append_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2])?;
    let result = eval_stack_helper(
        ExprOp::Append,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(3)],
    )?;
    let appended_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append must return a list",
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "appended list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append should extend the list",
        });
    }
    check_helper_type_error(ExprOp::Append, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_append_if_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1])?;
    let appended = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(true),
        ],
    )?;
    let appended_id = match appended {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list",
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if true should extend the list",
        });
    }
    let kept = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(false),
        ],
    )?;
    let kept_id = match kept {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list",
            });
        }
    };
    let kept_items = store
        .list(kept_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve",
        })?;
    if kept_items != [SlotValue::I64(1)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if false should copy the list",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_merge_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key_a = insert_text(&mut store, "a")?;
    let key_b = insert_text(&mut store, "b")?;
    let left = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_a,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let right = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_b,
                SlotValue::I64(2),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let result = eval_stack_helper(
        ExprOp::Merge,
        &mut store,
        &[SlotValue::Object(left), SlotValue::Object(right)],
    )?;
    let merged_id = match result {
        SlotValue::Object(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: merge must return an object",
            });
        }
    };
    let fields = store
        .object(merged_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "merged object must resolve",
        })?;
    if fields.len() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: merge should combine fields",
        });
    }
    check_helper_type_error(ExprOp::Merge, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_sum_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Sum, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(6) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: sum should total list items",
        });
    }
    check_helper_type_error(ExprOp::Sum, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_count_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Count, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: count should report list length",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 1, 3])?;
    let result = eval_stack_helper(ExprOp::Unique, &mut store, &[SlotValue::List(list)])?;
    let unique_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: unique must return a list",
            });
        }
    };
    let items = store
        .list(unique_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "unique list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: unique should deduplicate preserving order",
        });
    }
    Ok(())
}
