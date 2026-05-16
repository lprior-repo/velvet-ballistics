//! Collect pagination primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

use super::super::helpers::{expect_list, jump_to, jump_to_next, require_output};
use super::state::{self, CollectPaginationState};

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
    let source_taint = run.read_taint(source)?;
    let ps = page_size_from(page_size)?;
    validate_page_bound(ps, limit)?;
    let items = store.list(list_id)?;
    let item_count = items.len();
    validate_item_limit(item_count, limit)?;
    let collector = match output {
        Some(slot) => slot,
        None => source,
    };
    if items.is_empty() {
        run.write_slot_with_taint(
            collector,
            SlotValue::List(store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?),
            source_taint,
        )?;
        state::remove_collect_state(run, collector);
        return jump_to(run, done);
    }
    let page = copy_prefix(items, ps)?;
    let page_len = page.len();
    let current_page = write_collected_page_with_taint(run, store, collector, page, source_taint)?;
    let cursor = checked_add_usize(0, page_len, "collect cursor overflow")?;
    state::upsert_collect_state(CollectPaginationState {
        frame_key: state::collect_frame_key(run),
        run_id: run.run_id(),
        collector_slot: collector,
        source: list_id,
        current_page,
        cursor,
        page_size: ps,
        item_count,
        limit: usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?,
    })?;
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
    expect_list(*run.read_slot(collector_slot)?)?;
    jump_to(run, body)
}

/// Executes CollectNext by advancing the bounded pagination cursor recorded by
/// CollectStart for the visible page in `collector_slot`.
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
        state::remove_collect_state(run, collector_slot);
        return jump_to(run, done);
    }
    let st = state::find_collect_state(run, collector_slot, current_id)?;
    let source_items = store.list(st.source)?;
    validate_collect_state(&st, source_items.len())?;
    if st.cursor >= st.item_count {
        let empty_page = Vec::<SlotValue>::new().into_boxed_slice();
        let _ = write_collected_page(run, store, collector_slot, empty_page)?;
        state::remove_collect_state(run, collector_slot);
        return jump_to(run, done);
    }
    if st.cursor > source_items.len() {
        return Err(EngineError::InternalInvariantViolation {
            reason: "collect cursor beyond source items",
        });
    }
    let page = copy_page_range(source_items, st.cursor, st.page_size)?;
    let page_len = page.len();
    let current_page = write_collected_page(run, store, collector_slot, page)?;
    let cursor = checked_add_usize(st.cursor, page_len, "collect cursor overflow")?;
    state::upsert_collect_state(CollectPaginationState {
        current_page,
        cursor,
        ..st
    })?;
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
    let final_taint = run.read_taint(collector_slot)?;
    let out = require_output(output, step)?;
    run.write_slot_with_taint(out, final_value, final_taint)?;
    state::remove_collect_state(run, collector_slot);
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

fn validate_collect_state(st: &CollectPaginationState, source_len: usize) -> Result<(), EngineError> {
    if st.page_size > st.limit {
        return Err(EngineError::CollectPageLimitExceeded);
    }
    if st.item_count > st.limit {
        return Err(EngineError::CollectItemLimitExceeded);
    }
    if source_len != st.item_count {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect source length changed",
        });
    }
    Ok(())
}

fn checked_add_usize(left: usize, right: usize, reason: &'static str) -> Result<usize, EngineError> {
    left.checked_add(right)
        .ok_or(EngineError::InternalInvariantViolation { reason })
}

fn write_collected_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: Box<[SlotValue]>,
) -> Result<ListId, EngineError> {
    let page_id = store.insert_list(items)?;
    run.write_slot(collector, SlotValue::List(page_id))?;
    Ok(page_id)
}

fn write_collected_page_with_taint(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: Box<[SlotValue]>,
    taint: Taint,
) -> Result<ListId, EngineError> {
    let page_id = store.insert_list(items)?;
    run.write_slot_with_taint(collector, SlotValue::List(page_id), taint)?;
    Ok(page_id)
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

fn copy_page_range(
    items: &[SlotValue],
    start: usize,
    page_size: usize,
) -> Result<Box<[SlotValue]>, EngineError> {
    let remaining =
        items
            .len()
            .checked_sub(start)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect cursor beyond item count",
            })?;
    let count = page_size.min(remaining);
    let mut page = Vec::with_capacity(count);
    let mut offset = 0usize;
    while offset < count {
        let index = checked_add_usize(start, offset, "collect page index overflow")?;
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect page index checked by loop bound",
            })?;
        page.push(value);
        offset = checked_add_usize(offset, 1, "collect page offset overflow")?;
    }
    Ok(page.into_boxed_slice())
}
