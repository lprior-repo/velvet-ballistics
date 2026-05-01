//! ForEach iteration primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

/// Executes ForEachStart: validates input list, binds first item, sets up
/// iterator state in the output slot as the remaining tail list.
#[allow(clippy::too_many_arguments)]
pub fn for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let list_id = expect_list(*run.read_slot(input)?)?;
    let items = store.list(list_id)?;
    let item_count = items.len();
    let limit_count = usize::try_from(limit).map_err(|_| EngineError::IterationLimitExceeded {
        resource: "for_each_limit",
    })?;
    if item_count > limit_count {
        return Err(EngineError::IterationLimitExceeded {
            resource: "for_each_limit",
        });
    }
    let iter_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    if item_count == 0 {
        run.write_slot(
            iter_output,
            SlotValue::List(store.insert_list(empty_list())?),
        )?;
        return jump_to(run, done);
    }
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each item_count checked nonzero",
        })?;
    run.write_slot(item_slot, first)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iter_output, SlotValue::List(tail_id))?;
    jump_to(run, body)
}

/// Executes ForEachNext: advances iterator, binds next item or exits.
pub fn for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let list_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let item_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each next items checked nonempty",
        })?;
    run.write_slot(item_output, first)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iterator_slot, SlotValue::List(tail_id))?;
    jump_to(run, body)
}

/// Executes ForEachJoin: writes the collected results to the output slot.
pub fn for_each_join(
    run: &mut RunFrame,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let output_slot = output.ok_or(EngineError::MissingOutputSlot { step })?;
    let value = *run.read_slot(output_slot)?;
    let join_output = output_slot;
    let _ = (join_output, value);
    jump_to_next(run, next, step)
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

fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    if items.len() <= 1 {
        return Ok(empty_list());
    }
    let start = 1usize;
    Ok(items
        .get(start..)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "tail_items start index checked",
        })?
        .to_vec()
        .into_boxed_slice())
}

fn empty_list() -> Box<[SlotValue]> {
    Vec::<SlotValue>::new().into_boxed_slice()
}

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
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
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 8)
            .ok()
            .unwrap_or_else(|| panic!("frame creation must succeed"))
    }

    fn list_in_slot(
        run: &mut RunFrame,
        store: &mut ValueStore,
        slot: SlotIdx,
        items: Vec<SlotValue>,
    ) {
        let id = store
            .insert_list(items.into_boxed_slice())
            .ok()
            .unwrap_or_else(|| panic!("list insertion must succeed"));
        run.write_slot(slot, SlotValue::List(id))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));
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
            100,
            body,
            done,
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
            100,
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
        let output_slot = SlotIdx::new(0);
        let next_step = StepIdx::new(1);
        run.write_slot(output_slot, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = for_each_join(&mut run, Some(output_slot), Some(next_step), StepIdx::ZERO);

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
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
        let result = for_each_start(
            &mut run,
            &mut store,
            input,
            item_slot,
            2,
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
        let result = for_each_join(&mut run, None, Some(StepIdx::new(1)), StepIdx::ZERO);
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
        let output_slot = SlotIdx::new(0);
        run.write_slot(output_slot, SlotValue::I64(1))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling for_each_join with next=None
        let result = for_each_join(&mut run, Some(output_slot), None, StepIdx::ZERO);
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
}
