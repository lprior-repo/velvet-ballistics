#[test]
fn for_each_start_returns_continue_when_list_has_items() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let _done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(42), SlotValue::I64(99)],
    );

    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        FanoutLimit::new(100),
        body,
        StepIdx::new(2),
        Some(output_slot),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(42)
    );
}

#[test]
fn for_each_start_returns_done_when_list_is_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, input, vec![]);

    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        FanoutLimit::new(100),
        body,
        done,
        Some(output_slot),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn for_each_next_returns_continue_while_items_remain() {
    // Given a frame with a 2-item source list, after for_each_start the
    // iterator slot holds a (source_id, cursor=1) state. for_each_next
    // advances the cursor to 1 and binds the second item.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iter_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(7), SlotValue::I64(8)],
    );
    let start = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        16,
        body,
        done,
        Some(iter_slot),
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    // The state list now encodes (source_id, cursor=1).

    let result = for_each_next(&mut run, &mut store, iter_slot, body, done, Some(item_slot));

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(8)
    );
}

#[test]
fn for_each_join_returns_done_signal() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let next_step = StepIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        materialized_slot,
        vec![SlotValue::I64(42)],
    );

    let result = for_each_join(
        &mut run,
        materialized_slot,
        Some(output_slot),
        Some(next_step),
        StepIdx::ZERO,
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
}

#[test]
fn for_each_join_materializes_ordered_results() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        materialized_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    let result = for_each_join(
        &mut run,
        materialized_slot,
        Some(output_slot),
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let output_value = *run
        .read_slot(output_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"));
    let SlotValue::List(list_id) = output_value else {
        panic!("output must be list");
    };
    let items = store
        .list(list_id)
        .ok()
        .unwrap_or_else(|| panic!("list read must succeed"));
    assert_eq!(
        items,
        [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
    );
}

// BDD tests for for_each primitives

#[test]
fn for_each_start_returns_error_when_input_is_not_list() {
    // Given a frame with a non-list value in input slot
    let (mut run, mut store, input, item_slot, output_slot) = for_each_non_list_input_fixture();
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then it returns TypeMismatch error
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => {
            assert_eq!(other, expected_for_each_type_mismatch("number"));
        }
    }
}

#[test]
fn for_each_start_returns_error_when_limit_exceeded() {
    // Given a frame with a 3-item list and limit=2
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    // When calling for_each_start with limit=2
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        FanoutLimit::new(2),
        body,
        done,
        Some(output_slot),
    );
    // Then it returns IterationLimitExceeded
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => {
            assert_eq!(resource, "for_each_limit");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn for_each_start_returns_error_when_output_missing() {
    // Given a frame with a list in input but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    // When calling for_each_start with output=None
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        None,
    );
    // Then it returns MissingOutputSlot
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
fn for_each_start_binds_first_item_to_item_slot() {
    // Given a frame with a multi-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then item_slot has the first item
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(10)
    );
}

#[test]
fn for_each_start_writes_tail_to_output_slot() {
    // Given a frame with a 3-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then output_slot has a list (the tail)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(_) => {}
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_next_returns_done_when_tail_empty() {
    // Given a frame with an empty list in iterator slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![]);
    // When calling for_each_next
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        StepIdx::new(1),
        done,
        Some(output_slot),
    );
    // Then it jumps to done
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn for_each_next_returns_error_when_output_missing() -> Result<(), String> {
    // Given a frame with a real iterator state (source_id, cursor=0) but
    // no output slot.
    let (mut run, mut store, iterator_slot) = for_each_missing_output_fixture()?;
    // When calling for_each_next with output=None
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        StepIdx::new(1),
        StepIdx::new(2),
        None,
    );
    // Then it returns MissingOutputSlot
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, expected_for_each_missing_output());
        }
    }
    Ok(())
}

#[test]
fn for_each_next_returns_error_when_iterator_is_not_list() {
    // Given a frame with a non-list in iterator slot
    let (mut run, mut store, iterator_slot, output_slot) = for_each_non_list_iterator_fixture();
    // When calling for_each_next
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then it returns TypeMismatch
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, expected_for_each_type_mismatch("boolean"));
        }
    }
}

#[test]
fn for_each_join_returns_error_when_output_missing() {
    // Given a frame
    let mut run = fresh_frame();
    // When calling for_each_join with output=None
    let result = for_each_join(
        &mut run,
        SlotIdx::new(0),
        None,
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    // Then it returns MissingOutputSlot
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
fn for_each_join_returns_error_when_next_missing() {
    // Given a frame
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        materialized_slot,
        vec![SlotValue::I64(1)],
    );
    // When calling for_each_join with next=None
    let result = for_each_join(
        &mut run,
        materialized_slot,
        Some(output_slot),
        None,
        StepIdx::ZERO,
    );
    // Then it returns MissingNextStep
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
fn for_each_start_increments_executed_counter() {
    // Given a frame with a list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    let executed_before = run.executed();
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), executed_before + 1);
}

#[test]
fn for_each_next_increments_executed_counter() {
    // Given a frame with a real iterator state (source_id, cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1)],
        0,
    );
    let executed_before = run.executed();
    // When calling for_each_next
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), executed_before + 1);
}

#[test]
fn for_each_single_item_list_tail_is_empty() {
    // Given a frame with a single-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(42)]);
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then item_slot is the single item
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(42)
    );
    // And the iterator state in output_slot encodes (source_id, cursor=1).
    // RP-016: state is a bounded 2-element list, not a materialized tail.
    match *run
        .read_slot(output_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(state.len(), 2, "state must be a 2-element cursor list");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_next_writes_cursor_state_to_iterator_slot() {
    // Given a frame with a 3-item source encoded as iterator state (cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        0,
    );
    // When calling for_each_next
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then output has the first item and iterator state advances to cursor=1
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(1)
    );
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            // RP-016: state is always 2 elements regardless of source size
            assert_eq!(state.len(), 2);
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

// ── Adversarial BDD tests for for_each ──────────────────────────────

#[test]
fn for_each_start_limit_zero_allows_empty_list() {
    // Given a frame with an empty list and limit=0
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, input, vec![]);
    // When calling for_each_start with limit=0
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        0,
        StepIdx::new(1),
        done,
        Some(output_slot),
    );
    // Then it jumps to done (0 items <= 0 limit)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn for_each_start_limit_zero_rejects_single_item() {
    // Given a frame with a single-item list and limit=0
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    // When calling for_each_start with limit=0
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        0,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then it returns IterationLimitExceeded (1 > 0)
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => {
            assert_eq!(resource, "for_each_limit");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn for_each_start_limit_exactly_at_boundary_accepts() {
    // Given a frame with a 3-item list and limit=3
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    // When calling for_each_start with limit=3 (exactly the item count)
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        3,
        body,
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then it succeeds (3 <= 3)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn for_each_start_limit_exceeded_by_one_rejects() {
    // Given a frame with a 4-item list and limit=3
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
    );
    // When calling for_each_start with limit=3
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        3,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then it returns IterationLimitExceeded (4 > 3)
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => {
            assert_eq!(resource, "for_each_limit");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn for_each_start_item_slot_corruption_after_bind() {
    // Given a frame where item_slot is overwritten after for_each_start
    let (mut run, mut store, item_slot) = for_each_two_item_fixture();
    // When calling for_each_start
    let result = start_for_each_two_item_fixture(&mut run, &mut store);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // Then item_slot has the first item
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(10)
    );
    // When the item_slot is externally corrupted
    assert_eq!(
        run.write_slot(item_slot, SlotValue::I64(999)),
        Ok(()),
        "must write"
    );
    // Then the corruption persists (item_slot no longer has original value)
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(999)
    );
}

#[test]
fn for_each_start_empty_list_does_not_modify_item_slot() {
    // Given a frame with a pre-set item_slot and empty input list
    let (mut run, mut store, input, item_slot, output_slot, done) = for_each_empty_input_fixture();
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        StepIdx::new(1),
        done,
        Some(output_slot),
    );
    // Then it jumps to done and item_slot is unchanged (not bound to first item)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(999)
    );
}

#[test]
fn for_each_start_null_input_returns_type_mismatch() {
    // Given a frame with Null in the input slot
    let (mut run, mut store, input) = for_each_null_input_fixture();
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        SlotIdx::new(1),
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    );
    // Then it returns TypeMismatch (null is not a list)
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "null");
        }
        other => {
            assert_eq!(other, expected_for_each_type_mismatch("null"));
        }
    }
}

#[test]
fn for_each_next_output_slot_same_as_iterator_overwrite() {
    // Given a frame where output_slot == iterator_slot, with cursor state
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
        0,
    );
    // When calling for_each_next with output == iterator_slot
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        StepIdx::new(2),
        Some(iterator_slot),
    );
    // Then it succeeds -- output first writes item, then overwrites with
    // the next 2-element cursor state (cursor=1).
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must remain 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_start_drains_single_item_producing_empty_cursor() {
    // Given a frame with a single-item list
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(42)]);
    // When calling for_each_start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        100,
        body,
        StepIdx::new(2),
        Some(output_slot),
    );
    // Then item_slot has 42 and the iterator state has cursor=1 == source len,
    // so the next for_each_next call must jump to done.
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(42)
    );
    match *run
        .read_slot(output_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must remain 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_next_on_two_item_list_exhausts_after_one_call() {
    // Given a frame with a 2-item source encoded as iterator state (cursor=0)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    iterator_state_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(7), SlotValue::I64(8)],
        0,
    );
    // When calling for_each_next once
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(output_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    // The iterator state advanced to cursor=1
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            assert_eq!(state.len(), 2, "cursor state must remain 2 elements");
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(1)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
    // When calling for_each_next again, cursor advances to 2 (= source len)
    // and the function must jump to done.
    let result2 = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(output_slot),
    );
    // Then it still processes the last item and routes to done.
    assert_eq!(result2, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(state_id) => {
            let state = store
                .list(state_id)
                .ok()
                .unwrap_or_else(|| panic!("must read"));
            // After exhausting, cursor == source len (2). The state still
            // carries the 2-element format; for_each_next on it must jump
            // to done on the next call.
            assert_eq!(state.len(), 2);
            assert_eq!(state.get(1).copied(), Some(SlotValue::I64(2)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
    // When calling for_each_next a third time on the exhausted cursor
    let result3 = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(output_slot),
    );
    // Then it jumps to done
    assert_eq!(result3, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

fn for_each_non_list_input_fixture() -> (RunFrame, ValueStore, SlotIdx, SlotIdx, SlotIdx) {
    let mut run = fresh_frame();
    let store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    assert_eq!(
        run.write_slot(input, SlotValue::I64(42)),
        Ok(()),
        "write must succeed"
    );
    (run, store, input, item_slot, output_slot)
}

fn for_each_missing_output_fixture() -> Result<(RunFrame, ValueStore, SlotIdx), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let source_id = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| "insert source".to_owned())?;
    let state_id = store
        .insert_list(
            vec![
                SlotValue::I64(i64::from(source_id.get())),
                SlotValue::I64(0),
            ]
            .into_boxed_slice(),
        )
        .map_err(|_| "insert state".to_owned())?;
    assert_eq!(
        run.write_slot(iterator_slot, SlotValue::List(state_id)),
        Ok(()),
        "write state"
    );
    Ok((run, store, iterator_slot))
}

fn for_each_non_list_iterator_fixture() -> (RunFrame, ValueStore, SlotIdx, SlotIdx) {
    let mut run = fresh_frame();
    let store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    assert_eq!(
        run.write_slot(iterator_slot, SlotValue::Bool(true)),
        Ok(()),
        "write must succeed"
    );
    (run, store, iterator_slot, output_slot)
}

fn for_each_two_item_fixture() -> (RunFrame, ValueStore, SlotIdx) {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    (run, store, item_slot)
}

fn start_for_each_two_item_fixture(
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<vb_core::EngineSignal, EngineError> {
    for_each_start(
        run,
        store,
        SlotIdx::new(0),
        SlotIdx::new(1),
        100,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(2)),
    )
}

fn for_each_empty_input_fixture() -> (RunFrame, ValueStore, SlotIdx, SlotIdx, SlotIdx, StepIdx) {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let done = StepIdx::new(3);
    assert_eq!(
        run.write_slot(item_slot, SlotValue::I64(999)),
        Ok(()),
        "must write"
    );
    list_in_slot(&mut run, &mut store, input, vec![]);
    (run, store, input, item_slot, output_slot, done)
}

fn expected_for_each_type_mismatch(
    found: &'static str,
) -> Result<vb_core::EngineSignal, EngineError> {
    Err(EngineError::TypeMismatch {
        expected: "list",
        found,
    })
}

fn expected_for_each_missing_output() -> Result<vb_core::EngineSignal, EngineError> {
    Err(EngineError::MissingOutputSlot {
        step: StepIdx::ZERO,
    })
}

fn for_each_null_input_fixture() -> (RunFrame, ValueStore, SlotIdx) {
    let mut run = fresh_frame();
    let store = ValueStore::new();
    let input = SlotIdx::new(0);
    assert_eq!(run.write_slot(input, SlotValue::Null), Ok(()), "must write");
    (run, store, input)
}
