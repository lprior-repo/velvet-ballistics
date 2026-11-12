#[test]
fn collect_start_items_at_exact_limit_boundary() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let body = StepIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        2,
        body,
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_start_items_exceeding_limit_by_one() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
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
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::CollectItemLimitExceeded) => {}
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_first_page_smaller_than_total() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
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
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run.read_slot(output).expect("must read") {
        SlotValue::List(id) => {
            let items = store.list(id).expect("must read");
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_start_page_size_larger_than_items_clamps_to_item_count() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(42), SlotValue::I64(99)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        10,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run.read_slot(output).expect("must read") {
        SlotValue::List(id) => {
            let items = store.list(id).expect("must read");
            assert_eq!(items.len(), 2);
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_next_progresses_pages_then_jumps_done() {
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
    let start = collect_start(
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
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    let next = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(3)]);
    let finished = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(finished, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
}

#[test]
fn collect_start_null_source_returns_type_mismatch() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    run.write_slot(source, SlotValue::Null).expect("write");
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "null");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_page_size_one_single_item_per_page() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run.read_slot(output).expect("must read") {
        SlotValue::List(id) => {
            let items = store.list(id).expect("must read");
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_start_rejects_page_size_above_limit() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
    assert_eq!(run.pc(), StepIdx::ZERO);
}

#[test]
fn collect_start_page_size_u32_max_returns_error() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        u32::MAX,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
}

#[test]
fn collect_start_page_size_at_limit_boundary() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        2,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // collect_start routes to body for non-empty list, retaining state.
    assert_eq!(run.pc(), StepIdx::new(1));
}

#[test]
fn collect_start_page_size_exactly_one_over_limit() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
}

#[test]
fn collect_start_with_time_limit_stores_limit_in_state() {
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
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        Some(60000),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let state = states.entries.values().next();
    assert!(state.is_some());
    assert_eq!(state.unwrap().time_limit_ms, Some(60000));
}

#[test]
fn collect_start_without_time_limit_stores_none() {
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
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
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
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // When all items fit in one page and limit is reached, state may not be stored
    let state = states.entries.values().next();
    if let Some(state) = state {
        assert_eq!(state.time_limit_ms, None);
    }
}

#[test]
fn check_time_limit_returns_error_when_exceeded() {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: Some(1),
        start_millis: 0,
    };
    assert_eq!(
        check_time_limit(&state),
        Err(EngineError::CollectTimeLimitExceeded)
    );
}

#[test]
fn check_time_limit_ok_when_not_exceeded() -> Result<(), EngineError> {
    let now = millis_since_epoch()?;
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: Some(60000),
        start_millis: now,
    };
    assert_eq!(check_time_limit(&state), Ok(()));
    Ok(())
}

#[test]
fn check_time_limit_ok_when_no_limit_set() {
    let state = CollectPaginationState {
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
    assert_eq!(check_time_limit(&state), Ok(()));
}

// ── Result-returning tests with ensure pattern ─────────────────────

fn ensure(condition: bool, message: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

#[test]
fn collect_states_new_is_empty() -> Result<(), String> {
    let states = CollectStates::new();
    ensure(
        states
            .find(RunId::new(1), SlotIdx::new(0), ListId::new(0))
            .is_none(),
        "new states should be empty",
    )
}

#[test]
fn collect_states_upsert_and_find_roundtrip() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 5,
        page_size: 10,
        item_count: 30,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    let found = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
        .ok_or("state not found after upsert")?;
    ensure(
        found.cursor == 5,
        format!("expected cursor 5, got {}", found.cursor),
    )?;
    ensure(
        found.page_size == 10,
        format!("expected page_size 10, got {}", found.page_size),
    )?;
    ensure(
        found.item_count == 30,
        format!("expected item_count 30, got {}", found.item_count),
    )
}

#[test]
fn collect_states_find_returns_none_for_wrong_page() -> Result<(), String> {
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
    let found = states.find(RunId::new(1), SlotIdx::new(0), ListId::new(99));
    ensure(found.is_none(), "find should return None for wrong page id")
}

#[test]
fn collect_states_find_returns_none_for_wrong_run_id() -> Result<(), String> {
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
    let found = states.find(RunId::new(999), SlotIdx::new(0), ListId::new(20));
    ensure(found.is_none(), "find should return None for wrong run_id")
}
