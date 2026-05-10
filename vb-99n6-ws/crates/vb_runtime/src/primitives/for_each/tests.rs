//! Tests for for_each iteration primitive.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use crate::primitives::for_each::{for_each_join, for_each_next, for_each_start};
use crate::test_harness::list_in_slot;

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
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(42), SlotValue::I64(99)]);

    let result = for_each_start(
        &mut run, &mut store, input, item_slot, 100, body, done, Some(output_slot),
    );

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(
        *run.read_slot(item_slot).ok().unwrap(),
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
        &mut run, &mut store, input, item_slot, 100, body, done, Some(output_slot),
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
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(7), SlotValue::I64(8)]);

    let result = for_each_next(&mut run, &mut store, iterator_slot, body, done, Some(output_slot));

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_eq!(*run.read_slot(output_slot).ok().unwrap(), SlotValue::I64(7));
}

#[test]
fn for_each_join_returns_done_signal() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let next_step = StepIdx::new(1);
    list_in_slot(&mut run, &mut store, materialized_slot, vec![SlotValue::I64(42)]);

    let result = for_each_join(&mut run, materialized_slot, Some(output_slot), Some(next_step), StepIdx::ZERO);

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
}

#[test]
fn for_each_join_materializes_ordered_results() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, materialized_slot, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);

    let result = for_each_join(&mut run, materialized_slot, Some(output_slot), Some(StepIdx::new(1)), StepIdx::ZERO);

    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let output_value = *run.read_slot(output_slot).ok().unwrap();
    let SlotValue::List(list_id) = output_value else { panic!("output must be list") };
    let items = store.list(list_id).ok().unwrap();
    assert_eq!(items, [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);
}

#[test]
fn for_each_start_returns_error_when_input_is_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    run.write_slot(input, SlotValue::I64(42)).ok();
    let result = for_each_start(&mut run, &mut store, input, item_slot, 100, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_start_returns_error_when_limit_exceeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);
    let result = for_each_start(&mut run, &mut store, input, item_slot, 2, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => assert_eq!(resource, "for_each_limit"),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_start_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    let result = for_each_start(&mut run, &mut store, input, item_slot, 100, StepIdx::new(1), StepIdx::new(2), None);
    match result {
        Err(EngineError::MissingOutputSlot { step }) => assert_eq!(step, StepIdx::ZERO),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_next_returns_done_when_tail_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![]);
    let result = for_each_next(&mut run, &mut store, iterator_slot, StepIdx::new(1), done, Some(output_slot));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn for_each_next_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
    let result = for_each_next(&mut run, &mut store, iterator_slot, StepIdx::new(1), StepIdx::new(2), None);
    match result {
        Err(EngineError::MissingOutputSlot { step }) => assert_eq!(step, StepIdx::ZERO),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_next_returns_error_when_iterator_is_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    run.write_slot(iterator_slot, SlotValue::Bool(true)).ok();
    let result = for_each_next(&mut run, &mut store, iterator_slot, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_join_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let result = for_each_join(&mut run, SlotIdx::new(0), None, Some(StepIdx::new(1)), StepIdx::ZERO);
    match result {
        Err(EngineError::MissingOutputSlot { step }) => assert_eq!(step, StepIdx::ZERO),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_join_returns_error_when_next_missing() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let materialized_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, materialized_slot, vec![SlotValue::I64(1)]);
    let result = for_each_join(&mut run, materialized_slot, Some(output_slot), None, StepIdx::ZERO);
    match result {
        Err(EngineError::MissingNextStep { step }) => assert_eq!(step, StepIdx::ZERO),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_start_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    let before = run.executed();
    let result = for_each_start(&mut run, &mut store, input, item_slot, 100, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn for_each_next_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let output_slot = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
    let before = run.executed();
    let result = for_each_next(&mut run, &mut store, iterator_slot, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn for_each_start_limit_zero_allows_empty_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, input, vec![]);
    let result = for_each_start(&mut run, &mut store, input, item_slot, 0, StepIdx::new(1), done, Some(output_slot));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn for_each_start_limit_zero_rejects_single_item() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    let item_slot = SlotIdx::new(1);
    let output_slot = SlotIdx::new(2);
    list_in_slot(&mut run, &mut store, input, vec![SlotValue::I64(1)]);
    let result = for_each_start(&mut run, &mut store, input, item_slot, 0, StepIdx::new(1), StepIdx::new(2), Some(output_slot));
    match result {
        Err(EngineError::IterationLimitExceeded { resource }) => assert_eq!(resource, "for_each_limit"),
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_start_null_input_returns_type_mismatch() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let input = SlotIdx::new(0);
    run.write_slot(input, SlotValue::Null).ok();
    let result = for_each_start(&mut run, &mut store, input, SlotIdx::new(1), 100, StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(2)));
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "null");
        }
        other => assert_eq!(other, Ok(vb_core::EngineSignal::Continue)),
    }
}

#[test]
fn for_each_next_output_slot_same_as_iterator_overwrite() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let iterator_slot = SlotIdx::new(0);
    let body = StepIdx::new(1);
    list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1), SlotValue::I64(2)]);
    let result = for_each_next(&mut run, &mut store, iterator_slot, body, StepIdx::new(2), Some(iterator_slot));
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run.read_slot(iterator_slot).ok().unwrap() {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap();
            assert_eq!(items.len(), 1);
        }
        other => assert_eq!(other, SlotValue::I64(0)),
    }
}
