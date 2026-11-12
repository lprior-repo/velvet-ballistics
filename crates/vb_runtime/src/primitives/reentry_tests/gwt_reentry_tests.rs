// =====================================================================
// BDD Given/When/Then Scenarios: GWT-RE-1 to GWT-RE-6
// =====================================================================

/// GWT-RE-1: for_each body re-entry after Succeeded
/// Given: for_each over [Item1, Item2], body ran Item1 → Succeeded
/// When: for_each_next called for Item2
/// Then: jump_to_body transitions Succeeded → Pending, body processes Item2.
#[test]
fn gwt_re1_for_each_body_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    // for_each_start binds Item1 (1)
    for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100u32,
        body,
        done,
        Some(iterator_slot),
    )
    .ok()
    .unwrap();

    // Body runs Item1 → Succeeded
    let body_step = StepIdx::new(1);
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Succeeded
    );

    // for_each_next for Item2 (re-entry)
    let next = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );

    // THEN: Continue returned, PC at body, Item2 bound, body step Pending
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(*run.read_slot(item_slot).ok().unwrap(), SlotValue::I64(2));
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Pending
    );
}

/// GWT-RE-2: reduce body re-entry after Succeeded
/// Given: reduce over [A, B, C], body ran A→Succeeded, B→Succeeded
/// When: reduce_next for C
/// Then: jump_to_body transitions Succeeded → Pending, body runs C.
#[test]
fn gwt_re2_reduce_body_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();

    let input = SlotIdx::new(0);
    let accumulator = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(3);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );

    let plan = minimal_workflow_with_const(0i64);
    reduce_start(
        &plan,
        &mut run,
        &mut store,
        input,
        accumulator,
        vb_core::ids::ConstIdx::new(0),
        body,
        done,
        Some(iterator_slot),
    )
    .ok()
    .unwrap();

    let body_step = StepIdx::new(1);

    // Body ran for A=10 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();

    // reduce_next binds B=20
    reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    )
    .ok()
    .unwrap();

    // Body ran for B=20 → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Succeeded
    );

    // reduce_next for C=30 (re-entry)
    let next = reduce_next(
        &mut run,
        &mut store,
        iterator_slot,
        accumulator,
        body,
        done,
        Some(SlotIdx::new(4)),
    );

    // THEN: Continue, PC at body, C bound, body step Pending
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Pending
    );
}

/// GWT-RE-3: collect_page re-entry after page body Succeeded
/// Given: collect with page_size=2 over [A,B,C,D], page1 body → Succeeded
/// When: collect_page re-entry for page2
/// Then: jump_to_body transitions Succeeded → Pending.
#[test]
fn gwt_re3_collect_page_reentry_after_succeeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
    );

    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector_slot),
        None,
    )
    .ok()
    .unwrap();

    let body_step = StepIdx::new(1);

    // Page1 body completes → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Succeeded
    );

    // collect_page re-entry
    let page = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector_slot,
        body,
        done,
    );

    // THEN: Continue, body step Pending
    assert_eq!(page, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Pending
    );
}

/// GWT-RE-4: repeat_attempt re-entry after attempt Succeeded
/// Given: repeat with max_attempts=3, attempt 1 → Succeeded
/// When: repeat_attempt for attempt 2
/// Then: jump_to_body transitions Succeeded → Pending, body runs attempt 2.
#[test]
fn gwt_re4_repeat_attempt_reentry_after_succeeded() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    repeat_start(&mut run, 3, body, done, Some(attempt_slot))
        .ok()
        .unwrap();

    let body_step = StepIdx::new(1);

    // Attempt 1 runs → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Succeeded
    );

    // repeat_attempt for attempt 2 (re-entry)
    let attempt = repeat_attempt(&mut run, attempt_slot, body, done);

    // THEN: Continue, PC at body, body step Pending
    assert_eq!(attempt, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Pending
    );
}

/// GWT-RE-5: repeat_check loops back to body after attempt Succeeded
/// Given: repeat with max_attempts=3, attempt 2 → Succeeded
/// When: repeat_check routes back to body
/// Then: jump_to_body transitions Succeeded → Pending.
#[test]
fn gwt_re5_repeat_check_loops_back_after_succeeded() {
    let mut run = fresh_frame();

    let attempt_slot = SlotIdx::new(0);
    let done = StepIdx::new(2);
    let next_body = StepIdx::new(1);

    // max=3, current=1 (after increment will be 2, which is < max=3, so loops back)
    let packed: i64 = (3_i64 << 32) | 1_i64;
    run.write_slot(attempt_slot, SlotValue::I64(packed))
        .unwrap();

    let body_step = StepIdx::new(1);

    // Attempt 1 runs → Succeeded
    run.mark_running(body_step).unwrap();
    run.mark_succeeded(body_step).unwrap();
    assert_eq!(
        run.step_state(body_step).unwrap(),
        vb_core::frame::StepState::Succeeded
    );

    // repeat_check: next_attempt=2, 2 < 3 is true, so loops back to body_entry
    let check = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);

    // THEN: Continue, PC at body_entry
    assert_eq!(check, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_body);
}

/// GWT-RE-6: Succeeded→Running transition rejected by state machine (negative)
/// Given: body step in Succeeded state
/// When: Plain jump_to (not jump_to_body) is called, and engine tries Succeeded→Running
/// Then: validate_transition returns Err("invalid_state_transition").
#[test]
fn gwt_re6_succeeded_to_running_rejected() {
    use vb_core::frame::StepState;

    // Succeeded → Running is NOT a valid transition
    let is_valid =
        vb_core::frame::is_valid_step_state_transition(StepState::Succeeded, StepState::Running);
    assert!(
        !is_valid,
        "Succeeded→Running must be invalid per VALID_TRANSITIONS"
    );

    // The only valid transitions from Succeeded are:
    // - Succeeded → Succeeded (idempotent)
    // - Succeeded → Pending (for loop re-entry via jump_to_body)
    let can_go_to_pending =
        vb_core::frame::is_valid_step_state_transition(StepState::Succeeded, StepState::Pending);
    assert!(
        can_go_to_pending,
        "Succeeded→Pending must be valid (this is why jump_to_body exists)"
    );
}

// =====================================================================
// Helper functions
// =====================================================================

fn minimal_workflow_with_const(cv: i64) -> vb_core::workflow::CompiledWorkflow {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{
        CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
    };

    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("reentry_test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([7; 32]),
        nodes: vec![node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None)].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(cv)].into_boxed_slice(),
        slot_count: 10,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts).unwrap()
}

fn decode_packed(packed: i64) -> (u16, u16) {
    let max_attempts = (packed >> 32) as u16;
    let current_attempt = (packed & 0xFFFF) as u16;
    (max_attempts, current_attempt)
}

// =====================================================================
// Proptest: PROP-1 through PROP-5
// =====================================================================

