use super::*;
use crate::test_harness::list_in_slot;
use vb_core::value_store::ValueStore;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

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
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(7), SlotValue::I64(8)],
    );

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
    assert_eq!(
        *run.read_slot(output_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(7)
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
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    run.write_slot(input, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
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
fn for_each_next_returns_error_when_output_missing() {
    // Given a frame with items but no output slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn for_each_next_returns_error_when_iterator_is_not_list() {
    // Given a frame with a non-list in iterator slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    run.write_slot(iterator_slot, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
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
    // Given a frame with items in iterator
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
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
    // And the tail (output slot) is an empty list
    match *run
        .read_slot(output_slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items.len(), 0);
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_next_writes_tail_to_iterator_slot() {
    // Given a frame with 3-item iterator
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
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
    // Then output has first item and iterator has tail
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
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items.len(), 2);
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
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
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
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // Then item_slot has the first item
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(10)
    );
    // When the item_slot is externally corrupted
    run.write_slot(item_slot, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("must write"));
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
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let done = StepIdx::new(3);
    run.write_slot(item_slot, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("must write"));
    list_in_slot(&mut run, &mut store, input, vec![]);
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
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    run.write_slot(input, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("must write"));
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
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn for_each_next_output_slot_same_as_iterator_overwrite() {
    // Given a frame where output_slot == iterator_slot
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
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
    // Then it succeeds -- output first writes item, then overwrites with tail
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    // The iterator_slot should hold the tail list (overwrite semantics)
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            // Tail of [1, 2] is [2], so len == 1
            assert_eq!(items.len(), 1);
        }
        other => {
            // This branch should not be reached; fail the test
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_start_drains_single_item_producing_empty_tail() {
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
    // Then item_slot has 42 and tail is empty
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
        SlotValue::List(id) => {
            assert_eq!(
                store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("must read"))
                    .len(),
                0
            );
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn for_each_next_on_two_item_list_exhausts_after_one_call() {
    // Given a frame with a 2-item iterator
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        iterator_slot,
        vec![SlotValue::I64(7), SlotValue::I64(8)],
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
    // The iterator now has 1 remaining item
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            assert_eq!(
                store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("must read"))
                    .len(),
                1
            );
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
    // When calling for_each_next again on the 1-item tail
    let result2 = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(output_slot),
    );
    // Then it still processes the last item and leaves an empty tail
    assert_eq!(result2, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    match *run
        .read_slot(iterator_slot)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            assert_eq!(
                store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("must read"))
                    .len(),
                0
            );
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
    // When calling for_each_next a third time on empty tail
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

// =====================================================================
// Phase 22: For_each bounded iteration tests
// =====================================================================

/// Helper: default fanout limit matching ResourceContract::DEFAULT.max_fanout
const DEFAULT_FANOUT: u32 = 64;

// ── Verification 1: for_each respects fanout limits from ResourceContract ──

#[test]
fn phase22_for_each_start_accepts_list_at_default_fanout() {
    // Given a list with exactly DEFAULT_FANOUT items (ResourceContract default max_fanout)
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let items: Vec<SlotValue> = (0..DEFAULT_FANOUT)
        .map(|i| SlotValue::I64(i64::try_from(i).unwrap_or(0)))
        .collect();
    list_in_slot(&mut run, &mut store, input, items);

    // When calling for_each_start with limit = DEFAULT_FANOUT
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        DEFAULT_FANOUT,
        body,
        StepIdx::new(2),
        Some(output_slot),
    );

    // Then it succeeds (item_count == limit)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    // And the first item is bound correctly
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(0)
    );
}

#[test]
fn phase22_for_each_start_rejects_list_exceeding_default_fanout() {
    // Given a list with DEFAULT_FANOUT + 1 items
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let count = usize::try_from(DEFAULT_FANOUT)
        .ok()
        .map(|n| n.checked_add(1))
        .flatten()
        .unwrap_or(DEFAULT_FANOUT as usize + 1);
    let items: Vec<SlotValue> = (0..count)
        .map(|i| SlotValue::I64(i64::try_from(i).unwrap_or(0)))
        .collect();
    list_in_slot(&mut run, &mut store, input, items);

    // When calling for_each_start with limit = DEFAULT_FANOUT
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        DEFAULT_FANOUT,
        StepIdx::new(1),
        StepIdx::new(2),
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
fn phase22_for_each_start_custom_fanout_limit_enforced() {
    // Given a list with 10 items and a custom fanout limit of 8
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let items: Vec<SlotValue> = (0..10).map(SlotValue::I64).collect();
    list_in_slot(&mut run, &mut store, input, items);

    // When calling for_each_start with limit=8
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        8,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );

    // Then it returns IterationLimitExceeded (10 > 8)
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => {
            assert_eq!(resource, "for_each_limit");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

// ── Verification 2: item count bounds -- total items cannot exceed max_fanout ──

#[test]
fn phase22_for_each_start_empty_list_respects_zero_bound() {
    // Given an empty list and fanout limit of 0
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

    // Then it succeeds (0 items <= 0 limit) and jumps to done
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn phase22_for_each_start_single_item_within_bound() {
    // Given a single-item list and fanout limit of 1
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(42)]);

    // When calling for_each_start with limit=1
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        1,
        body,
        StepIdx::new(2),
        Some(output_slot),
    );

    // Then it succeeds (1 <= 1) and binds the item
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
fn phase22_for_each_start_two_items_exceeds_limit_one() {
    // Given a two-item list and fanout limit of 1
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        input,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    // When calling for_each_start with limit=1
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output_slot),
    );

    // Then it returns IterationLimitExceeded (2 > 1)
    assert!(
        matches!(result, Err(EngineError::IterationLimitExceeded { resource }) if resource == "for_each_limit"),
        "expected IterationLimitExceeded, got {result:?}"
    );
}

// ── Verification 3: at_once mode -- all items validated up front before processing ──

#[test]
fn phase22_for_each_start_validates_all_items_before_binding() {
    // Given a list with 5 items but limit=3: no item should be bound
    // because the entire list is validated before any processing.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    // Pre-set item_slot to a sentinel value to verify it's not overwritten
    run.write_slot(item_slot, SlotValue::I64(999))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let items: Vec<SlotValue> = (0..5).map(SlotValue::I64).collect();
    list_in_slot(&mut run, &mut store, input, items);

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

    // Then it returns IterationLimitExceeded (all 5 validated up front)
    assert!(
        matches!(result, Err(EngineError::IterationLimitExceeded { resource }) if resource == "for_each_limit"),
        "expected IterationLimitExceeded, got {result:?}"
    );
    // And item_slot retains its sentinel value (never overwritten by binding)
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(999)
    );
}

#[test]
fn phase22_for_each_start_at_once_all_items_within_bound() {
    // Given a list with 5 items and limit=5: all items are validated
    // simultaneously and the first is bound.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let items: Vec<SlotValue> = (100..105).map(SlotValue::I64).collect();
    list_in_slot(&mut run, &mut store, input, items);

    // When calling for_each_start with limit=5
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        5,
        body,
        StepIdx::new(2),
        Some(output_slot),
    );

    // Then it succeeds and the first item (100) is bound
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(100)
    );
}

// ── Verification 4: ordered output -- result list preserves item order ──

#[test]
fn phase22_for_each_full_iteration_preserves_order() {
    // Given a 5-item list, simulate the full iteration lifecycle
    // to verify the output list preserves insertion order.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Create ordered items [10, 20, 30, 40, 50]
    let original: Vec<SlotValue> = vec![
        SlotValue::I64(10),
        SlotValue::I64(20),
        SlotValue::I64(30),
        SlotValue::I64(40),
        SlotValue::I64(50),
    ];
    list_in_slot(&mut run, &mut store, input, original.clone());

    // Step 1: for_each_start -- binds first item, tail in iterator
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        5,
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(10)
    );

    // Collect items in order as the loop would bind them
    let mut collected: Vec<SlotValue> = vec![
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
    ];

    // Step 2-N: for_each_next -- advance through remaining items
    for expected_value in [20i64, 30, 40, 50] {
        let result = for_each_next(
            &mut run,
            &mut store,
            iterator_slot,
            body,
            done,
            Some(item_slot),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        let bound = *run
            .read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"));
        assert_eq!(bound, SlotValue::I64(expected_value));
        collected.push(bound);
    }

    // Final call: for_each_next on empty tail jumps to done
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);

    // Verify collected items match original order
    assert_eq!(collected, original);
}

#[test]
fn phase22_for_each_join_preserves_list_order() {
    // Given a materialized list [1, 2, 3] in the materialized slot,
    // for_each_join must preserve the exact order in the output.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let next_step = StepIdx::new(1);
    let ordered: Vec<SlotValue> = vec![
        SlotValue::I64(100),
        SlotValue::I64(200),
        SlotValue::I64(300),
        SlotValue::I64(400),
        SlotValue::I64(500),
    ];
    list_in_slot(&mut run, &mut store, materialized_slot, ordered.clone());

    let result = for_each_join(
        &mut run,
        materialized_slot,
        Some(output_slot),
        Some(next_step),
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
    assert_eq!(items, ordered.as_slice());
}

#[test]
fn phase22_for_each_preserves_order_with_varied_types() {
    // Given a list with mixed types, verify order is preserved through
    // a full iteration cycle.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    let original: Vec<SlotValue> = vec![
        SlotValue::I64(1),
        SlotValue::Bool(true),
        SlotValue::Null,
        SlotValue::I64(4),
    ];
    list_in_slot(&mut run, &mut store, input, original.clone());

    // Start
    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        4,
        body,
        done,
        Some(iterator_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

    let mut collected: Vec<SlotValue> = vec![
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
    ];

    // Advance through remaining 3 items
    for _ in 0..3 {
        let result = for_each_next(
            &mut run,
            &mut store,
            iterator_slot,
            body,
            done,
            Some(item_slot),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        collected.push(
            *run.read_slot(item_slot)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
        );
    }

    // Final call jumps to done
    let result = for_each_next(
        &mut run,
        &mut store,
        iterator_slot,
        body,
        done,
        Some(item_slot),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);

    assert_eq!(collected, original);
}

// ── Verification 5: nested for_each -- inner loops have their own budget ──

#[test]
fn phase22_nested_for_each_outer_and_inner_have_separate_limits() {
    // Simulate a nested loop: outer has 3 items, inner has 2 items.
    // Each gets its own limit. The outer limit does not constrain the inner.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let outer_input = SlotIdx::new(0);
    let outer_item_slot = SlotIdx::new(1);
    let outer_iterator = SlotIdx::new(2);
    let inner_input = SlotIdx::new(3);
    let inner_item_slot = SlotIdx::new(4);
    let inner_iterator = SlotIdx::new(5);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Outer list: [100, 200, 300]
    let outer_items = vec![
        SlotValue::I64(100),
        SlotValue::I64(200),
        SlotValue::I64(300),
    ];
    list_in_slot(&mut run, &mut store, outer_input, outer_items);

    // Inner list: [1, 2]
    let inner_items = vec![SlotValue::I64(1), SlotValue::I64(2)];
    list_in_slot(&mut run, &mut store, inner_input, inner_items);

    // Start outer with limit=3
    let outer_result = for_each_start(
        &mut run,
        &mut store,
        outer_input,
        outer_item_slot,
        3, // outer budget
        body,
        done,
        Some(outer_iterator),
    );
    assert_eq!(outer_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(outer_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(100)
    );

    // Start inner with limit=2 (its own budget, independent of outer's limit)
    let inner_result = for_each_start(
        &mut run,
        &mut store,
        inner_input,
        inner_item_slot,
        2, // inner budget
        body,
        done,
        Some(inner_iterator),
    );
    assert_eq!(inner_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(inner_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(1)
    );

    // Advance inner (consumes item 2)
    let inner_next = for_each_next(
        &mut run,
        &mut store,
        inner_iterator,
        body,
        done,
        Some(inner_item_slot),
    );
    assert_eq!(inner_next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(inner_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(2)
    );

    // Inner finishes (empty tail -> done)
    let inner_done = for_each_next(
        &mut run,
        &mut store,
        inner_iterator,
        body,
        done,
        Some(inner_item_slot),
    );
    assert_eq!(inner_done, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);

    // Outer can still advance (its iterator is independent)
    // Reset PC to a valid step for next advance
    assert!(run.set_pc(StepIdx::ZERO).is_ok());
    let outer_next = for_each_next(
        &mut run,
        &mut store,
        outer_iterator,
        body,
        done,
        Some(outer_item_slot),
    );
    assert_eq!(outer_next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(outer_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(200)
    );
}

#[test]
fn phase22_nested_for_each_inner_exceeding_own_limit_rejected() {
    // Outer loop has limit=5 but inner loop has limit=1 with 3 items.
    // The inner loop's limit must reject, regardless of outer's budget.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let outer_input = SlotIdx::new(0);
    let outer_item_slot = SlotIdx::new(1);
    let outer_iterator = SlotIdx::new(2);
    let inner_input = SlotIdx::new(3);
    let inner_item_slot = SlotIdx::new(4);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Outer list: [100] -- within outer limit
    list_in_slot(&mut run, &mut store, outer_input, vec![SlotValue::I64(100)]);
    // Start outer with limit=5
    let outer_result = for_each_start(
        &mut run,
        &mut store,
        outer_input,
        outer_item_slot,
        5,
        body,
        done,
        Some(outer_iterator),
    );
    assert_eq!(outer_result, Ok(vb_core::EngineSignal::Continue));

    // Inner list: [1, 2, 3] -- exceeds inner's limit of 1
    let inner_items = vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)];
    list_in_slot(&mut run, &mut store, inner_input, inner_items);

    // Start inner with limit=1 -- must fail because 3 > 1
    let inner_result = for_each_start(
        &mut run,
        &mut store,
        inner_input,
        inner_item_slot,
        1, // inner budget
        body,
        done,
        Some(SlotIdx::new(5)),
    );

    assert!(
        matches!(inner_result, Err(EngineError::IterationLimitExceeded { resource }) if resource == "for_each_limit"),
        "inner loop must reject list exceeding its own budget, got {inner_result:?}"
    );

    // Outer state is untouched -- the inner rejection did not corrupt outer
    assert_eq!(
        *run.read_slot(outer_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(100)
    );
}

#[test]
fn phase22_nested_for_each_inner_with_larger_budget_than_outer_succeeds() {
    // Outer has limit=2, inner has limit=10. Inner succeeds independently.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let outer_input = SlotIdx::new(0);
    let outer_item_slot = SlotIdx::new(1);
    let outer_iterator = SlotIdx::new(2);
    let inner_input = SlotIdx::new(3);
    let inner_item_slot = SlotIdx::new(4);
    let inner_iterator = SlotIdx::new(5);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Outer: 2 items, limit=2
    list_in_slot(
        &mut run,
        &mut store,
        outer_input,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    let outer_result = for_each_start(
        &mut run,
        &mut store,
        outer_input,
        outer_item_slot,
        2,
        body,
        done,
        Some(outer_iterator),
    );
    assert_eq!(outer_result, Ok(vb_core::EngineSignal::Continue));

    // Inner: 5 items, limit=10 (larger than outer's limit=2, but independent)
    let inner_items: Vec<SlotValue> = (0..5).map(SlotValue::I64).collect();
    list_in_slot(&mut run, &mut store, inner_input, inner_items);
    let inner_result = for_each_start(
        &mut run,
        &mut store,
        inner_input,
        inner_item_slot,
        10, // inner budget is independent and larger than outer
        body,
        done,
        Some(inner_iterator),
    );

    // Inner succeeds -- its budget is independent of outer's
    assert_eq!(inner_result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(inner_item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(0)
    );
}

// ── Additional boundary tests for fanout ──

#[test]
fn phase22_for_each_start_u32_max_limit_accepts_large_list() {
    // Given a list with 100 items and limit=u32::MAX
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let items: Vec<SlotValue> = (0..100).map(SlotValue::I64).collect();
    list_in_slot(&mut run, &mut store, input, items);

    let result = for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        FanoutLimit::new(u32::MAX),
        body,
        StepIdx::new(2),
        Some(output_slot),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn phase22_for_each_iteration_drains_all_items_sequentially() {
    // Given a 4-item list, verify the full drain cycle through
    // for_each_start + repeated for_each_next produces all items.
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let iterator_slot = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    let limit = FanoutLimit::new(64);

    let original = vec![
        SlotValue::I64(10),
        SlotValue::I64(20),
        SlotValue::I64(30),
        SlotValue::I64(40),
    ];
    list_in_slot(&mut run, &mut store, input, original.clone());

    // Start
    for_each_start(
        &mut run,
        &mut store,
        input,
        item_slot,
        limit,
        body,
        done,
        Some(iterator_slot),
    )
    .ok()
    .unwrap_or_else(|| panic!("start must succeed"));

    let mut seen: Vec<SlotValue> = Vec::new();
    seen.push(
        *run.read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
    );

    // Drain remaining items via for_each_next
    loop {
        let before_pc = run.pc();
        let result = for_each_next(
            &mut run,
            &mut store,
            iterator_slot,
            body,
            done,
            Some(item_slot),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        if run.pc() == done {
            break;
        }
        // Must have jumped to body
        assert_eq!(run.pc(), body);
        let item = *run
            .read_slot(item_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"));
        seen.push(item);
        let _ = before_pc; // used for clarity
    }

    assert_eq!(seen, original);
}
