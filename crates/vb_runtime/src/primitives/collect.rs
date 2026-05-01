//! Collect pagination primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

/// Executes CollectStart: reads source list, writes first page,
/// stores remaining items in collector slot, jumps to body.
#[allow(clippy::too_many_arguments)]
pub fn collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let list_id = expect_list(*run.read_slot(source)?)?;
    let items: Vec<SlotValue> = store.list(list_id)?.to_vec();
    validate_item_limit(items.len(), limit)?;
    let collector = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    if items.is_empty() {
        run.write_slot(
            collector,
            SlotValue::List(store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?),
        )?;
        return jump_to(run, done);
    }
    let ps = page_size_from(page_size)?;
    write_first_page(run, store, collector, &items, ps)?;
    jump_to(run, body)
}

/// Executes CollectPage: reads current page from collector slot
/// and dispatches to body for processing.
pub fn collect_page(
    run: &mut RunFrame,
    _store: &mut ValueStore,
    collector_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let _ = expect_list(*run.read_slot(collector_slot)?)?;
    jump_to(run, body)
}

/// Executes CollectNext: advances to next page from remaining items.
///
/// The collector slot stores remaining items after the current page.
/// Takes the next page and updates the slot, or jumps to done.
#[allow(clippy::too_many_arguments)]
pub fn collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    _source: SlotIdx,
    limit: u32,
    page_size: u32,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let remaining_id = expect_list(*run.read_slot(collector_slot)?)?;
    let remaining = store.list(remaining_id)?;
    if remaining.is_empty() {
        return jump_to(run, done);
    }
    validate_item_limit(remaining.len(), limit)?;
    let ps = page_size_from(page_size)?;
    let page = take_items(remaining, ps);
    let page_id = store.insert_list(page?)?;
    run.write_slot(collector_slot, SlotValue::List(page_id))?;
    jump_to(run, body)
}

/// Executes CollectFinish: writes the collected result to output.
pub fn collect_finish(
    run: &mut RunFrame,
    collector_slot: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let final_value = *run.read_slot(collector_slot)?;
    let out = output.ok_or(EngineError::MissingOutputSlot { step })?;
    run.write_slot(out, final_value)?;
    jump_to_next(run, next, step)
}

fn validate_item_limit(count: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?;
    if count > max {
        Err(EngineError::CollectItemLimitExceeded)
    } else {
        Ok(())
    }
}

fn expect_list(value: SlotValue) -> Result<vb_core::ids::ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

fn page_size_from(raw: u32) -> Result<usize, EngineError> {
    if raw == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        });
    }
    usize::try_from(raw).map_err(|_| EngineError::CollectPageLimitExceeded)
}

fn write_first_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: &[SlotValue],
    page_size: usize,
) -> Result<(), EngineError> {
    let page = take_items(items, page_size);
    let page_id = store.insert_list(page?)?;
    run.write_slot(collector, SlotValue::List(page_id))?;
    Ok(())
}

fn take_items(items: &[SlotValue], max: usize) -> Result<Box<[SlotValue]>, EngineError> {
    let end = items.len().min(max);
    Ok(items
        .get(..end)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "take_items end within bounds",
        })?
        .to_vec()
        .into_boxed_slice())
}

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target);
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;
    use vb_core::value_store::ValueStore;

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 8, 8).ok().unwrap_or_else(||
            panic!("frame creation must succeed")
        )
    }

    fn list_in_slot(run: &mut RunFrame, store: &mut ValueStore, slot: SlotIdx, items: Vec<SlotValue>) {
        let id = store.insert_list(items.into_boxed_slice()).ok().unwrap_or_else(||
            panic!("list insertion must succeed")
        );
        run.write_slot(slot, SlotValue::List(id)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );
    }

    #[test]
    fn collect_start_initializes_collector() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);

        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            body,
            done,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        let slot_val = *run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed"));
        assert!(matches!(slot_val, SlotValue::List(_)));
    }

    #[test]
    fn collect_page_increments_page_count() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(10)]);

        let result = collect_page(
            &mut run,
            &mut store,
            collector,
            body,
            done,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_next_returns_continue_while_pages_remain() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(5), SlotValue::I64(6)]);

        let result = collect_next(
            &mut run,
            &mut store,
            source,
            100,
            1,
            collector,
            body,
            done,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_finish_materializes_output() {
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(collector, SlotValue::I64(99)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = collect_finish(
            &mut run,
            collector,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(*run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed")), SlotValue::I64(99));
    }

    // BDD tests for collect primitives

    #[test]
    fn collect_start_returns_error_when_source_is_not_list() {
        // Given a frame with a non-list in source slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        run.write_slot(source, SlotValue::Bool(true)).ok().unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_start
        let result = collect_start(&mut run, &mut store, source, 100, 2, StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(1)));
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
    fn collect_start_returns_error_when_limit_exceeded() {
        // Given a frame with a 5-item list and limit=3
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3), SlotValue::I64(4), SlotValue::I64(5)]);
        // When calling collect_start with limit=3
        let result = collect_start(&mut run, &mut store, source, 3, 2, StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(1)));
        // Then it returns CollectItemLimitExceeded
        match result {
            Err(EngineError::CollectItemLimitExceeded) => {}
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn collect_start_returns_error_when_output_missing() {
        // Given a frame with a list in source but no output slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        // When calling collect_start with output=None
        let result = collect_start(&mut run, &mut store, source, 100, 2, StepIdx::new(1), StepIdx::new(2), None);
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
    fn collect_start_returns_error_when_page_size_zero() {
        // Given a frame with a list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        // When calling collect_start with page_size=0
        let result = collect_start(&mut run, &mut store, source, 100, 0, StepIdx::new(1), StepIdx::new(2), Some(SlotIdx::new(1)));
        // Then it returns InvalidCompiledWorkflow
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
        // Given a frame with an empty list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![]);
        // When calling collect_start
        let result = collect_start(&mut run, &mut store, source, 100, 2, StepIdx::new(1), done, Some(output));
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_next_returns_done_when_remaining_empty() {
        // Given a frame with an empty list in collector slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, collector, vec![]);
        // When calling collect_next
        let result = collect_next(&mut run, &mut store, SlotIdx::new(1), 100, 2, collector, StepIdx::new(1), done);
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_finish_returns_error_when_output_missing() {
        // Given a frame
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::I64(1)).ok().unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_finish with output=None
        let result = collect_finish(&mut run, collector, None, Some(StepIdx::new(1)), StepIdx::ZERO);
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
    fn collect_finish_returns_error_when_next_missing() {
        // Given a frame
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        run.write_slot(collector, SlotValue::I64(1)).ok().unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_finish with next=None
        let result = collect_finish(&mut run, collector, Some(output), None, StepIdx::ZERO);
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
    fn collect_page_returns_error_when_collector_not_list() {
        // Given a frame with non-list in collector
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::I64(42)).ok().unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_page
        let result = collect_page(&mut run, &mut store, collector, StepIdx::new(1), StepIdx::new(2));
        // Then it returns TypeMismatch
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
        // Given a frame with 3 items and page_size=2
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);
        // When calling collect_start
        let result = collect_start(&mut run, &mut store, source, 100, 2, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then collector has first page (2 items)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed")) {
            SlotValue::List(id) => {
                let items = store.list(id).ok().unwrap_or_else(|| panic!("list read must succeed"));
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
        // Given a frame with a list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        let before = run.executed();
        // When calling collect_start
        let result = collect_start(&mut run, &mut store, source, 100, 2, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_increments_executed_counter() {
        // Given a frame with remaining items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
        let before = run.executed();
        // When calling collect_next
        let result = collect_next(&mut run, &mut store, SlotIdx::new(1), 100, 1, collector, StepIdx::new(1), StepIdx::new(2));
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_page_increments_executed_counter() {
        // Given a frame with collector list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
        let before = run.executed();
        // When calling collect_page
        let result = collect_page(&mut run, &mut store, collector, StepIdx::new(1), StepIdx::new(2));
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_finish_increments_executed_counter() {
        // Given a frame with collector value
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        run.write_slot(collector, SlotValue::I64(99)).ok().unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling collect_finish
        let result = collect_finish(&mut run, collector, Some(output), Some(StepIdx::new(1)), StepIdx::ZERO);
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_returns_error_when_limit_exceeded() {
        // Given a frame with items exceeding limit
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]);
        // When calling collect_next with limit=2
        let result = collect_next(&mut run, &mut store, SlotIdx::new(1), 2, 1, collector, StepIdx::new(1), StepIdx::new(2));
        // Then it returns CollectItemLimitExceeded
        match result {
            Err(EngineError::CollectItemLimitExceeded) => {}
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn collect_next_returns_error_when_not_list() {
        // Given a frame with non-list in collector
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::Bool(true)).ok().unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_next
        let result = collect_next(&mut run, &mut store, SlotIdx::new(1), 100, 2, collector, StepIdx::new(1), StepIdx::new(2));
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
}
