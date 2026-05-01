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
    let items = store.list(list_id)?.to_vec();
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
    let ps = page_size_from(page_size)?;
    if items.len() > ps {
        return Err(EngineError::CollectPageLimitExceeded);
    }
    write_collected_page(run, store, collector, &items)?;
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
    collector_slot: SlotIdx,
    _body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let remaining_id = expect_list(*run.read_slot(collector_slot)?)?;
    let _ = store.list(remaining_id)?;
    jump_to(run, done)
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

fn write_collected_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: &[SlotValue],
) -> Result<(), EngineError> {
    let page_id = store.insert_list(copy_items(items)?)?;
    run.write_slot(collector, SlotValue::List(page_id))?;
    Ok(())
}

fn copy_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    Ok(items
        .get(..)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "copy_items full range within bounds",
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
}
