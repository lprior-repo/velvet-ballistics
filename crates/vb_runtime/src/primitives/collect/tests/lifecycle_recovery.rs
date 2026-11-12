// =========================================================================
// Additional coverage: state machine transitions, pagination boundaries,
// capacity enforcement, and collect lifecycle edge cases.
// =========================================================================

/// Full start -> page -> next -> next -> done lifecycle with page_size=1.
#[test]
fn collect_full_lifecycle_single_item_pages() -> Result<(), String> {
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
        vec![
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(30),
            SlotValue::I64(40),
        ],
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
    .map_err(|e| format!("collect_start: {e:?}"))?;
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 1: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 1")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 2: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 2")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(30)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 3: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 3")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(40)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 4: {e:?}"))?;
    ensure(
        run.pc() == done,
        format!("expected pc={done:?} after exhaustion, got {:?}", run.pc()),
    )?;
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(30),
            SlotValue::I64(40),
        ],
    );
    Ok(())
}

/// Captured pagination extra hydrates a fresh state table and preserves CollectNext progress.
#[test]
fn collect_pagination_extra_round_trips_for_recovery() -> Result<(), String> {
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
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
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
    .map_err(|e| format!("collect_start: {e:?}"))?;

    let extra = states
        .capture_extra(run.run_id(), collector)
        .map_err(|e| format!("capture: {e:?}"))?
        .ok_or("expected pagination extra")?;
    let mut recovered = fresh_states();
    recovered
        .hydrate_extra(run.run_id(), collector, &extra)
        .map_err(|e| format!("hydrate: {e:?}"))?;

    collect_next(&mut run, &mut store, &mut recovered, collector, body, done)
        .map_err(|e| format!("collect_next: {e:?}"))?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);
    Ok(())
}

/// Corrupt durable pagination extra is rejected instead of silently losing cursor state.
#[test]
fn collect_pagination_extra_rejects_corrupt_bytes() -> Result<(), String> {
    let mut states = fresh_states();
    let result = states.hydrate_extra(RunId::new(1), SlotIdx::new(1), &[255, 0, 7]);
    assert_invalid_workflow_reason(result, "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_journal_extra_rejects_corrupt_bytes() -> Result<(), String> {
    let event = slot_written_extra(RunId::new(1), SlotIdx::new(1), vec![255, 0, 7]);
    let mut states = fresh_states();
    let result = states.hydrate_journal_events(&[event]);
    assert_invalid_workflow_reason(result, "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_rejects_corrupt_bytes() -> Result<(), String> {
    let dir = disk_backed_temp_dir("collect-corrupt-")?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
    let run = RunId::new(9);
    journal
        .append_strict(&slot_written_extra(run, SlotIdx::new(1), vec![255, 0, 7]))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .map_err(|e| format!("recover: {e:?}"))?;
    let result = hydrate_collect_states_from_recovered_journal(&recovered);
    assert_invalid_workflow_reason(result.map(|_| ()), "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page()
-> Result<(), String> {
    let dir = disk_backed_temp_dir("collect-round-trip-")?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
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
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
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
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let extra = states
        .capture_extra(run.run_id(), collector)
        .map_err(|e| format!("capture: {e:?}"))?
        .ok_or("expected pagination extra")?;
    journal
        .append_strict(&slot_written_extra(run.run_id(), collector, extra))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered = recover_full_journal(&journal, run.run_id(), &mut tracker, &[], &[])
        .map_err(|e| format!("recover: {e:?}"))?;
    let mut hydrated = hydrate_collect_states_from_recovered_journal(&recovered)
        .map_err(|e| format!("hydrate: {e:?}"))?;
    let hydrated_state = hydrated
        .capture_state(run.run_id(), collector)
        .ok_or("expected hydrated pagination state")?;
    assert_eq!(hydrated_state.cursor, 1);
    assert_eq!(hydrated_state.page_size, 1);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);

    collect_next(&mut run, &mut store, &mut hydrated, collector, body, done)
        .map_err(|e| format!("collect_next: {e:?}"))?;

    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);
    let resumed_state = hydrated
        .capture_state(run.run_id(), collector)
        .ok_or("expected resumed pagination state")?;
    assert_eq!(resumed_state.cursor, 2);
    assert_eq!(resumed_state.page_size, 1);
    Ok(())
}

/// Hydration must not accept extras from another run or collector slot.
#[test]
fn collect_pagination_extra_rejects_identity_mismatch() -> Result<(), String> {
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let mut recovered = fresh_states();
    let result = recovered.hydrate_extra(RunId::new(2), collector, &extra);
    assert_invalid_workflow_reason(result, "collect pagination state identity mismatch");
    Ok(())
}

#[test]
fn collect_journal_extra_rejects_identity_mismatch() -> Result<(), String> {
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let event = slot_written_extra(RunId::new(2), collector, extra);
    let mut recovered = fresh_states();
    let result = recovered.hydrate_journal_events(&[event]);
    assert_invalid_workflow_reason(result, "collect pagination state identity mismatch");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_rejects_identity_mismatch() -> Result<(), String> {
    let dir = disk_backed_temp_dir("collect-identity-")?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let durable_run = RunId::new(2);
    journal
        .append_strict(&slot_written_extra(durable_run, collector, extra))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered = recover_full_journal(&journal, durable_run, &mut tracker, &[], &[])
        .map_err(|e| format!("recover: {e:?}"))?;
    let result = hydrate_collect_states_from_recovered_journal(&recovered);
    assert_invalid_workflow_reason(
        result.map(|_| ()),
        "collect pagination state identity mismatch",
    );
    Ok(())
}

/// CollectFinish propagates list values to the output slot.
#[test]
fn collect_finish_propagates_list_to_output() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);

    let items: Box<[SlotValue]> = vec![SlotValue::I64(7), SlotValue::I64(8)].into_boxed_slice();
    let list_id = store.insert_list(items).map_err(|e| format!("{e:?}"))?;
    run.write_slot(collector, SlotValue::List(list_id))
        .map_err(|e| format!("{e:?}"))?;

    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("collect_finish: {e:?}"))?;

    match *run.read_slot(output).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => {
            ensure(id == list_id, "output list id should match collector")?;
        }
        other => return Err(format!("expected List, got {other:?}")),
    }
    ensure(
        run.pc() == next,
        format!("expected pc={next:?}, got {:?}", run.pc()),
    )?;
    ensure(
        states.find(run.run_id(), collector, list_id).is_none(),
        "state should be removed after finish",
    )
}

/// CollectPage on a non-empty list at an arbitrary step advances to body.
#[test]
fn collect_page_with_nonempty_collector_advances_to_body() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(5);

    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        body,
        StepIdx::new(6),
    );
    let signal = result.map_err(|e| format!("{e:?}"))?;
    ensure(
        signal == vb_core::EngineSignal::Continue,
        "expected Continue",
    )?;
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )
}

/// Limit boundary: exact match succeeds, one over fails.
#[test]
fn collect_start_limit_boundary_exact_vs_one_over() -> Result<(), String> {
    let mut run1 = fresh_frame();
    let mut store1 = ValueStore::new();
    let mut states1 = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run1,
        &mut store1,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let r1 = collect_start(
        &mut run1,
        &mut store1,
        &mut states1,
        source,
        5,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    ensure(r1.is_ok(), format!("exact limit should succeed: {r1:?}"))?;

    let mut run2 = fresh_frame();
    let mut store2 = ValueStore::new();
    let mut states2 = fresh_states();
    list_in_slot(
        &mut run2,
        &mut store2,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let r2 = collect_start(
        &mut run2,
        &mut store2,
        &mut states2,
        source,
        4,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    match r2 {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

/// CollectNext with expired time_limit returns CollectTimeLimitExceeded.
#[test]
fn collect_next_time_limit_exceeded_returns_error() -> Result<(), String> {
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
        Some(1),
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;

    // Corrupt start_millis to 0 to guarantee expiry
    let current_page = match *run.read_slot(collector).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    let state = states
        .find(run.run_id(), collector, current_page)
        .ok_or("state not found")?;
    states
        .upsert(CollectPaginationState {
            start_millis: 0,
            ..state
        })
        .map_err(|e| format!("{e:?}"))?;

    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    match result {
        Err(EngineError::CollectTimeLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectTimeLimitExceeded, got {other:?}")),
    }
}

/// Two entries with different run IDs do not collide.
#[test]
fn collect_states_independent_entries_per_run() -> Result<(), String> {
    let mut states = CollectStates::new();
    let s1 = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 3,
        page_size: 5,
        item_count: 10,
        limit: 50,
        time_limit_ms: None,
        start_millis: 0,
    };
    let s2 = CollectPaginationState {
        run_id: RunId::new(2),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(11),
        current_page: ListId::new(21),
        cursor: 7,
        page_size: 5,
        item_count: 15,
        limit: 50,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(s1).map_err(|e| format!("{e:?}"))?;
    states.upsert(s2).map_err(|e| format!("{e:?}"))?;
    let f1 = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
        .ok_or("run 1 state missing")?;
    let f2 = states
        .find(RunId::new(2), SlotIdx::new(0), ListId::new(21))
        .ok_or("run 2 state missing")?;
    ensure(
        f1.cursor == 3,
        format!("cursor 1 should be 3, got {}", f1.cursor),
    )?;
    ensure(
        f2.cursor == 7,
        format!("cursor 2 should be 7, got {}", f2.cursor),
    )
}

/// Remove on a non-existent key is a silent no-op.
#[test]
fn collect_states_remove_nonexistent_is_noop() -> Result<(), String> {
    let mut states = CollectStates::new();
    states.remove(RunId::new(999), SlotIdx::new(99));
    ensure(
        states.entries.is_empty(),
        "removing nonexistent key should not add entries",
    )
}

/// copy_page_range with start at end of items returns empty page.
#[test]
fn copy_page_range_at_end_returns_empty() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let page = copy_page_range(&items, 2, 5).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.is_empty(),
        format!("expected empty page, got {} items", page.len()),
    )
}

/// copy_page_range with start beyond length returns error.
#[test]
fn copy_page_range_start_beyond_length_returns_error() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1)].into_boxed_slice();
    let result = copy_page_range(&items, 5, 1);
    match result {
        Err(EngineError::InternalInvariantViolation { reason }) => ensure(
            reason.as_str() == "collect cursor beyond item count",
            format!("unexpected: {reason}"),
        ),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

/// copy_prefix with zero-length items returns empty page.
#[test]
fn copy_prefix_empty_items_returns_empty() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![].into_boxed_slice();
    let page = copy_prefix(&items, 10).map_err(|e| format!("{e:?}"))?;
    ensure(page.is_empty(), "empty items should produce empty prefix")
}

/// CollectFinish propagates taint from collector to output.
#[test]
fn collect_finish_propagates_taint() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);

    run.write_slot_with_taint(collector, SlotValue::I64(42), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("{e:?}"))?;

    let taint = run.read_taint(output).map_err(|e| format!("{e:?}"))?;
    ensure(
        taint == Taint::Secret,
        format!("expected Secret, got {taint:?}"),
    )
}

/// CollectNext where cursor exactly equals item_count goes to done.
#[test]
fn collect_next_cursor_at_item_count_goes_to_done() -> Result<(), String> {
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
    .map_err(|e| format!("{e:?}"))?;

    // collect_start routes to body for non-empty list, retaining state.
    assert_eq!(run.pc(), body);
    // collect_next sees cursor at item_count and routes to done.
    let next_result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(next_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    Ok(())
}
