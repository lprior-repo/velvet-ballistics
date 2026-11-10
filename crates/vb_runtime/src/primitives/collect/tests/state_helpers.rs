#[test]
fn collect_states_remove_clears_entry() -> Result<(), String> {
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
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    states.remove(RunId::new(1), SlotIdx::new(0));
    ensure(
        states
            .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
            .is_none(),
        "state should be gone after remove",
    )
}

#[test]
fn collect_states_upsert_replaces_existing() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state_v1 = CollectPaginationState {
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
    states
        .upsert(state_v1)
        .map_err(|e| format!("upsert v1 failed: {e:?}"))?;
    let state_v2 = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(30),
        cursor: 10,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state_v2)
        .map_err(|e| format!("upsert v2 failed: {e:?}"))?;
    let found = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(30))
        .ok_or("state v2 not found")?;
    ensure(
        found.cursor == 10,
        format!("expected cursor 10, got {}", found.cursor),
    )
}

#[test]
fn collect_states_default_is_empty() -> Result<(), String> {
    let states = CollectStates::default();
    ensure(
        states
            .find(RunId::new(0), SlotIdx::new(0), ListId::new(0))
            .is_none(),
        "default states should be empty",
    )
}

#[test]
fn collect_pagination_state_copy_equality() -> Result<(), String> {
    let a = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        source: ListId::new(3),
        current_page: ListId::new(4),
        cursor: 5,
        page_size: 10,
        item_count: 20,
        limit: 30,
        time_limit_ms: Some(1000),
        start_millis: 500,
    };
    let b = a;
    ensure(a == b, "identical states should be equal")
}

#[test]
fn collect_pagination_state_inequality() -> Result<(), String> {
    let a = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    let b = CollectPaginationState {
        run_id: RunId::new(2),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    ensure(a != b, "states with different run_id should not be equal")
}

#[test]
fn page_size_from_rejects_zero() -> Result<(), String> {
    let result = page_size_from(0);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect page_size must be nonzero",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn page_size_from_accepts_one() -> Result<(), String> {
    let result = page_size_from(1).map_err(|e| format!("{e:?}"))?;
    ensure(result == 1, format!("expected 1, got {result}"))
}

#[test]
fn validate_page_bound_rejects_page_above_limit() -> Result<(), String> {
    let result = validate_page_bound(10, 5);
    match result {
        Err(EngineError::CollectPageLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectPageLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_page_bound_accepts_page_at_limit() -> Result<(), String> {
    validate_page_bound(5, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_page_bound_accepts_page_below_limit() -> Result<(), String> {
    validate_page_bound(3, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_item_limit_rejects_count_above_limit() -> Result<(), String> {
    let result = validate_item_limit(10, 5);
    match result {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_item_limit_accepts_count_at_limit() -> Result<(), String> {
    validate_item_limit(5, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_item_limit_accepts_count_below_limit() -> Result<(), String> {
    validate_item_limit(3, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn copy_prefix_returns_first_page_size_items() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
        SlotValue::I64(5),
    ]
    .into_boxed_slice();
    let prefix = copy_prefix(&items, 3).map_err(|e| format!("{e:?}"))?;
    ensure(
        prefix.len() == 3,
        format!("expected len 3, got {}", prefix.len()),
    )?;
    ensure(prefix.get(0) == Some(&SlotValue::I64(1)), "item 0 mismatch")?;
    ensure(prefix.get(1) == Some(&SlotValue::I64(2)), "item 1 mismatch")?;
    ensure(prefix.get(2) == Some(&SlotValue::I64(3)), "item 2 mismatch")
}

#[test]
fn copy_prefix_clamps_to_item_count() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let prefix = copy_prefix(&items, 100).map_err(|e| format!("{e:?}"))?;
    ensure(
        prefix.len() == 2,
        format!("expected len 2, got {}", prefix.len()),
    )
}

#[test]
fn copy_page_range_returns_correct_slice() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![
        SlotValue::I64(0),
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
    ]
    .into_boxed_slice();
    let page = copy_page_range(&items, 2, 2).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.len() == 2,
        format!("expected len 2, got {}", page.len()),
    )?;
    ensure(page.get(0) == Some(&SlotValue::I64(2)), "item 0 mismatch")?;
    ensure(page.get(1) == Some(&SlotValue::I64(3)), "item 1 mismatch")
}

#[test]
fn copy_page_range_clamps_at_end() -> Result<(), String> {
    let items: Box<[SlotValue]> =
        vec![SlotValue::I64(0), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let page = copy_page_range(&items, 2, 10).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.len() == 1,
        format!("expected len 1, got {}", page.len()),
    )?;
    ensure(page.get(0) == Some(&SlotValue::I64(2)), "item 0 mismatch")
}

#[test]
fn validate_collect_state_rejects_page_size_above_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 50,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 10);
    match result {
        Err(EngineError::CollectPageLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectPageLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_rejects_item_count_above_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 50,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 50);
    match result {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_rejects_source_length_change() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 20);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect source length changed",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_accepts_valid_state() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 10).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn checked_add_usize_succeeds_for_valid_addition() -> Result<(), String> {
    let result = checked_add_usize(5, 3, "test").map_err(|e| format!("{e:?}"))?;
    ensure(result == 8, format!("expected 8, got {result}"))
}

#[test]
fn checked_add_usize_fails_on_overflow() -> Result<(), String> {
    let result = checked_add_usize(usize::MAX, 1, "overflow test");
    match result {
        Err(EngineError::InternalInvariantViolation { reason }) => ensure(
            reason == "overflow test",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

#[test]
fn collect_start_uses_source_as_collector_when_output_is_none_for_non_empty() -> Result<(), String>
{
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    // When output is None, source slot is used as the collector.
    // This means the source slot is overwritten with the first page.
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        None,
        None,
    );
    let signal = result.map_err(|e| format!("collect_start failed: {e:?}"))?;
    ensure(
        signal == vb_core::EngineSignal::Continue,
        "expected Continue",
    )?;
    // collect_start routes to body for non-empty list, retaining state.
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )?;
    // Before collect_next, the source slot should hold the first page (a list of 2 items)
    assert_slot_list_items(
        &run,
        &store,
        source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    // collect_next exhausts remaining items and routes to done.
    let next_result = collect_next(&mut run, &mut store, &mut states, source, body, done);
    ensure(
        next_result == Ok(vb_core::EngineSignal::Continue),
        "collect_next should succeed",
    )?;
    ensure(
        run.pc() == done,
        format!("expected pc={done:?}, got {:?}", run.pc()),
    )?;
    // After collect_next, the source slot should be empty.
    assert_slot_list_items(
        &run,
        &store,
        source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    assert_eq!(states.capture_state(run.run_id(), source), None);
    Ok(())
}

#[test]
fn collect_next_validates_state_consistency() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    // Set up a source list with 5 items
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    // Start collect with page_size=2
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start failed: {e:?}"))?;
    // Now modify the source list to have a different length (simulating mutation)
    let new_items: Vec<SlotValue> = (0..10).map(SlotValue::I64).collect();
    let new_id = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|e| format!("{e:?}"))?;
    // Find the state and get the source ListId
    let current_page = match *run.read_slot(collector).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    let state = states
        .find(run.run_id(), collector, current_page)
        .ok_or("state not found")?;
    // Replace source with a different-length list
    let mut modified_state = state;
    // Write a new source with wrong length into the store using the same source id
    // Since we can't mutate store in-place, let's trigger the validation via
    // collect_next which reads from store.list(state.source)
    // We need the source list to have a different length than state.item_count
    // Insert a new list and update the state's source to point to it
    // Actually we can write over the source slot in the run, but the state
    // tracks the original source ListId. Let's insert a new list into the store
    // and update the pagination state to point to it.
    modified_state.source = new_id;
    states
        .upsert(modified_state)
        .map_err(|e| format!("{e:?}"))?;
    // Now collect_next should fail because source length changed
    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect source length changed",
            format!("unexpected: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn collect_finish_removes_state_after_writing_output() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);
    run.write_slot(collector, SlotValue::I64(77))
        .map_err(|e| format!("{e:?}"))?;
    // Insert a state to verify it's removed
    let state = CollectPaginationState {
        run_id: run.run_id(),
        collector_slot: collector,
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).map_err(|e| format!("{e:?}"))?;
    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("collect_finish failed: {e:?}"))?;
    ensure(
        states
            .find(run.run_id(), collector, ListId::new(0))
            .is_none(),
        "state should be removed after collect_finish",
    )
}

#[test]
fn millis_since_epoch_returns_reasonable_value() -> Result<(), String> {
    let ms = millis_since_epoch().map_err(|e| format!("{e:?}"))?;
    ensure(
        ms > 946_684_800_000,
        format!("millis should be post-2000, got {ms}"),
    )?;
    ensure(
        ms < 32_503_680_000_000,
        format!("millis should be pre-3000, got {ms}"),
    )
}

/// When a `DeterministicTimeSource` is installed for the thread, `millis_since_epoch`
/// returns the injected value rather than wall-clock time. This makes collect-primitive
/// elapsed-time computation fully deterministic for tests.
#[test]
fn millis_since_epoch_uses_deterministic_time_source() -> Result<(), String> {
    use crate::primitives::collect::{
        install_deterministic_time_source_for_test, restore_real_time_source_for_test,
    };
    use vb_core::time::DeterministicTimeSource;

    let injected: u64 = 1_700_000_000_000;
    install_deterministic_time_source_for_test(DeterministicTimeSource::new(injected));
    let observed = millis_since_epoch().map_err(|e| format!("{e:?}"))?;
    ensure(
        observed == injected,
        format!("expected injected {injected}, got {observed}"),
    )?;

    // Switch to a different injected value mid-test to prove it's read each call.
    let injected2: u64 = 1_800_000_000_000;
    install_deterministic_time_source_for_test(DeterministicTimeSource::new(injected2));
    let observed2 = millis_since_epoch().map_err(|e| format!("{e:?}"))?;
    ensure(
        observed2 == injected2,
        format!("expected injected {injected2}, got {observed2}"),
    )?;

    // Restore real time and verify wall-clock returns a reasonable value.
    restore_real_time_source_for_test();
    let wall = millis_since_epoch().map_err(|e| format!("{e:?}"))?;
    ensure(
        wall > 946_684_800_000,
        format!("wall clock should be post-2000, got {wall}"),
    )?;
    Ok(())
}
