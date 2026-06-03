#![forbid(unsafe_code)]
//! Collect validation helpers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

use super::state::CollectPaginationState;

/// Validates that cursor position is within source bounds.
pub(crate) fn validate_cursor_in_source(
    state: &CollectPaginationState,
    source_len: usize,
) -> Result<(), EngineError> {
    if state.cursor > source_len {
        return Err(EngineError::InternalInvariantViolation {
            reason: "collect cursor beyond source items",
        });
    }
    Ok(())
}

/// Validates item count against collect limit.
pub(crate) fn validate_item_limit(count: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?;
    if count > max {
        Err(EngineError::CollectItemLimitExceeded)
    } else {
        Ok(())
    }
}

/// Validates page size is nonzero and within bounds.
pub(crate) fn page_size_from(raw: u32) -> Result<usize, EngineError> {
    if raw == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        });
    }
    usize::try_from(raw).map_err(|_| EngineError::CollectPageLimitExceeded)
}

/// Validates page size doesn't exceed limit.
pub(crate) fn validate_page_bound(page_size: usize, limit: u32) -> Result<(), EngineError> {
    let max = usize::try_from(limit).map_err(|_| EngineError::CollectPageLimitExceeded)?;
    if page_size > max {
        Err(EngineError::CollectPageLimitExceeded)
    } else {
        Ok(())
    }
}

/// Validates collect state consistency.
pub(crate) fn validate_collect_state(
    state: &CollectPaginationState,
    source_len: usize,
) -> Result<(), EngineError> {
    if state.page_size > state.limit {
        return Err(EngineError::CollectPageLimitExceeded);
    }
    if state.item_count > state.limit {
        return Err(EngineError::CollectItemLimitExceeded);
    }
    if source_len != state.item_count {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect source length changed",
        });
    }
    Ok(())
}

/// Copies a prefix of items up to page_size.
pub(crate) fn copy_prefix(items: &[SlotValue], page_size: usize) -> Result<Box<[SlotValue]>, EngineError> {
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

/// Copies a range of items from start position.
pub(crate) fn copy_page_range(
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

/// Checked usize addition.
pub(crate) fn checked_add_usize(
    left: usize,
    right: usize,
    reason: &'static str,
) -> Result<usize, EngineError> {
    left.checked_add(right)
        .ok_or(EngineError::InternalInvariantViolation { reason })
}

/// Writes a collected page to the collector slot.
pub(crate) fn write_collected_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: Box<[SlotValue]>,
) -> Result<ListId, EngineError> {
    let page_id = store.insert_list(items)?;
    run.write_slot(collector, SlotValue::List(page_id))?;
    Ok(page_id)
}

/// Writes a collected page with taint to the collector slot.
pub(crate) fn write_collected_page_with_taint(
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

/// Writes an empty collector slot.
pub(crate) fn write_empty_collector(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    taint: Taint,
) -> Result<(), EngineError> {
    let empty_id = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot_with_taint(collector, SlotValue::List(empty_id), taint)?;
    Ok(())
}
