// =============================================================================
// Error Handling Tests (B-200 to B-211)
// =============================================================================

// B-200: eval_expr_inner returns SlotOutOfBounds on invalid slot access
#[test]
fn eval_expr_returns_slot_out_of_bounds_for_invalid_slot() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(99))].into_boxed_slice();

    // slot_count is only 2 (indices 0 and 1), so 99 is OOB
    let slots = vec![
        (SlotValue::I64(1), Taint::Clean),
        (SlotValue::I64(2), Taint::Clean),
    ];

    let result = eval_workflow_with_slots(ops, slots, vec![]);
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        other => Err(format!("expected SlotOutOfBounds, got: {other:?}")),
    }
}

// B-201: eval_expr_inner returns SlotUninitialized on uninitialized slot read
#[test]
fn eval_expr_returns_slot_uninitialized_when_slot_not_written() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();

    // Don't write to any slot - slot 0 is uninitialized
    let slots: Vec<(SlotValue, Taint)> = vec![];

    let result = eval_workflow_with_slots(ops, slots, vec![]);
    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

// B-202: eval_expr_inner returns ExprOutOfBounds on invalid expression index
#[test]
fn eval_expr_returns_expr_out_of_bounds_for_invalid_index() -> Result<(), String> {
    // Create a workflow with 1 valid expression at index 0
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice();
    let max_stack =
        crate::workflow::check_expr_stack_bound(&ops, crate::limits::MAX_EXPRESSION_STACK)
            .map_err(|e| e.to_string())?;
    let expr = ExprProgram::try_from_parts(ops, max_stack).map_err(|e| e.to_string())?;

    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "expr_oob_test".into(),
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
        constants: vec![ConstValue::I64(1)].into(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // ExprIdx(5) doesn't exist - only ExprIdx(0) exists
    let result = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(5));

    match result {
        Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(5) => Ok(()),
        other => Err(format!("expected ExprOutOfBounds, got: {other:?}")),
    }
}

// B-203: eval_expr_inner returns ConstOutOfBounds on invalid constant index
#[test]
fn eval_expr_returns_const_out_of_bounds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(99))].into_boxed_slice();

    // Only constant at index 0 exists
    let slots: Vec<(SlotValue, Taint)> = vec![];

    let result = eval_workflow_with_slots(ops, slots, vec![ConstValue::I64(1)]);
    match result {
        Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(99) => Ok(()),
        other => Err(format!("expected ConstOutOfBounds, got: {other:?}")),
    }
}

// B-204: build_object_with_taint returns SlotOutOfBounds on invalid field slot
#[test]
fn build_object_with_taint_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    let fields = vec![(SymbolId::new(0), SlotIdx::new(5))]; // Slot 5 is OOB

    use crate::engine::object_list::build_object_with_taint;
    let result = build_object_with_taint(&mut store, &run, &fields);

    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("expected SlotOutOfBounds, got: {other:?}")),
    }
}

// B-206: build_list_with_taint returns SlotOutOfBounds on invalid item slot
#[test]
fn build_list_with_taint_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(5)]; // Slot 5 is OOB

    use crate::engine::object_list::build_list_with_taint;
    let result = build_list_with_taint(&mut store, &run, &items);

    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("expected SlotOutOfBounds, got: {other:?}")),
    }
}

// B-208: choose_expr_branch returns MissingNextStep when no match and no otherwise
#[test]
fn choose_expr_branch_returns_missing_next_step_when_no_match_and_no_otherwise()
-> Result<(), String> {
    use crate::engine::choose::choose_expr_branch;

    let expr_false =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(|e| crate::WorkflowError::Expression(e))
            .map_err(|e| e.to_string())?;

    // Need at least 2 nodes: node 0 is the Choose, node 1 is a valid branch target
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "choose_missing_test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: None.into(),
                },
                None,
                None,
            ),
            // This node exists so StepIdx(1) is a valid target
            crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        ]
        .into(),
        expressions: vec![expr_false].into(),
        accessors: vec![].into(),
        constants: vec![ConstValue::Bool(false)].into(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let branches = vec![ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(1),
    }];

    let result = choose_expr_branch(&workflow, &mut run, &mut store, &branches, None);

    match result {
        Err(EngineError::MissingNextStep { .. }) => Ok(()),
        other => Err(format!("expected MissingNextStep, got: {other:?}")),
    }
}

// B-209: choose_slot_branch returns TypeMismatch when condition is non-boolean
#[test]
fn choose_slot_branch_returns_type_mismatch() -> Result<(), String> {
    use crate::engine::choose::choose_slot_branch;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1).map_err(|e| e.to_string())?;

    // Write I64(1) instead of Bool
    run.write_slot(SlotIdx::ZERO, SlotValue::I64(1))
        .map_err(|e| e.to_string())?;

    let branches = vec![SlotBranch {
        condition: SlotIdx::ZERO,
        target: StepIdx::new(1),
    }];

    let result = choose_slot_branch(&mut run, &branches, Some(StepIdx::new(2)));

    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("expected TypeMismatch, got: {other:?}")),
    }
}

// B-210: finish_run returns SlotUninitialized when result not written
#[test]
fn finish_run_returns_slot_uninitialized_when_result_not_written() -> Result<(), String> {
    use crate::engine::node_helpers::finish_run;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1).map_err(|e| e.to_string())?;

    // Don't write to slot 0 - it's uninitialized
    let result = finish_run(&mut run, SlotIdx::ZERO);

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::ZERO => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

// B-211: copy_slot returns SlotUninitialized when source not written
#[test]
fn copy_slot_returns_slot_uninitialized_for_slot_zero_source() -> Result<(), String> {
    use crate::engine::node_helpers::copy_slot;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 3).map_err(|e| e.to_string())?;

    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );

    // Don't write to slot 0 - it's uninitialized
    let result = copy_slot(&mut run, &node, SlotIdx::new(0));

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

// =============================================================================
// C-001 to C-005: Full Taint Lattice (Random, TimeDependent)
// =============================================================================

// C-001: join_taint with Random absorbs Clean, DerivedFromSecret, and Secret
#[test]
fn join_taint_random_absorbs_secret_derived_and_clean() {
    assert_eq!(join_taint(Taint::Random, Taint::Clean), Taint::Random);
    assert_eq!(
        join_taint(Taint::Random, Taint::DerivedFromSecret),
        Taint::Random
    );
    assert_eq!(join_taint(Taint::Random, Taint::Secret), Taint::Random);
    assert_eq!(join_taint(Taint::Random, Taint::Random), Taint::Random);
}

// C-002: join_taint with TimeDependent absorbs all other taint levels
#[test]
fn join_taint_time_dependent_absorbs_all_other_taint_levels() {
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::Clean),
        Taint::TimeDependent
    );
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::DerivedFromSecret),
        Taint::TimeDependent
    );
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::Secret),
        Taint::TimeDependent
    );
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::Random),
        Taint::TimeDependent
    );
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::TimeDependent),
        Taint::TimeDependent
    );
}

// C-003: TimeDependent joined with Random returns TimeDependent
#[test]
fn join_taint_time_dependent_with_random_returns_time_dependent() {
    assert_eq!(
        join_taint(Taint::TimeDependent, Taint::Random),
        Taint::TimeDependent
    );
    assert_eq!(
        join_taint(Taint::Random, Taint::TimeDependent),
        Taint::TimeDependent
    );
}

// C-004: join_taint lattice order is total: Clean < DerivedFromSecret < Secret < Random < TimeDependent
#[test]
fn join_taint_lattice_order_is_total() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for (i, lo) in all.iter().enumerate() {
        for (j, hi) in all.iter().enumerate() {
            let result = join_taint(*lo, *hi);
            if i <= j {
                assert_eq!(
                    result, *hi,
                    "join({lo:?}, {hi:?}) must be {hi:?}, got {result:?}"
                );
            } else {
                assert_eq!(
                    result, *lo,
                    "join({lo:?}, {hi:?}) must be {lo:?}, got {result:?}"
                );
            }
        }
    }
}

// C-005: join_taint with any variant against itself is idempotent
#[test]
fn join_taint_is_idempotent_for_all_variants() {
    for v in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        assert_eq!(join_taint(v, v), v, "idempotent failed for {v:?}");
    }
}

// =============================================================================
// C-006 to C-008: TimeDependent as TRUE Lattice Top
// =============================================================================

// C-006: TimeDependent is lattice top for all variants
#[test]
fn time_dependent_is_lattice_top_for_all_variants() {
    for v in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
    ] {
        assert_eq!(
            join_taint(v, Taint::TimeDependent),
            Taint::TimeDependent,
            "TimeDependent must absorb {v:?}"
        );
        assert_eq!(
            join_taint(Taint::TimeDependent, v),
            Taint::TimeDependent,
            "TimeDependent must absorb {v:?} (reversed)"
        );
    }
}

// C-007: Secret is NOT the lattice top; Random and TimeDependent outrank it
#[test]
fn secret_is_not_the_lattice_top_random_and_time_dependent_outrank_it() {
    // Secret < Random in lattice
    assert_eq!(
        join_taint(Taint::Secret, Taint::Random),
        Taint::Random,
        "Random outranks Secret"
    );
    // Secret < TimeDependent in lattice
    assert_eq!(
        join_taint(Taint::Secret, Taint::TimeDependent),
        Taint::TimeDependent,
        "TimeDependent outranks Secret"
    );
}

// C-008: Random sits between Secret and TimeDependent in the lattice
#[test]
fn random_sits_between_secret_and_time_dependent_in_lattice() {
    // Secret < Random
    assert_eq!(join_taint(Taint::Secret, Taint::Random), Taint::Random);
    // Random < TimeDependent
    assert_eq!(
        join_taint(Taint::Random, Taint::TimeDependent),
        Taint::TimeDependent
    );
    // Clean < Random
    assert_eq!(join_taint(Taint::Clean, Taint::Random), Taint::Random);
}

// =============================================================================
// C-010 to C-012: Taint Sanitization (Secret→Clean if allowed)
// =============================================================================

// C-010: write_taint can downgrade taint from Secret to Clean on an initialized slot
#[test]
fn write_taint_can_downgrade_secret_to_clean() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)?;

    // Downgrade to Clean
    run.write_taint(SlotIdx::ZERO, Taint::Clean)
        .map_err(|e| e.to_string())?;
    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)
}

// C-011: write_slot implicitly sets taint to Clean (zeroes taint)
#[test]
fn write_slot_implicitly_sets_taint_to_clean() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)?;

    // write_slot uses write_slot_with_taint with Clean
    run.write_slot(SlotIdx::ZERO, SlotValue::I64(99))
        .map_err(|e| e.to_string())?;
    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)?;

    let value = run.read_slot(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(*value, SlotValue::I64(99))
}

// C-012: reinitialize resets all taint to Clean including TimeDependent
#[test]
fn reinitialize_resets_time_dependent_taint_to_clean() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    run.reinitialize(RunId::new(2), StepIdx::new(0), 1, 1)
        .map_err(|e| e.to_string())?;

    // After reinitialize, slot is uninitialized
    match run.read_taint(SlotIdx::ZERO) {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::ZERO => Ok(()),
        other => Err(format!(
            "expected SlotUninitialized after reinit, got: {other:?}"
        )),
    }
}

