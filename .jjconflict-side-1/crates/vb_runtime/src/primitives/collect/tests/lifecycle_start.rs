#[test]
fn collect_start_initializes_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
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
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    let slot_val = *run.read_slot(output).expect("read must succeed");
    assert!(matches!(slot_val, SlotValue::List(_)));
}

#[test]
fn collect_page_increments_page_count() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(10)]);
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_next_advances_to_next_page_while_page_has_items() {
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
        vec![SlotValue::I64(5), SlotValue::I64(6), SlotValue::I64(7)],
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
        &[SlotValue::I64(5), SlotValue::I64(6)],
    );
    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(7)]);
}

#[test]
fn collect_finish_materializes_output() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(collector, SlotValue::I64(99))
        .expect("slot write must succeed");
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output).expect("read must succeed"),
        SlotValue::I64(99)
    );
}

#[test]
fn collect_start_returns_error_when_source_is_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    run.write_slot(source, SlotValue::Bool(true))
        .expect("write must succeed");
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
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_returns_error_when_limit_exceeded() {
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
            SlotValue::I64(5),
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
fn collect_start_returns_error_when_output_missing() {
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
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        None,
        None,
    );
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_returns_error_when_page_size_zero() {
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
        0,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "collect page_size must be nonzero");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_jumps_to_done_when_source_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        done,
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_next_returns_done_when_remaining_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, collector, vec![]);
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        done,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_finish_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::I64(1))
        .expect("write must succeed");
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        None,
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_finish_returns_error_when_next_missing() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    run.write_slot(collector, SlotValue::I64(1))
        .expect("write must succeed");
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        None,
        StepIdx::ZERO,
    );
    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_page_returns_error_when_collector_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::I64(42))
        .expect("write must succeed");
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_writes_first_page_to_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
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
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run.read_slot(output).expect("read must succeed") {
        SlotValue::List(id) => {
            let items = store.list(id).expect("list read must succeed");
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
fn collect_start_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let before = run.executed();
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
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_next_increments_executed_with_pagination_state() {
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
    let start = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(collector),
        None,
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    let before = run.executed();
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_page_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
    let before = run.executed();
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_finish_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    run.write_slot(collector, SlotValue::I64(99))
        .expect("write must succeed");
    let before = run.executed();
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_next_rejects_nonempty_current_page_without_state() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect pagination state missing",
        })
    );
    assert_eq!(run.pc(), StepIdx::ZERO);
}

#[test]
fn collect_next_returns_error_when_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::Bool(true))
        .expect("write must succeed");
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_zero_items_with_nonzero_limit_goes_to_done() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        done,
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_start_page_size_zero_returns_error_even_for_empty_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        0,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        })
    );
}
