#[test]
fn collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state()
-> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![SlotValue::I64(11), SlotValue::I64(22), SlotValue::I64(33)],
        3,
        1,
    )?;
    let page1 = scenario.current_page()?;
    let unrelated_before_next = scenario
        .store
        .insert_list(Box::from([SlotValue::I64(900)]))
        .map_err(|e| format!("insert unrelated list before next: {e:?}"))?;

    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    let page2 = scenario.current_page()?;
    let unrelated_after_next = scenario
        .store
        .insert_list(Box::from([SlotValue::I64(901)]))
        .map_err(|e| format!("insert unrelated list after next: {e:?}"))?;
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
        unrelated_before_next,
        ListId::new(
            page1
                .get()
                .checked_add(1)
                .ok_or("page1 list id overflow".to_owned())?
        )
    );
    assert_eq!(
        page2,
        ListId::new(
            unrelated_before_next
                .get()
                .checked_add(1)
                .ok_or("unrelated list id overflow".to_owned())?
        )
    );
    assert_eq!(
        unrelated_after_next,
        ListId::new(
            page2
                .get()
                .checked_add(1)
                .ok_or("page2 list id overflow".to_owned())?
        )
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
    assert_slot_list_items(
        &scenario.run,
        &scenario.store,
        SlotIdx::new(0),
        &[SlotValue::I64(11), SlotValue::I64(22), SlotValue::I64(33)],
    );
    Ok(())
}

#[test]
fn collect_next_stale_page_returns_order_violation_stale_and_preserves_state() -> Result<(), String>
{
    let mut scenario = CollectScenario::start(
        vec![SlotValue::I64(7), SlotValue::I64(8), SlotValue::I64(9)],
        3,
        1,
    )?;
    let page1 = scenario.current_page()?;
    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    let page3 = scenario.current_page()?;
    let before = scenario
        .states
        .capture_state(scenario.run.run_id(), scenario.collector)
        .ok_or("expected live collect state before stale rejection".to_owned())?;

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
            kind: CollectPageOrderViolationKind::Stale,
            run_id: scenario.run.run_id(),
            collector_slot: scenario.collector,
            expected_page: page3,
            observed_page: page1,
        })
    );
    assert_eq!(
        scenario
            .states
            .capture_state(scenario.run.run_id(), scenario.collector),
        Some(before)
    );
    Ok(())
}

#[test]
fn collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state()
-> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        3,
        1,
    )?;
    let expected_page = scenario.current_page()?;
    let before = scenario
        .states
        .capture_state(scenario.run.run_id(), scenario.collector)
        .ok_or("expected live collect state before future-page rejection".to_owned())?;
    let future_page = scenario
        .store
        .insert_list(Box::from([SlotValue::I64(99)]))
        .map_err(|e| format!("insert future page: {e:?}"))?;

    scenario.write_collector_page(future_page)?;
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
            kind: CollectPageOrderViolationKind::OutOfOrder,
            run_id: scenario.run.run_id(),
            collector_slot: scenario.collector,
            expected_page,
            observed_page: future_page,
        })
    );
    assert_eq!(
        scenario
            .states
            .capture_state(scenario.run.run_id(), scenario.collector),
        Some(before)
    );
    assert_slot_list_items(
        &scenario.run,
        &scenario.store,
        SlotIdx::new(0),
        &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    Ok(())
}

#[test]
fn collect_hydration_empty_extra_returns_empty_extra_error_and_no_state() {
    let run_id = RunId::new(3901);
    let collector = SlotIdx::new(1);
    let mut states = CollectStates::new();

    assert_eq!(
        states.hydrate_extra(run_id, collector, &[]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::EmptyExtra,
            run_id,
            collector_slot: collector,
            event_seq: None,
        })
    );
    assert_eq!(states.capture_state(run_id, collector), None);
}

#[test]
fn collect_hydration_corrupt_extra_returns_decode_failed_and_no_state() {
    let run_id = RunId::new(3902);
    let collector = SlotIdx::new(1);
    let mut states = CollectStates::new();

    assert_eq!(
        states.hydrate_extra(run_id, collector, &[0xFF, 0x00, 0x13]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::DecodeFailed,
            run_id,
            collector_slot: collector,
            event_seq: None,
        })
    );
    assert_eq!(states.capture_state(run_id, collector), None);
}

#[test]
fn recovered_collect_state_rejects_run_mismatch_and_inserts_no_state() -> Result<(), String> {
    let durable_run = RunId::new(3801);
    let encoded_run = RunId::new(9999);
    let collector = SlotIdx::new(1);
    let state = qi37_3_valid_state(encoded_run, collector, ListId::new(10), ListId::new(20));
    let extra = postcard::to_allocvec(&state).map_err(|e| format!("encode state: {e:?}"))?;
    let event = JournalEvent::SlotWrittenEvent {
        run: durable_run,
        seq: EventSeq::new(3),
        slot: collector,
        value: None,
        extra: Some(extra),
        attempt: 1,
    };

    assert_eq!(
        hydrate_collect_states_from_recovered_journal(&[event]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::RunMismatch {
                expected: durable_run,
                actual: encoded_run,
            },
            run_id: durable_run,
            collector_slot: collector,
            event_seq: Some(CoreEventSeq::new(3)),
        })
    );
    assert_eq!(
        CollectStates::new().capture_state(durable_run, collector),
        None
    );
    Ok(())
}

#[test]
fn recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state() -> Result<(), String> {
    let run_id = RunId::new(3801);
    let event_slot = SlotIdx::new(1);
    let encoded_slot = SlotIdx::new(2);
    let state = qi37_3_valid_state(run_id, encoded_slot, ListId::new(10), ListId::new(20));
    let extra = postcard::to_allocvec(&state).map_err(|e| format!("encode state: {e:?}"))?;
    let event = JournalEvent::SlotWrittenEvent {
        run: run_id,
        seq: EventSeq::new(4),
        slot: event_slot,
        value: None,
        extra: Some(extra),
        attempt: 1,
    };

    assert_eq!(
        hydrate_collect_states_from_recovered_journal(&[event]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::SlotMismatch {
                expected: event_slot,
                actual: encoded_slot,
            },
            run_id,
            collector_slot: event_slot,
            event_seq: Some(CoreEventSeq::new(4)),
        })
    );
    assert_eq!(CollectStates::new().capture_state(run_id, event_slot), None);
    Ok(())
}

#[test]
fn collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state() -> Result<(), String>
{
    let run_id = RunId::new(3802);
    let collector = SlotIdx::new(1);
    let expected_page = ListId::new(21);
    let actual_page = ListId::new(20);
    let state = qi37_3_valid_state(run_id, collector, ListId::new(10), actual_page);
    let extra = postcard::to_allocvec(&state).map_err(|e| format!("encode state: {e:?}"))?;
    let encoded_value = postcard::to_allocvec(&SlotValue::List(expected_page))
        .map_err(|e| format!("encode slot value: {e:?}"))?;
    let event = JournalEvent::SlotWrittenEvent {
        run: run_id,
        seq: EventSeq::new(5),
        slot: collector,
        value: Some(encoded_value),
        extra: Some(extra),
        attempt: 1,
    };
    let mut states = CollectStates::new();

    assert_eq!(
        states.hydrate_journal_events(&[event]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::CurrentPageMismatch {
                expected: expected_page,
                actual: actual_page,
            },
            run_id,
            collector_slot: collector,
            event_seq: Some(CoreEventSeq::new(5)),
        })
    );
    assert_eq!(states.capture_state(run_id, collector), None);
    Ok(())
}

#[test]
fn collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state()
-> Result<(), String> {
    let run_id = RunId::new(3803);
    let collector = SlotIdx::new(1);
    let state = qi37_3_valid_state(run_id, collector, ListId::new(10), ListId::new(20));
    let extra = postcard::to_allocvec(&state).map_err(|e| format!("encode state: {e:?}"))?;
    let event = JournalEvent::SlotWrittenEvent {
        run: run_id,
        seq: EventSeq::new(6),
        slot: collector,
        value: Some(vec![0xFF, 0x00, 0x13]),
        extra: Some(extra),
        attempt: 1,
    };
    let mut states = CollectStates::new();

    assert_eq!(
        states.hydrate_journal_events(&[event]),
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::DecodeFailed,
            run_id,
            collector_slot: collector,
            event_seq: Some(CoreEventSeq::new(6)),
        })
    );
    assert_eq!(states.capture_state(run_id, collector), None);
    Ok(())
}

// =========================================================================
// vb-hbav B01-B09: collect_page behavior hardening tests.
// Sharp assertions on exact error variants, PC advancement, signal values,
// empty/boundary/heterogeneous inputs, and collector slot boundaries.
// =========================================================================

#[test]
fn collect_page_returns_continue_signal_on_success() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

#[test]
fn collect_page_returns_type_mismatch_when_collector_is_bool() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::Bool(true))
        .expect("write must succeed");
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "list",
            found: "boolean"
        })
    );
}

#[test]
fn collect_page_returns_type_mismatch_when_collector_is_null() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::Null)
        .expect("write must succeed");
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "list",
            found: "null"
        })
    );
}

#[test]
fn collect_page_handles_empty_list_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, collector, vec![]);
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_page_works_with_collector_slot_at_zero() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::ZERO;
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(42)]);
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_page_does_not_panic_with_max_slot_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(7);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(99)]);
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_page_succeeds_with_heterogeneous_list_items() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![
            SlotValue::I64(1),
            SlotValue::Bool(true),
            SlotValue::Null,
            SlotValue::I64(2),
        ],
    );
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
}

#[test]
fn collect_total_items_consumed_survives_completion() -> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
        5,
        2,
    )?;
    assert_eq!(scenario.states.total_items_consumed(), 2);
    scenario.next()?;
    assert_eq!(scenario.states.total_items_consumed(), 4);
    scenario.next()?;
    assert_eq!(scenario.states.total_items_consumed(), 5);
    scenario.next()?;
    assert_eq!(
        scenario
            .states
            .capture_state(scenario.run.run_id(), scenario.collector),
        None
    );
    assert_eq!(scenario.states.total_items_consumed(), 5);
    Ok(())
}

#[test]
fn collect_total_items_consumed_ignores_duplicate_upsert() -> Result<(), String> {
    let mut scenario = CollectScenario::start(vec![SlotValue::I64(1), SlotValue::I64(2)], 2, 2)?;
    assert_eq!(scenario.states.total_items_consumed(), 2);
    let page = scenario.current_page()?;
    let live = scenario
        .states
        .find(scenario.run.run_id(), scenario.collector, page)
        .ok_or("live state missing for re-upsert".to_owned())?;
    scenario
        .states
        .upsert(live)
        .map_err(|error| format!("re-upsert: {error:?}"))?;
    assert_eq!(scenario.states.total_items_consumed(), 2);
    Ok(())
}

#[test]
fn collect_total_pages_delivered_survives_completion() -> Result<(), String> {
    let mut scenario = CollectScenario::start(
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
        5,
        2,
    )?;
    assert_eq!(scenario.states.total_pages_delivered(), 1);
    scenario.next()?;
    assert_eq!(scenario.states.total_pages_delivered(), 2);
    scenario.next()?;
    assert_eq!(scenario.states.total_pages_delivered(), 3);
    scenario.next()?;
    assert_eq!(
        scenario
            .states
            .capture_state(scenario.run.run_id(), scenario.collector),
        None
    );
    assert_eq!(scenario.states.total_pages_delivered(), 3);
    Ok(())
}

#[test]
fn collect_total_pages_delivered_ignores_duplicate_upsert() -> Result<(), String> {
    let mut scenario = CollectScenario::start(vec![SlotValue::I64(1), SlotValue::I64(2)], 2, 2)?;
    assert_eq!(scenario.states.total_pages_delivered(), 1);
    let page = scenario.current_page()?;
    let live = scenario
        .states
        .find(scenario.run.run_id(), scenario.collector, page)
        .ok_or("live state missing for re-upsert".to_owned())?;
    scenario
        .states
        .upsert(live)
        .map_err(|error| format!("re-upsert: {error:?}"))?;
    assert_eq!(scenario.states.total_pages_delivered(), 1);
    Ok(())
}
