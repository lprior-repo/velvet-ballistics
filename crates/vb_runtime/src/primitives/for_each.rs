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
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 8).ok().unwrap_or_else(||
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
        assert_eq!(*run.read_slot(item_slot).ok().unwrap_or_else(|| panic!("read must succeed")), SlotValue::I64(42));
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
        list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(7), SlotValue::I64(8)]);

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
        assert_eq!(*run.read_slot(output_slot).ok().unwrap_or_else(|| panic!("read must succeed")), SlotValue::I64(7));
    }

    #[test]
    fn for_each_join_returns_done_signal() {
        let mut run = fresh_frame();
        let output_slot = SlotIdx::new(0);
        let next_step = StepIdx::new(1);
        run.write_slot(output_slot, SlotValue::I64(42)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = for_each_join(
            &mut run,
            Some(output_slot),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
    }
}
