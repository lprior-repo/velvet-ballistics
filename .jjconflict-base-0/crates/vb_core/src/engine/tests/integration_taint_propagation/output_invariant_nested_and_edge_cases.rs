// =============================================================================
// C-090 to C-092: Output Taint >= Max Input Taint Invariant
// =============================================================================

// C-090: eval_expr output taint >= max input taint for all 5 variants
#[test]
fn eval_expr_output_taint_satisfies_output_gte_max_input() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]
    .into_boxed_slice();

    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];

    for a in all {
        for b in all {
            let max_input = join_taint(a, b);
            let slots = vec![(SlotValue::I64(1), a), (SlotValue::I64(1), b)];
            let (_, out_taint) =
                eval_workflow_with_slots(ops.clone(), slots, vec![]).map_err(|e| e.to_string())?;
            assert_eq!(
                out_taint, max_input,
                "output taint must equal max input for inputs ({a:?}, {b:?})"
            );
        }
    }
    Ok(())
}

// C-091: build_object output taint >= max field taint
#[test]
fn build_object_output_taint_satisfies_output_gte_max_field_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 3).map_err(|e| e.to_string())?;

    // Mix Random and TimeDependent
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Random)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
        (SymbolId::new(2), SlotIdx::new(2)),
    ];

    use crate::engine::object_list::build_object_with_taint;
    let (_, taint) =
        build_object_with_taint(&mut store, &run, &fields).map_err(|e| e.to_string())?;

    // Max of (Random, TimeDependent, Clean) = TimeDependent
    ensure_equal(taint, Taint::TimeDependent)
}

// C-092: build_list output taint >= max item taint
#[test]
fn build_list_output_taint_satisfies_output_gte_max_item_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 3).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Random)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1), SlotIdx::new(2)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    // Max of (DerivedFromSecret, Secret, Random) = Random
    ensure_equal(taint, Taint::Random)
}

// =============================================================================
// C-100 to C-102: Nested Structure Taint
// =============================================================================

// C-100: Nested list taint: outer list taint = max of inner item taints
#[test]
fn nested_list_outer_taint_reflects_max_inner_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 3).map_err(|e| e.to_string())?;

    // Build inner list with Secret taint
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::I64(20),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;

    let inner_items = vec![SlotIdx::new(0), SlotIdx::new(1)];
    let (inner_list, inner_taint) =
        crate::engine::object_list::build_list_with_taint(&mut store, &run, &inner_items)
            .map_err(|e| e.to_string())?;
    ensure_equal(inner_taint, Taint::Secret)?;

    // Store the inner list handle in a slot with its taint
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::List(inner_list), inner_taint)
        .map_err(|e| e.to_string())?;

    // Build outer list containing the inner list
    let outer_items = vec![SlotIdx::new(2)];
    let (_, outer_taint) =
        crate::engine::object_list::build_list_with_taint(&mut store, &run, &outer_items)
            .map_err(|e| e.to_string())?;

    // Outer taint must be >= inner taint (Secret)
    ensure_equal(outer_taint, Taint::Secret)
}

// C-101: Nested object taint: outer object taint = max of inner field taints
#[test]
fn nested_object_outer_taint_reflects_max_inner_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 4).map_err(|e| e.to_string())?;

    // Build inner object with Random taint fields
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Random)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let inner_fields = vec![
        (SymbolId::new(0), SlotIdx::new(0)),
        (SymbolId::new(1), SlotIdx::new(1)),
    ];
    let (inner_obj, inner_taint) =
        crate::engine::object_list::build_object_with_taint(&mut store, &run, &inner_fields)
            .map_err(|e| e.to_string())?;
    ensure_equal(inner_taint, Taint::Random)?;

    // Store inner object with its taint
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::Object(inner_obj), inner_taint)
        .map_err(|e| e.to_string())?;

    // Build outer object containing inner object + a TimeDependent field
    run.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(3), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let outer_fields = vec![
        (SymbolId::new(0), SlotIdx::new(2)),
        (SymbolId::new(1), SlotIdx::new(3)),
    ];
    let (_, outer_taint) =
        crate::engine::object_list::build_object_with_taint(&mut store, &run, &outer_fields)
            .map_err(|e| e.to_string())?;

    // Max of (Random, TimeDependent) = TimeDependent
    ensure_equal(outer_taint, Taint::TimeDependent)
}

// C-102: Three-level nested structure taint propagation
#[test]
fn three_level_nested_structure_taint_propagation() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 8).map_err(|e| e.to_string())?;

    // Level 0: Write a Secret-tagged value
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
        .map_err(|e| e.to_string())?;

    // Level 1: Build inner list from Secret slot
    let (inner_list, t1) =
        crate::engine::object_list::build_list_with_taint(&mut store, &run, &[SlotIdx::new(0)])
            .map_err(|e| e.to_string())?;
    ensure_equal(t1, Taint::Secret)?;

    // Store inner list in a slot
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::List(inner_list), t1)
        .map_err(|e| e.to_string())?;

    // Level 2: Build mid object containing inner list
    let (mid_obj, t2) = crate::engine::object_list::build_object_with_taint(
        &mut store,
        &run,
        &[(SymbolId::new(0), SlotIdx::new(1))],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(t2, Taint::Secret)?;

    // Store mid object
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::Object(mid_obj), t2)
        .map_err(|e| e.to_string())?;

    // Add a Random-tainted slot for the outer structure
    run.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(1), Taint::Random)
        .map_err(|e| e.to_string())?;

    // Level 3: Build outer list containing mid object + Random slot
    let (_, t3) = crate::engine::object_list::build_list_with_taint(
        &mut store,
        &run,
        &[SlotIdx::new(2), SlotIdx::new(3)],
    )
    .map_err(|e| e.to_string())?;

    // Outer taint = join(Secret, Random) = Random
    ensure_equal(t3, Taint::Random)
}

// =============================================================================
// C-110 to C-112: Additional Error/Edge Cases
// =============================================================================

// C-110: write_taint rejects uninitialized slot when writing TimeDependent
#[test]
fn write_taint_rejects_uninitialized_slot_with_time_dependent() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    let result = run.write_taint(SlotIdx::ZERO, Taint::TimeDependent);

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::ZERO => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

// C-111: write_taint rejects uninitialized slot when writing Random
#[test]
fn write_taint_rejects_uninitialized_slot_with_random() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    let result = run.write_taint(SlotIdx::ZERO, Taint::Random);

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::ZERO => Ok(()),
        other => Err(format!("expected SlotUninitialized, got: {other:?}")),
    }
}

// C-112: read_taint returns TimeDependent after write_slot_with_taint
#[test]
fn read_taint_reads_time_dependent_after_write() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let taint = run.read_taint(SlotIdx::ZERO).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::TimeDependent)
}
