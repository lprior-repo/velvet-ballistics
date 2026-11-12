// =============================================================================
// B-040 to B-047: BuildObject Taint Propagation (POST-002)
// =============================================================================

// B-040: build_object_with_taint returns Clean when all fields Clean
#[test]
fn build_object_with_taint_returns_clean_when_all_fields_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Clean)
}

// B-041: build_object_with_taint returns DerivedFromSecret when any field has DerivedFromSecret
#[test]
fn build_object_with_taint_returns_derived_from_secret_when_any_field_has_that_taint()
-> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::DerivedFromSecret)
}

// B-042: build_object_with_taint returns Secret when any field is Secret
#[test]
fn build_object_with_taint_returns_secret_when_any_field_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Secret)
}

// B-043: build_object_with_taint joins taint across all fields (order-independent)
#[test]
fn build_object_with_taint_joins_taint_across_fields_order_independent() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 3).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    // Fields in different order: slot2, slot0, slot1
    let fields = vec![
        (SymbolId::new(2), SlotIdx::new(2)),
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    // With Secret in any field, result must be Secret
    ensure_equal(taint, Taint::Secret)
}

// B-047: Impossible to produce Clean-tainted object from Secret-tainted inputs
#[test]
fn build_object_cannot_produce_clean_taint_from_secret_inputs() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let fields = vec![(SymbolId::new(0), SlotIdx::new(0))];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    // Must be Secret, never Clean
    ensure_equal(taint, Taint::Secret)
}

// =============================================================================
// B-050 to B-057: BuildList Taint Propagation (POST-003)
// =============================================================================

// B-050: build_list_with_taint returns Clean when all items Clean
#[test]
fn build_list_with_taint_returns_clean_when_all_items_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Clean)
}

// B-051: build_list_with_taint returns DerivedFromSecret when any item has DerivedFromSecret
#[test]
fn build_list_with_taint_returns_derived_from_secret_when_any_item_has_that_taint()
-> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::DerivedFromSecret)
}

// B-052: build_list_with_taint returns Secret when any item is Secret
#[test]
fn build_list_with_taint_returns_secret_when_any_item_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Secret)
}

// B-053: build_list_with_taint joins taint across all items (order-independent)
#[test]
fn build_list_with_taint_joins_taint_across_items_order_independent() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 3).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    // Items in different order: slot1, slot2, slot0
    let items = vec![SlotIdx::new(1), SlotIdx::new(2), SlotIdx::new(0)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    // With Secret in any item, result must be Secret
    ensure_equal(taint, Taint::Secret)
}

// B-057: Impossible to produce Clean-tainted list from Secret-tainted inputs
#[test]
fn build_list_cannot_produce_clean_taint_from_secret_inputs() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    // Must be Secret, never Clean
    ensure_equal(taint, Taint::Secret)
}

// =============================================================================
// B-060 to B-068: Choose Taint Semantics (POST-004)
// =============================================================================

// B-060: choose_expr_branch does not accumulate taint from condition evaluation
#[test]
fn choose_expr_branch_does_not_accumulate_taint_from_condition() -> Result<(), String> {
    // This test verifies that choose_expr_branch does not propagate taint
    // from the condition evaluation to the overall operation.
    // The function should just return Continue with PC set to branch target.
    use crate::engine::choose::choose_expr_branch;

    let expr_true =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(|e| crate::WorkflowError::Expression(e))
            .map_err(|e| e.to_string())?;

    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "choose_taint_test".into(),
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
                    otherwise: Some(StepIdx::new(2)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
            crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        ]
        .into(),
        expressions: vec![expr_true].into(),
        accessors: vec![].into(),
        constants: vec![ConstValue::Bool(true)].into(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 1).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Write Secret to slot 0 - condition will be true (Bool(true))
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let branches = vec![ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(1),
    }];

    let result = choose_expr_branch(
        &workflow,
        &mut run,
        &mut store,
        &branches,
        Some(StepIdx::new(2)),
    )
    .map_err(|e| e.to_string())?;

    // Result should be Continue with PC set to StepIdx(1)
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

// B-061: choose_slot_branch does not accumulate taint from slot reads
#[test]
fn choose_slot_branch_does_not_accumulate_taint_from_slot_reads() -> Result<(), String> {
    use crate::engine::choose::choose_slot_branch;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    // Write Secret taint but Bool(true) value
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let result = choose_slot_branch(&mut run, &branches, Some(StepIdx::new(2)))
        .map_err(|e| e.to_string())?;

    // Result should be Continue with PC set to StepIdx(1)
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

// =============================================================================
// B-070 to B-073: Finish Taint Propagation (POST-005)
// =============================================================================

// B-070: finish_run returns Finished signal with exact slot taint
#[test]
fn finish_run_returns_finished_with_exact_slot_taint() -> Result<(), String> {
    use crate::engine::node_helpers::finish_run;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::I64(42),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;

    let result = finish_run(&mut run, SlotIdx::new(1)).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(42), Taint::DerivedFromSecret),
    )
}

// B-072: finish_run returns SlotOutOfBounds for out-of-range result slot
#[test]
fn finish_run_returns_slot_out_of_bounds_for_oob_result_slot() -> Result<(), String> {
    use crate::engine::node_helpers::finish_run;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    // The frame has slot_count = 2, so slot 99 is out of bounds
    let result = finish_run(&mut run, SlotIdx::new(99));

    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        other => Err(format!("expected SlotOutOfBounds, got: {other:?}")),
    }
}

// B-073: finish_run preserves exact taint (no promotion/demotion)
#[test]
fn finish_run_preserves_exact_taint_no_promotion_or_demotion() -> Result<(), String> {
    use crate::engine::node_helpers::finish_run;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(99), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;

    let result = finish_run(&mut run, SlotIdx::ZERO).map_err(|e| e.to_string())?;

    // Must be exactly DerivedFromSecret, not promoted to Secret or demoted to Clean
    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::DerivedFromSecret),
    )
}

