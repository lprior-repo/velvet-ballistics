//! Collect pagination primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use super::helpers::{expect_list, jump_to, jump_to_next, require_output};

/// Executes CollectStart: reads source list, writes the first page,
/// and jumps to body or done.
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
    let ps = page_size_from(page_size)?;
    validate_page_bound(ps, limit)?;
    let items = store.list(list_id)?;
    validate_item_limit(items.len(), limit)?;
    let collector = match output {
        Some(slot) => slot,
        None => source,
    };
    if items.is_empty() {
        run.write_slot(
            collector,
            SlotValue::List(store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?),
        )?;
        return jump_to(run, done);
    }
    let page = copy_prefix(items, ps)?;
    write_collected_page(run, store, collector, page)?;
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

/// Executes CollectNext.
///
/// The current IR does not provide a cursor/source state slot to distinguish
/// the current page from remaining items. Non-empty continuation is therefore
/// unsupported instead of pretending to advance by repeating the same page.
#[allow(clippy::too_many_arguments)]
pub fn collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current_id = expect_list(*run.read_slot(collector_slot)?)?;
    let current = store.list(current_id)?;
    if current.is_empty() {
        return jump_to(run, done);
    }
    let _ = body;
    Err(EngineError::UnsupportedPrimitive {
        primitive: "CollectNext.pagination_state",
    })
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
    let out = require_output(output, step)?;
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
fn page_size_from(raw: u32) -> Result<usize, EngineError> {
    if raw == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        });
    }
    usize::try_from(raw).map_err(|_| EngineError::CollectPageLimitExceeded)
}

fn validate_page_bound(page_size: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectPageLimitExceeded)?;
    if page_size > max {
        Err(EngineError::CollectPageLimitExceeded)
    } else {
        Ok(())
    }
}

fn write_collected_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: Box<[SlotValue]>,
) -> Result<(), EngineError> {
    let page_id = store.insert_list(items)?;
    run.write_slot(collector, SlotValue::List(page_id))?;
    Ok(())
}

fn copy_prefix(items: &[SlotValue], page_size: usize) -> Result<Box<[SlotValue]>, EngineError> {
    let count = page_size.min(items.len());
    let mut page = Vec::with_capacity(count);
    let mut index = 0usize;
    while index < count {
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect prefix index checked by loop bound",
            })?;
        page.push(value);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect prefix index overflow",
            })?;
    }
    Ok(page.into_boxed_slice())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::list_in_slot;
    use vb_core::value_store::ValueStore;

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 8)
    }

    #[test]
    fn collect_start_initializes_collector() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
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
            source,
            100,
            2,
            body,
            done,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        let slot_val = *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"));
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

        let result = collect_page(&mut run, &mut store, collector, body, done);

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_next_returns_unsupported_while_page_has_items() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            collector,
            vec![SlotValue::I64(5), SlotValue::I64(6)],
        );

        let result = collect_next(&mut run, &mut store, collector, body, done);

        assert_eq!(
            result,
            Err(EngineError::UnsupportedPrimitive {
                primitive: "CollectNext.pagination_state",
            })
        );
        assert_eq!(run.pc(), StepIdx::ZERO);
    }

    #[test]
    fn collect_finish_materializes_output() {
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(collector, SlotValue::I64(99))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = collect_finish(
            &mut run,
            collector,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(99)
        );
    }

    // BDD tests for collect primitives

    #[test]
    fn collect_start_returns_error_when_source_is_not_list() {
        // Given a frame with a non-list in source slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        run.write_slot(source, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_start
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
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
    fn collect_start_returns_error_when_limit_exceeded() {
        // Given a frame with a 5-item list and limit=3
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
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
        // When calling collect_start with limit=3
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            3,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
        );
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
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
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
    fn collect_start_returns_error_when_page_size_zero() {
        // Given a frame with a list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        // When calling collect_start with page_size=0
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            0,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
        );
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
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            done,
            Some(output),
        );
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
        let result = collect_next(&mut run, &mut store, collector, StepIdx::new(1), done);
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_finish_returns_error_when_output_missing() {
        // Given a frame
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::I64(1))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_finish with output=None
        let result = collect_finish(
            &mut run,
            collector,
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
    fn collect_finish_returns_error_when_next_missing() {
        // Given a frame
        let mut run = fresh_frame();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        run.write_slot(collector, SlotValue::I64(1))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
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
        run.write_slot(collector, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_page
        let result = collect_page(
            &mut run,
            &mut store,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
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
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        // When calling collect_start
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then collector has first page (2 items)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::List(id) => {
                let items = store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("list read must succeed"));
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
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_does_not_increment_executed_without_state() {
        // Given a frame with remaining items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
        let before = run.executed();
        // When calling collect_next
        let result = collect_next(
            &mut run,
            &mut store,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
        assert_eq!(
            result,
            Err(EngineError::UnsupportedPrimitive {
                primitive: "CollectNext.pagination_state",
            })
        );
        assert_eq!(run.executed(), before);
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
        let result = collect_page(
            &mut run,
            &mut store,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
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
        run.write_slot(collector, SlotValue::I64(99))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling collect_finish
        let result = collect_finish(
            &mut run,
            collector,
            Some(output),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_rejects_nonempty_current_page_without_state() {
        // Given a frame with items in collector
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        list_in_slot(
            &mut run,
            &mut store,
            collector,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        // When calling collect_next with remaining items
        let result = collect_next(
            &mut run,
            &mut store,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
        assert_eq!(
            result,
            Err(EngineError::UnsupportedPrimitive {
                primitive: "CollectNext.pagination_state",
            })
        );
        assert_eq!(run.pc(), StepIdx::ZERO);
    }

    #[test]
    fn collect_next_returns_error_when_not_list() {
        // Given a frame with non-list in collector
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling collect_next
        let result = collect_next(
            &mut run,
            &mut store,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
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

    // ── Adversarial BDD tests for collect ───────────────────────────────

    #[test]
    fn collect_start_zero_items_with_nonzero_limit_goes_to_done() {
        // Given a frame with 0 items and limit=100
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![]);
        // When calling collect_start
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            done,
            Some(output),
        );
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_start_page_size_zero_returns_error_even_for_empty_list() {
        // Given a frame with 0 items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(&mut run, &mut store, source, vec![]);
        // When calling collect_start with page_size=0
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            0,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        assert_eq!(
            result,
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect page_size must be nonzero",
            })
        );
    }

    #[test]
    fn collect_start_items_at_exact_limit_boundary() {
        // Given a frame with exactly 3 items and limit=3
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let body = StepIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        // When calling collect_start with limit=3 (exact boundary)
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            3,
            2,
            body,
            StepIdx::new(2),
            Some(output),
        );
        // Then it succeeds (3 <= 3)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_start_items_exceeding_limit_by_one() {
        // Given a frame with 4 items and limit=3
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
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
        // When calling collect_start with limit=3
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            3,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
        );
        // Then it returns CollectItemLimitExceeded
        match result {
            Err(EngineError::CollectItemLimitExceeded) => {}
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn collect_start_first_page_smaller_than_total() {
        // Given a frame with 5 items and page_size=2
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
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
        // When calling collect_start with page_size=2
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then collector has exactly 2 items (the first page)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
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
        // Given a frame with 2 items and page_size=10
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(42), SlotValue::I64(99)],
        );
        // When calling collect_start with page_size=10 (larger than items)
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            10,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then collector has 2 items (clamped to source size)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
                assert_eq!(items.len(), 2);
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    #[test]
    fn collect_next_does_not_fake_next_page_when_items_remain() {
        // Given a frame with remaining items in collector
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let collector = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            collector,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        // When calling collect_next with items remaining
        let result = collect_next(&mut run, &mut store, collector, body, done);
        assert_eq!(
            result,
            Err(EngineError::UnsupportedPrimitive {
                primitive: "CollectNext.pagination_state",
            })
        );
        assert_eq!(run.pc(), StepIdx::ZERO);
    }

    #[test]
    fn collect_start_null_source_returns_type_mismatch() {
        // Given a frame with Null in source slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        run.write_slot(source, SlotValue::Null)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling collect_start
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
        );
        // Then it returns TypeMismatch
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
        // Given a frame with 3 items and page_size=1
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
        );
        // When calling collect_start with page_size=1
        let result = collect_start(
            &mut run,
            &mut store,
            source,
            100,
            1,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then collector has exactly 1 item (first page)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
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
            source,
            1,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
        );

        assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
        assert_eq!(run.pc(), StepIdx::ZERO);
    }
}
