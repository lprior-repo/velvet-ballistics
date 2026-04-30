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
    let collector = output
        .ok_or(EngineError::MissingOutputSlot {
            step: run.pc(),
        })?;
    if items.is_empty() {
        run.write_slot(collector, SlotValue::List(
            store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?,
        ))?;
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
    let out = output
        .ok_or(EngineError::MissingOutputSlot { step })?;
    run.write_slot(out, final_value)?;
    jump_to_next(run, next, step)
}

fn validate_item_limit(count: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit)
        .map_err(|_| EngineError::CollectItemLimitExceeded)?;
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

fn jump_to(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
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
