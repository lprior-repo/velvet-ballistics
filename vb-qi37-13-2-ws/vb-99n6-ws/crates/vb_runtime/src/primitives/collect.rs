#![forbid(unsafe_code)]
//! Collect pagination primitive handlers.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_storage::JournalEvent;

use super::helpers::{expect_list, jump_to, jump_to_next, require_output};

/// Per-run pagination state stored in a side table keyed by (RunId, SlotIdx).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectPaginationState {
    run_id: RunId,
    collector_slot: SlotIdx,
    source: ListId,
    current_page: ListId,
    cursor: usize,
    page_size: usize,
    item_count: usize,
    limit: usize,
    time_limit_ms: Option<u64>,
    start_millis: u64,
}

/// Side table replacing the global Mutex. Owns pagination state per (RunId, SlotIdx).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectStates {
    entries: HashMap<(RunId, SlotIdx), CollectPaginationState>,
}

impl CollectStates {
    /// Create an empty state table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the state for the given key.
    pub fn upsert(&mut self, state: CollectPaginationState) -> Result<(), EngineError> {
        let key = (state.run_id, state.collector_slot);
        self.entries.insert(key, state);
        Ok(())
    }

    /// Find the state matching (run_id, collector_slot, current_page).
    pub fn find(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
        current_page: ListId,
    ) -> Option<CollectPaginationState> {
        self.entries
            .get(&(run_id, collector_slot))
            .filter(|s| s.current_page == current_page)
            .copied()
    }

    /// Remove state for the given key.
    pub fn remove(&mut self, run_id: RunId, collector_slot: SlotIdx) {
        self.entries.remove(&(run_id, collector_slot));
    }

    /// Serialize the active state for a collector slot as durable frame extra data.
    pub fn capture_extra(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
    ) -> Result<Option<Vec<u8>>, EngineError> {
        self.entries
            .get(&(run_id, collector_slot))
            .map(postcard::to_allocvec)
            .transpose()
            .map_err(|_| EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state encode failed",
            })
    }

    /// Capture the active state for a collector slot.
    #[must_use]
    pub fn capture_state(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
    ) -> Option<CollectPaginationState> {
        self.entries.get(&(run_id, collector_slot)).copied()
    }

    /// Hydrate durable frame extra data into the pagination side table.
    pub fn hydrate_extra(
        &mut self,
        run_id: RunId,
        collector_slot: SlotIdx,
        extra: &[u8],
    ) -> Result<(), EngineError> {
        let state: CollectPaginationState =
            postcard::from_bytes(extra).map_err(|_| EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state decode failed",
            })?;
        validate_hydrated_identity(&state, run_id, collector_slot)?;
        self.upsert(state)
    }

    /// Hydrate durable pagination extras carried by slot-write journal events.
    pub fn hydrate_journal_events(&mut self, events: &[JournalEvent]) -> Result<(), EngineError> {
        events
            .iter()
            .try_for_each(|event| self.hydrate_journal_event(event))
    }

    fn hydrate_journal_event(&mut self, event: &JournalEvent) -> Result<(), EngineError> {
        match event {
            JournalEvent::SlotWrittenEvent {
                run,
                slot,
                extra: Some(extra),
                ..
            } => self.hydrate_extra(*run, *slot, extra),
            _ => Ok(()),
        }
    }
}

/// Builds collect pagination state from durable journal events recovered for a run.
pub fn hydrate_collect_states_from_recovered_journal(
    events: &[JournalEvent],
) -> Result<CollectStates, EngineError> {
    let mut states = CollectStates::new();
    states.hydrate_journal_events(events)?;
    Ok(states)
}

fn validate_hydrated_identity(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
) -> Result<(), EngineError> {
    if state.run_id != run_id || state.collector_slot != collector_slot {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect pagination state identity mismatch",
        });
    }
    Ok(())
}

/// Executes CollectStart: reads source list, writes the first page,
/// and jumps to body or done.
#[allow(clippy::too_many_arguments)]
pub fn collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
    time_limit_ms: Option<u64>,
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
        write_empty_collector(run, store, collector, source_taint)?;
        states.remove(run.run_id(), collector);
        return jump_to(run, done);
    }
    let page = copy_prefix(items, ps)?;
    let page_len = page.len();
    let current_page = write_collected_page_with_taint(run, store, collector, page, source_taint)?;
    let cursor = checked_add_usize(0, page_len, "collect cursor overflow")?;
    let start_millis = millis_since_epoch()?;
    states.upsert(CollectPaginationState {
        run_id: run.run_id(),
        collector_slot: collector,
        source: list_id,
        current_page,
        cursor,
        page_size: ps,
        item_count,
        limit: usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?,
        time_limit_ms,
        start_millis,
    })?;
    jump_to(run, body)
}

/// Executes CollectPage: reads current page from collector slot
/// and dispatches to body for processing.
pub fn collect_page(
    run: &mut RunFrame,
    _store: &mut ValueStore,
    _states: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    expect_list(*run.read_slot(collector_slot)?)?;
    jump_to(run, body)
}

/// Executes CollectNext by advancing the bounded pagination cursor.
#[allow(clippy::too_many_arguments)]
pub fn collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current_id = expect_list(*run.read_slot(collector_slot)?)?;
    let current = store.list(current_id)?;
    if current.is_empty() {
        states.remove(run.run_id(), collector_slot);
        return jump_to(run, done);
    }
    let state = states
        .find(run.run_id(), collector_slot, current_id)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "collect pagination state missing",
        })?;
    check_time_limit(&state)?;
    let source_items = store.list(state.source)?;
    validate_collect_state(&state, source_items.len())?;
    if state.cursor >= state.item_count {
        let empty_page = Vec::<SlotValue>::new().into_boxed_slice();
        let _ = write_collected_page(run, store, collector_slot, empty_page)?;
        states.remove(run.run_id(), collector_slot);
        return jump_to(run, done);
    }
    if state.cursor > source_items.len() {
        return Err(EngineError::InternalInvariantViolation {
            reason: "collect cursor beyond source items",
        });
    }
    let page = copy_page_range(source_items, state.cursor, state.page_size)?;
    let page_len = page.len();
    let current_page = write_collected_page(run, store, collector_slot, page)?;
    let cursor = checked_add_usize(state.cursor, page_len, "collect cursor overflow")?;
    states.upsert(CollectPaginationState {
        current_page,
        cursor,
        ..state
    })?;
    jump_to(run, body)
}

/// Executes CollectFinish: writes the collected result to output.
pub fn collect_finish(
    run: &mut RunFrame,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let final_value = *run.read_slot(collector_slot)?;
    let final_taint = run.read_taint(collector_slot)?;
    let out = require_output(output, step)?;
    run.write_slot_with_taint(out, final_value, final_taint)?;
    states.remove(run.run_id(), collector_slot);
    jump_to_next(run, next, step)
}

fn check_time_limit(state: &CollectPaginationState) -> Result<(), EngineError> {
    if let Some(limit_ms) = state.time_limit_ms {
        let elapsed = millis_since_epoch()?.saturating_sub(state.start_millis);
        if elapsed > limit_ms {
            return Err(EngineError::CollectTimeLimitExceeded);
        }
    }
    Ok(())
}

fn millis_since_epoch() -> Result<u64, EngineError> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| EngineError::InternalInvariantViolation {
            reason: "system time is before UNIX epoch",
        })?
        .as_millis()
        .try_into()
        .map_err(|_| EngineError::InternalInvariantViolation {
            reason: "millis_since_epoch overflow",
        })
}

fn write_empty_collector(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    taint: Taint,
) -> Result<(), EngineError> {
    let empty_id = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot_with_taint(collector, SlotValue::List(empty_id), taint)?;
    Ok(())
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

fn validate_collect_state(
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

fn checked_add_usize(
    left: usize,
    right: usize,
    reason: &'static str,
) -> Result<usize, EngineError> {
    left.checked_add(right)
        .ok_or(EngineError::InternalInvariantViolation { reason })
}

#[cfg(test)]
#[path = "../collect_tests.rs"]
mod tests;
