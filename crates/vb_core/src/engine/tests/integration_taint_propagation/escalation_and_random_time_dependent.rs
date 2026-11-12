// =============================================================================
// C-013 to C-015: Taint Escalation Prevention
// =============================================================================

// C-013: join_taint never returns a taint lower than either input
#[test]
fn join_taint_never_lower_than_either_input() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for a in all {
        for b in all {
            let result = join_taint(a, b);
            // Check monotonicity: the result is >= each individual input
            let disc_a = match a {
                Taint::Clean => 0u8,
                Taint::DerivedFromSecret => 1,
                Taint::Secret => 2,
                Taint::Random => 3,
                Taint::TimeDependent => 4,
            };
            let disc_b = match b {
                Taint::Clean => 0u8,
                Taint::DerivedFromSecret => 1,
                Taint::Secret => 2,
                Taint::Random => 3,
                Taint::TimeDependent => 4,
            };
            let disc_result = match result {
                Taint::Clean => 0u8,
                Taint::DerivedFromSecret => 1,
                Taint::Secret => 2,
                Taint::Random => 3,
                Taint::TimeDependent => 4,
            };
            assert!(
                disc_result >= disc_a && disc_result >= disc_b,
                "join({a:?}, {b:?}) = {result:?} violates monotonicity"
            );
        }
    }
}

// C-014: No operation can spontaneously escalate taint without an input carrying that taint
#[test]
fn eval_expr_does_not_escalate_to_random_without_random_input() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]
    .into_boxed_slice();

    let slots = vec![
        (SlotValue::I64(1), Taint::Secret),
        (SlotValue::I64(2), Taint::Clean),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    // Should be Secret (max of inputs), NOT escalated to Random or TimeDependent
    ensure_equal(taint, Taint::Secret)
}

// C-015: eval_expr does not spontaneously introduce Random taint from Clean inputs
#[test]
fn eval_expr_does_not_spontaneously_introduce_random_taint_from_clean_inputs() -> Result<(), String>
{
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Mul,
    ]
    .into_boxed_slice();

    let slots = vec![
        (SlotValue::I64(5), Taint::Clean),
        (SlotValue::I64(7), Taint::Clean),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Clean)
}

// =============================================================================
// C-020 to C-022: Cross-Component Taint Tracking
// =============================================================================

// C-020: finish_run preserves TimeDependent taint in Finished signal
#[test]
fn finish_run_preserves_time_dependent_taint_in_finished_signal() -> Result<(), String> {
    use crate::engine::node_helpers::finish_run;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let result = finish_run(&mut run, SlotIdx::ZERO).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(1), Taint::TimeDependent),
    )
}

// C-021: copy_slot preserves Random taint across slots
#[test]
fn copy_slot_preserves_random_taint() -> Result<(), String> {
    use crate::engine::node_helpers::copy_slot;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 3).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Random)
        .map_err(|e| e.to_string())?;

    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );

    copy_slot(&mut run, &node, SlotIdx::new(0)).map_err(|e| e.to_string())?;

    let taint = run.read_taint(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Random)
}

// C-022: resume_action_completion preserves Random taint after resume
#[test]
fn resume_action_completion_preserves_random_taint() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "resume_random".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: vec![].into(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = action_ticket(
        RunId::new(1),
        StepIdx::new(0),
        SeqNo::new(1),
        ActionId::new(1),
    );
    let contract = action_contract(ActionId::new(1), 8);
    let output = action_output(SlotIdx::ZERO, SlotValue::Bool(true), Taint::Random);

    let (signal, _journal) =
        resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract)
            .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;

    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Random)
}

// =============================================================================
// C-030 to C-032: Random/TimeDependent in EvalExpr
// =============================================================================

// C-030: eval_expr_with_store returns Random when any loaded slot is Random
#[test]
fn eval_expr_returns_random_when_any_loaded_slot_is_random() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();

    let slots = vec![(SlotValue::I64(99), Taint::Random)];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Random)
}

// C-031: eval_expr_with_store returns TimeDependent when any loaded slot is TimeDependent
#[test]
fn eval_expr_returns_time_dependent_when_any_loaded_slot_is_time_dependent() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();

    let slots = vec![(SlotValue::I64(42), Taint::TimeDependent)];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::TimeDependent)
}

// C-032: eval_expr with multiple slots returns max taint among TimeDependent, Random, Secret
#[test]
fn eval_expr_returns_max_taint_among_time_dependent_random_secret() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::Add,
        ExprOp::Add,
    ]
    .into_boxed_slice();

    let slots = vec![
        (SlotValue::I64(1), Taint::Secret),
        (SlotValue::I64(2), Taint::Random),
        (SlotValue::I64(3), Taint::TimeDependent),
    ];

    let (_, taint) = eval_workflow_with_slots(ops, slots, vec![]).map_err(|e| e.to_string())?;

    // TimeDependent is the true top, outranking Random and Secret
    ensure_equal(taint, Taint::TimeDependent)
}

// =============================================================================
// C-040 to C-043: BuildObject with Random/TimeDependent Taint
// =============================================================================

// C-040: build_object_with_taint returns Random when any field is Random
#[test]
fn build_object_with_taint_returns_random_when_any_field_is_random() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Random)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Random)
}

// C-041: build_object_with_taint returns TimeDependent when any field is TimeDependent
#[test]
fn build_object_with_taint_returns_time_dependent_when_any_field_time_dependent()
-> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::TimeDependent)
}

// C-042: build_object_with_taint returns TimeDependent when fields have mixed taint
#[test]
fn build_object_with_taint_joins_mixed_taint_including_time_dependent() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 4).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(4), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
        (SymbolId::new(2), SlotIdx::new(2)),
        (SymbolId::new(3), SlotIdx::new(3)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::TimeDependent)
}

// C-043: build_object_with_taint stores field-level taint in ValueStore
#[test]
fn build_object_with_taint_stores_random_field_taint_in_value_store() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Random)
        .map_err(|e| e.to_string())?;

    let fields = vec![(SymbolId::new(0), SlotIdx::ZERO)];

    use crate::engine::object_list::build_object_with_taint;
    let (obj, _) = build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    let stored_fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(stored_fields.len(), 1)?;
    ensure_equal(stored_fields[0].taint, Taint::Random)
}

