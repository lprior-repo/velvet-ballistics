// =============================================================================
// B-001 to B-007: join_taint Lattice Algebra
// =============================================================================

// B-001: join_taint returns Secret when either input is Secret
#[test]
fn join_taint_returns_secret_when_either_input_is_secret() {
    let result = join_taint(Taint::Clean, Taint::Secret);
    assert_eq!(result, Taint::Secret);

    let result = join_taint(Taint::Secret, Taint::Clean);
    assert_eq!(result, Taint::Secret);

    let result = join_taint(Taint::Secret, Taint::Secret);
    assert_eq!(result, Taint::Secret);

    let result = join_taint(Taint::DerivedFromSecret, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

// B-002: join_taint returns DerivedFromSecret when neither is Secret
#[test]
fn join_taint_returns_derived_from_secret_when_neither_is_secret() {
    let result = join_taint(Taint::DerivedFromSecret, Taint::Clean);
    assert_eq!(result, Taint::DerivedFromSecret);

    let result = join_taint(Taint::Clean, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);

    let result = join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

// B-003: join_taint returns Clean when both inputs are Clean
#[test]
fn join_taint_returns_clean_when_both_inputs_are_clean() {
    let result = join_taint(Taint::Clean, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

// B-004: join_taint is commutative
#[test]
fn join_taint_is_commutative_for_all_taint_pairs() {
    let pairs = [
        (Taint::Clean, Taint::Clean),
        (Taint::Clean, Taint::DerivedFromSecret),
        (Taint::Clean, Taint::Secret),
        (Taint::DerivedFromSecret, Taint::DerivedFromSecret),
        (Taint::DerivedFromSecret, Taint::Secret),
        (Taint::Secret, Taint::Secret),
    ];

    for (a, b) in pairs {
        let ab = join_taint(a, b);
        let ba = join_taint(b, a);
        assert_eq!(
            ab, ba,
            "join_taint({a:?}, {b:?}) != join_taint({b:?}, {a:?})"
        );
    }
}

// B-005: join_taint is associative
#[test]
fn join_taint_is_associative_for_all_taint_triples() {
    let triples = [
        (Taint::Clean, Taint::Clean, Taint::Clean),
        (Taint::Clean, Taint::Clean, Taint::DerivedFromSecret),
        (Taint::Clean, Taint::DerivedFromSecret, Taint::Secret),
        (Taint::Secret, Taint::DerivedFromSecret, Taint::Clean),
    ];

    for (a, b, c) in triples {
        let ab_c = join_taint(join_taint(a, b), c);
        let a_bc = join_taint(a, join_taint(b, c));
        assert_eq!(
            ab_c, a_bc,
            "join_taint(join_taint({a:?}, {b:?}), {c:?}) != join_taint({a:?}, join_taint({b:?}, {c:?}))"
        );
    }
}

// B-006: join_taint has Secret as lattice top
#[test]
fn join_taint_returns_secret_for_all_inputs_when_first_is_secret() {
    for b in [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
        let result = join_taint(Taint::Secret, b);
        assert_eq!(result, Taint::Secret);
    }
}

// B-007: join_taint has Clean as lattice bottom
#[test]
fn join_taint_returns_second_arg_when_first_is_clean() {
    for b in [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
        let result = join_taint(Taint::Clean, b);
        assert_eq!(result, b);
    }
}

// =============================================================================
// B-010 to B-020: Frame Slot Operations
// =============================================================================

// B-010: RunFrame::new creates frame with all slots uninitialized and taint Clean
#[test]
fn runframe_new_creates_uninitialized_slots_with_clean_taint() -> Result<(), String> {
    let run = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4).map_err(|e| e.to_string())?;

    // All slots should be uninitialized
    for i in 0..4 {
        let slot = SlotIdx::new(i);
        match run.read_slot(slot) {
            Err(EngineError::SlotUninitialized { slot: s }) if s == slot => {}
            other => return Err(format!("slot {i} should be uninitialized, got: {other:?}")),
        }
        match run.read_taint(slot) {
            Err(EngineError::SlotUninitialized { slot: s }) if s == slot => {}
            other => return Err(format!("taint {i} should be uninitialized, got: {other:?}")),
        }
    }
    Ok(())
}

// B-011: write_slot_with_taint atomically writes both arrays
#[test]
fn write_slot_with_taint_atomically_updates_both_slots_and_taint_arrays() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;

    // Verify value
    let value = run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(*value, SlotValue::I64(42))?;

    // Verify taint
    let taint = run.read_taint(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)
}

// B-018: After write_slot_with_taint, read_taint returns exactly taint
#[test]
fn write_slot_with_taint_then_read_taint_returns_exact_taint() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(99), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

// B-019: After write_slot_with_taint, read_slot returns exactly value
#[test]
fn write_slot_with_taint_then_read_slot_returns_exact_value() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::Bool(true), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let value = run.read_slot(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(*value, SlotValue::Bool(true))
}

// B-020: reinitialize resets all slots to uninitialized and taint to Clean
#[test]
fn reinitialize_resets_all_slots_and_taint_to_clean() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;

    run.reinitialize(RunId::new(2), StepIdx::new(0), 2, 2)
        .map_err(|e| e.to_string())?;

    // All slots should be uninitialized after reinitialize
    for i in 0..2 {
        let slot = SlotIdx::new(i);
        match run.read_slot(slot) {
            Err(EngineError::SlotUninitialized { slot: s }) if s == slot => {}
            other => {
                return Err(format!(
                    "slot {i} should be uninitialized after reinit, got: {other:?}"
                ));
            }
        }
        match run.read_taint(slot) {
            Err(EngineError::SlotUninitialized { slot: s }) if s == slot => {}
            other => {
                return Err(format!(
                    "taint {i} should be uninitialized after reinit, got: {other:?}"
                ));
            }
        }
    }
    Ok(())
}

// =============================================================================
// B-030 to B-038: EvalExpr Taint Propagation (POST-001)
// =============================================================================

fn eval_workflow_with_slots(
    ops: Box<[ExprOp]>,
    slots: Vec<(SlotValue, Taint)>,
    constants: Vec<ConstValue>,
) -> Result<(SlotValue, Taint), EngineError> {
    let max_stack =
        crate::workflow::check_expr_stack_bound(&ops, crate::limits::MAX_EXPRESSION_STACK)
            .map_err(|_| EngineError::InvalidCompiledWorkflow {
                reason: "stack check failed",
            })?;
    let expr = ExprProgram::try_from_parts(ops, max_stack).map_err(|_| {
        EngineError::InvalidCompiledWorkflow {
            reason: "expr parts",
        }
    })?;

    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "taint_eval_test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "workflow parts",
    })?;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 8)?;
    for (i, (value, taint)) in slots.iter().enumerate() {
        run.write_slot_with_taint(SlotIdx::new(i as u16), *value, *taint)?;
    }

    let mut store = ValueStore::new();
    eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
}

// B-030: eval_expr_with_store returns Clean when all loaded slots are Clean
#[test]
fn eval_expr_returns_clean_taint_when_all_slots_clean() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]
    .into_boxed_slice();

    let slots = vec![
        (SlotValue::I64(10), Taint::Clean),
        (SlotValue::I64(20), Taint::Clean),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Clean)
}

// B-031: eval_expr_with_store returns DerivedFromSecret when any slot is DerivedFromSecret
#[test]
fn eval_expr_returns_derived_from_secret_when_any_slot_has_that_taint() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]
    .into_boxed_slice();

    let slots = vec![
        (SlotValue::I64(10), Taint::Clean),
        (SlotValue::I64(20), Taint::DerivedFromSecret),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::DerivedFromSecret)
}

// B-032: eval_expr_with_store returns Secret when any loaded slot is Secret
#[test]
fn eval_expr_returns_secret_when_any_loaded_slot_is_secret() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();

    let slots = vec![(SlotValue::I64(99), Taint::Secret)];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Secret)
}

// B-033: taint_accum never decreases during expression evaluation
#[test]
fn eval_expr_taint_accum_never_decreases() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]
    .into_boxed_slice();

    // First slot DerivedFromSecret, second Clean
    let slots = vec![
        (SlotValue::I64(10), Taint::DerivedFromSecret),
        (SlotValue::I64(20), Taint::Clean),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::DerivedFromSecret)
}

// B-037: eval_expr_inner rejects SlotUninitialized when loading from uninitialized slot
#[test]
fn eval_expr_load_slot_rejects_uninitialized_slot() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();

    // Slot 0 is not written, so it's uninitialized
    let slots: Vec<(SlotValue, Taint)> = vec![];

    let result = eval_workflow_with_slots(ops, slots, vec![]);
    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

