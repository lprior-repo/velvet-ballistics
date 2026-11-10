/// validate_collect_state accepts when page_size == limit.
#[test]
fn validate_collect_state_accepts_page_size_at_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 5,
        limit: 10,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// validate_collect_state accepts when item_count == limit.
#[test]
fn validate_collect_state_accepts_item_count_at_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 10,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 10).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// checked_add_usize with zero operands succeeds.
#[test]
fn checked_add_usize_zero_plus_zero() -> Result<(), String> {
    let result = checked_add_usize(0, 0, "zero test").map_err(|e| format!("{e:?}"))?;
    ensure(result == 0, format!("expected 0, got {result}"))
}

/// CollectStates find with matching key but wrong slot returns None.
#[test]
fn collect_states_find_returns_none_for_wrong_slot() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).map_err(|e| format!("{e:?}"))?;
    let found = states.find(RunId::new(1), SlotIdx::new(5), ListId::new(20));
    ensure(
        found.is_none(),
        "find should return None for wrong collector_slot",
    )
}

/// Multiple rounds of start -> next produce independent state.
#[test]
fn collect_repeated_start_next_cycles() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        3,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("start 1: {e:?}"))?;

    // collect_start routes to body for non-empty list, retaining state.
    ensure(run.pc() == body, "first cycle should start at body")?;
    // collect_next exhausts remaining items and routes to done.
    let next_result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    ensure(
        next_result == Ok(vb_core::EngineSignal::Continue),
        "first cycle collect_next should succeed",
    )?;
    ensure(
        run.pc() == done,
        "first cycle should reach done after collect_next",
    )?;
    ensure(
        states.capture_state(run.run_id(), collector).is_none(),
        "state should be removed after exhaustion",
    )?;

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("start 2: {e:?}"))?;
    ensure(run.pc() == body, "second cycle should start at body")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);
    Ok(())
}

#[test]
fn collect_first_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let scenario = non_monotonic_collect_scenario()?;

    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(30), SlotValue::I64(10)])?;
    scenario.assert_current_cursor(2)?;
    Ok(())
}

#[test]
fn collect_second_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let mut scenario = non_monotonic_collect_scenario()?;

    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(20), SlotValue::I64(40)])?;
    scenario.assert_current_cursor(4)?;
    Ok(())
}

#[test]
fn collect_third_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let mut scenario = non_monotonic_collect_scenario()?;

    scenario.next()?;
    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(15)])?;
    scenario.assert_current_cursor(5)?;
    Ok(())
}

#[test]
fn collect_next_rejects_duplicate_first_page_response_after_cursor_advanced() -> Result<(), String>
{
    duplicate_page_rejection_scenario()?;
    Ok(())
}

#[test]
fn duplicate_first_page_rejection_preserves_advanced_state() -> Result<(), String> {
    let (scenario, advanced_page) = duplicate_page_rejection_scenario()?;

    scenario.assert_live_cursor(advanced_page, 4)?;
    Ok(())
}

#[test]
fn collect_next_rejects_stale_completion_page() -> Result<(), String> {
    stale_page_rejection_scenario()?;
    Ok(())
}

#[test]
fn stale_completion_page_rejection_preserves_live_state() -> Result<(), String> {
    let (scenario, live_page) = stale_page_rejection_scenario()?;

    scenario.assert_live_cursor(live_page, 2)?;
    Ok(())
}

#[test]
fn collect_start_enforces_page_size_bounds_before_allocating_page() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            2,
            3,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(collector),
            None,
        ),
        Err(EngineError::CollectPageLimitExceeded)
    );
    assert_eq!(
        run.read_slot(collector),
        Err(EngineError::SlotUninitialized { slot: collector })
    );
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    Ok(())
}

#[test]
fn collect_start_enforces_fanout_item_limit_at_exact_boundary() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            3,
            2,
            body,
            done,
            Some(collector),
            None,
        ),
        Ok(vb_core::EngineSignal::Continue)
    );
    assert_eq!(run.pc(), body);
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    assert_eq!(
        collect_state_for_current_page(&states, &run, collector)?.item_count,
        3
    );
    assert_eq!(
        collect_state_for_current_page(&states, &run, collector)?.limit,
        3
    );
    Ok(())
}

#[test]
fn collect_start_rejects_fanout_one_over_limit_without_collector_state() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            2,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(collector),
            None,
        ),
        Err(EngineError::CollectItemLimitExceeded)
    );
    assert_eq!(
        run.read_slot(collector),
        Err(EngineError::SlotUninitialized { slot: collector })
    );
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    Ok(())
}

#[test]
fn collect_next_honors_value_store_arena_cap_without_advancing_cursor() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::with_max_slots(2);
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let first_page = slot_list_id(&run, collector)?;

    assert_eq!(
        collect_next(&mut run, &mut store, &mut states, collector, body, done),
        Err(EngineError::BudgetExceeded {
            budget: "max_slots",
            limit: 2,
        })
    );
    assert_eq!(slot_list_id(&run, collector)?, first_page);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(1)]);
    assert_eq!(
        states
            .find(run.run_id(), collector, first_page)
            .ok_or("first-page state missing after arena-cap rejection".to_owned())?
            .cursor,
        1
    );
    Ok(())
}

#[test]
fn collect_next_writes_empty_page_and_removes_state_after_last_item() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(99)]);
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let _first_page = slot_list_id(&run, collector)?;

    // collect_start routes to body for non-empty list, retaining state.
    assert_eq!(run.pc(), body);
    // collect_next sees cursor at item_count and routes to done.
    let next_result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(99)]);
    Ok(())
}

#[test]
fn collect_next_rejects_empty_non_current_page_without_removing_state() -> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        3,
        1,
    )?;
    scenario.next()?;
    let live_page = scenario.current_page()?;
    let empty_page = scenario
        .store
        .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
        .map_err(|e| format!("empty page insert: {e:?}"))?;
    scenario.write_collector_page(empty_page)?;

    let result = collect_next(
        &mut scenario.run,
        &mut scenario.store,
        &mut scenario.states,
        scenario.collector,
        scenario.body,
        scenario.done,
    );

    assert!(matches!(
        result,
        Err(EngineError::CollectPageOrderViolation { .. })
    ));
    scenario.assert_live_cursor(live_page, 2)?;
    Ok(())
}

// =========================================================================
// vb-qi37.3.1 RED PHASE: collect state isolation contract tests.
// These tests encode the approved bead plan and are expected to fail until
// the implementation changes collect pagination completion/isolation behavior.
// =========================================================================

fn qi37_start_collect_with_items(
    items: Vec<SlotValue>,
    page_size: u32,
) -> Result<
    (
        RunFrame,
        ValueStore,
        CollectStates,
        SlotIdx,
        SlotIdx,
        StepIdx,
        StepIdx,
    ),
    String,
> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(&mut run, &mut store, source, items);
    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            32,
            page_size,
            body,
            done,
            Some(collector),
            None,
        ),
        Ok(vb_core::EngineSignal::Continue)
    );

    Ok((run, store, states, source, collector, body, done))
}

#[test]
fn collect_start_exact_page_limit_finishes_without_active_pagination_state() -> Result<(), String> {
    let (mut run, mut store, mut states, _source, collector, _body, done) =
        qi37_start_collect_with_items(vec![SlotValue::I64(10), SlotValue::I64(20)], 2)?;

    // collect_start routes to body for non-empty list, retaining state.
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(10), SlotValue::I64(20)],
    );
    // State is retained until collect_next exhausts.
    let retained = states
        .capture_state(run.run_id(), collector)
        .ok_or("missing retained state")?;
    assert_eq!(retained.cursor, 2);
    assert_eq!(retained.item_count, 2);
    // collect_next sees cursor at item_count and routes to done.
    let next_result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        done,
    );
    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(10), SlotValue::I64(20)],
    );
    Ok(())
}

#[test]
fn collect_next_returns_cursor_beyond_source_error_when_cursor_is_one_above_source_len()
-> Result<(), String> {
    let (mut run, mut store, mut states, _source, collector, body, done) =
        qi37_start_collect_with_items(vec![SlotValue::I64(10), SlotValue::I64(20)], 1)?;
    let page = slot_list_id(&run, collector)?;
    let state = states
        .find(run.run_id(), collector, page)
        .ok_or("expected active state".to_owned())?;
    states
        .upsert(CollectPaginationState { cursor: 3, ..state })
        .map_err(|e| format!("{e:?}"))?;

    assert_eq!(
        collect_next(&mut run, &mut store, &mut states, collector, body, done),
        Err(EngineError::InternalInvariantViolation {
            reason: "collect cursor beyond source items",
        })
    );
    Ok(())
}

// =========================================================================
// vb-qi37.3 STATE 5 RED PHASE: collect pagination durability/hydration.
// These tests intentionally target the approved typed API from the bead plan.
// The typed variants/kinds do not exist yet; compile failure is valid red
// evidence for State 6 and must not be hidden by accepting generic errors.
// =========================================================================

fn qi37_3_valid_state(
    run_id: RunId,
    collector_slot: SlotIdx,
    source: ListId,
    current_page: ListId,
) -> CollectPaginationState {
    CollectPaginationState {
        run_id,
        collector_slot,
        source,
        current_page,
        cursor: 2,
        page_size: 2,
        item_count: 4,
        limit: 4,
        time_limit_ms: None,
        start_millis: 10,
    }
}

#[test]
fn collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state()
-> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
        4,
        2,
    )?;
    let page1 = scenario.current_page()?;
    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    let page2 = scenario.current_page()?;
    let before = scenario
        .states
        .capture_state(scenario.run.run_id(), scenario.collector)
        .ok_or("expected live collect state before duplicate rejection".to_owned())?;

    scenario.write_collector_page(page1)?;
    let result = collect_next(
        &mut scenario.run,
        &mut scenario.store,
        &mut scenario.states,
        scenario.collector,
        scenario.body,
        scenario.done,
    );

    assert_eq!(
        result,
        Err(EngineError::CollectPageOrderViolation {
            kind: CollectPageOrderViolationKind::Duplicate,
            run_id: scenario.run.run_id(),
            collector_slot: scenario.collector,
            expected_page: page2,
            observed_page: page1,
        })
    );
    assert_eq!(
        scenario
            .states
            .capture_state(scenario.run.run_id(), scenario.collector),
        Some(before)
    );
    assert_eq!(slot_list_id(&scenario.run, scenario.collector)?, page1);
    Ok(())
}
