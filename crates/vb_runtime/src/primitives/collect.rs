#![forbid(unsafe_code)]
//! Collect pagination primitive handlers.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use vb_core::errors::{
    CollectExtraHydrationFailureKind, CollectPageOrderViolationKind, EngineError,
};
use vb_core::frame::RunFrame;
use vb_core::ids::{EventSeq, ListId, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_storage::JournalEvent;

use super::helpers::{expect_list, jump_to, jump_to_next, require_output};

/// Per-run pagination state stored in a side table keyed by (RunId, SlotIdx).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectPaginationState {
    /// Run owning this pagination state.
    pub run_id: RunId,
    /// Collector slot holding the current page.
    pub collector_slot: SlotIdx,
    /// Source list being paginated.
    pub source: ListId,
    /// Current page list expected in the collector slot.
    pub current_page: ListId,
    /// Next source item cursor.
    pub cursor: usize,
    /// Maximum page size.
    pub page_size: usize,
    /// Source item count captured at start.
    pub item_count: usize,
    /// Collect item limit.
    pub limit: usize,
    /// Optional wall-clock collect time limit.
    pub time_limit_ms: Option<u64>,
    /// Start timestamp in milliseconds since UNIX epoch.
    pub start_millis: u64,
}

/// Side table replacing the global Mutex. Owns pagination state per (RunId, SlotIdx).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectStates {
    entries: HashMap<(RunId, SlotIdx), CollectPaginationState>,
    lineages: HashMap<(RunId, SlotIdx), CollectPageLineage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CollectPageLineage {
    previous_page: Option<ListId>,
    stale_pages: Vec<ListId>,
}

impl CollectStates {
    /// Create an empty state table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the state for the given key.
    pub fn upsert(&mut self, state: CollectPaginationState) -> Result<(), EngineError> {
        let key = (state.run_id, state.collector_slot);
        self.record_lineage(key, state.current_page)?;
        self.entries.insert(key, state);
        Ok(())
    }

    fn record_lineage(
        &mut self,
        key: (RunId, SlotIdx),
        next_page: ListId,
    ) -> Result<(), EngineError> {
        let Some(current) = self.entries.get(&key).map(|state| state.current_page) else {
            self.lineages.entry(key).or_default();
            return Ok(());
        };
        if current == next_page {
            return Ok(());
        }
        let lineage = self.lineages.entry(key).or_default();
        if let Some(previous) = lineage.previous_page {
            lineage.stale_pages.try_reserve(1).map_err(|_| {
                EngineError::InternalInvariantViolation {
                    reason: "collect lineage allocation failed",
                }
            })?;
            lineage.stale_pages.push(previous);
        }
        lineage.previous_page = Some(current);
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

    fn require_current_page(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
        observed_page: ListId,
    ) -> Result<CollectPaginationState, EngineError> {
        let Some(state) = self.entries.get(&(run_id, collector_slot)).copied() else {
            return Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state missing",
            });
        };
        if state.current_page == observed_page {
            return Ok(state);
        }
        let kind = self.classify_observed_page((run_id, collector_slot), observed_page);
        Err(EngineError::CollectPageOrderViolation {
            kind,
            run_id,
            collector_slot,
            expected_page: state.current_page,
            observed_page,
        })
    }

    /// Remove state for the given key.
    pub fn remove(&mut self, run_id: RunId, collector_slot: SlotIdx) {
        let key = (run_id, collector_slot);
        self.entries.remove(&key);
        self.lineages.remove(&key);
    }

    fn classify_observed_page(
        &self,
        key: (RunId, SlotIdx),
        observed_page: ListId,
    ) -> CollectPageOrderViolationKind {
        let Some(lineage) = self.lineages.get(&key) else {
            return CollectPageOrderViolationKind::OutOfOrder;
        };
        if lineage.previous_page == Some(observed_page) {
            return CollectPageOrderViolationKind::Duplicate;
        }
        if lineage.stale_pages.contains(&observed_page) {
            return CollectPageOrderViolationKind::Stale;
        }
        CollectPageOrderViolationKind::OutOfOrder
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
        self.hydrate_extra_with_context(run_id, collector_slot, None, None, extra)
    }

    fn hydrate_extra_with_context(
        &mut self,
        run_id: RunId,
        collector_slot: SlotIdx,
        event_seq: Option<EventSeq>,
        expected_page: Option<ListId>,
        extra: &[u8],
    ) -> Result<(), EngineError> {
        if extra.is_empty() {
            return Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::EmptyExtra,
                run_id,
                collector_slot,
                event_seq,
            });
        }
        let state: CollectPaginationState =
            postcard::from_bytes(extra).map_err(|_| EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            })?;
        validate_hydrated_identity(&state, run_id, collector_slot, event_seq)?;
        if let Some(expected) = expected_page {
            validate_hydrated_page(&state, run_id, collector_slot, event_seq, expected)?;
        }
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
                seq,
                value,
                extra: Some(extra),
                ..
            } => match collect_page_from_event_value(
                *run,
                *slot,
                Some(core_event_seq(*seq)),
                value.as_deref(),
            )? {
                Some(expected_page) => self.hydrate_extra_with_context(
                    *run,
                    *slot,
                    Some(core_event_seq(*seq)),
                    Some(expected_page),
                    extra,
                ),
                None if value.is_none() => self.hydrate_extra_with_context(
                    *run,
                    *slot,
                    Some(core_event_seq(*seq)),
                    None,
                    extra,
                ),
                None => Ok(()),
            },
            _ => Ok(()),
        }
    }
}

fn core_event_seq(seq: vb_storage::EventSeq) -> EventSeq {
    EventSeq::new(seq.get())
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
    event_seq: Option<EventSeq>,
) -> Result<(), EngineError> {
    if state.run_id != run_id {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::RunMismatch {
                expected: run_id,
                actual: state.run_id,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    if state.collector_slot != collector_slot {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::SlotMismatch {
                expected: collector_slot,
                actual: state.collector_slot,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

fn validate_hydrated_page(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    expected: ListId,
) -> Result<(), EngineError> {
    if state.current_page != expected {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::CurrentPageMismatch {
                expected,
                actual: state.current_page,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

fn collect_page_from_event_value(
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    value: Option<&[u8]>,
) -> Result<Option<ListId>, EngineError> {
    match value {
        Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
            Ok(SlotValue::List(page)) => Ok(Some(page)),
            Ok(_) => Ok(None),
            Err(_) => Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            }),
        },
        None => Ok(None),
    }
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
    let mut plan = build_collect_start_plan(run, store, source, output, limit, page_size)?;
    if plan.page.is_empty() {
        return finish_empty_collect_start(
            run,
            store,
            states,
            plan.collector,
            plan.source_taint,
            done,
        );
    }
    let page = core::mem::take(&mut plan.page);
    let current_page =
        write_collected_page_with_taint(run, store, plan.collector, page, plan.source_taint)?;
    finish_collect_start_page(run, states, plan, current_page, time_limit_ms, body, done)
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

struct CollectStartPlan {
    list_id: ListId,
    source_taint: Taint,
    page: Box<[SlotValue]>,
    page_len: usize,
    item_count: usize,
    collector: SlotIdx,
    page_size: usize,
    limit: usize,
}

fn build_collect_start_plan(
    run: &RunFrame,
    store: &ValueStore,
    source: SlotIdx,
    output: Option<SlotIdx>,
    limit: u32,
    page_size: u32,
) -> Result<CollectStartPlan, EngineError> {
    let list_id = expect_list(*run.read_slot(source)?)?;
    let ps = page_size_from(page_size)?;
    validate_page_bound(ps, limit)?;
    let items = store.list(list_id)?;
    validate_item_limit(items.len(), limit)?;
    let page = copy_prefix(items, ps)?;
    let collector = match output {
        Some(slot) => slot,
        None => source,
    };
    Ok(CollectStartPlan {
        list_id,
        source_taint: run.read_taint(source)?,
        page_len: page.len(),
        item_count: items.len(),
        collector,
        page_size: ps,
        limit: usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?,
        page,
    })
}

fn finish_empty_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    collector: SlotIdx,
    source_taint: Taint,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    write_empty_collector(run, store, collector, source_taint)?;
    states.remove(run.run_id(), collector);
    jump_to(run, done)
}

fn finish_collect_start_page(
    run: &mut RunFrame,
    states: &mut CollectStates,
    plan: CollectStartPlan,
    current_page: ListId,
    time_limit_ms: Option<u64>,
    body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let cursor = checked_add_usize(0, plan.page_len, "collect cursor overflow")?;
    if cursor >= plan.item_count {
        states.remove(run.run_id(), plan.collector);
        return jump_to(run, done);
    }
    upsert_started_collect(run, states, plan, current_page, cursor, time_limit_ms)?;
    jump_to(run, body)
}

fn upsert_started_collect(
    run: &RunFrame,
    states: &mut CollectStates,
    plan: CollectStartPlan,
    current_page: ListId,
    cursor: usize,
    time_limit_ms: Option<u64>,
) -> Result<(), EngineError> {
    states.upsert(CollectPaginationState {
        run_id: run.run_id(),
        collector_slot: plan.collector,
        source: plan.list_id,
        current_page,
        cursor,
        page_size: plan.page_size,
        item_count: plan.item_count,
        limit: plan.limit,
        time_limit_ms,
        start_millis: millis_since_epoch()?,
    })
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
    let plan = build_collect_next_plan(run, store, states, collector_slot, current_id)?;
    let Some((state, page, page_len)) = plan else {
        return write_terminal_collect_page(run, store, states, collector_slot, done);
    };
    let current_page = write_collected_page(run, store, collector_slot, page)?;
    let cursor = checked_add_usize(state.cursor, page_len, "collect cursor overflow")?;
    states.upsert(CollectPaginationState {
        current_page,
        cursor,
        ..state
    })?;
    jump_to(run, body)
}

type CollectNextPlan = Option<(CollectPaginationState, Box<[SlotValue]>, usize)>;

fn build_collect_next_plan(
    run: &RunFrame,
    store: &ValueStore,
    states: &CollectStates,
    collector_slot: SlotIdx,
    current_id: ListId,
) -> Result<CollectNextPlan, EngineError> {
    let state = states.require_current_page(run.run_id(), collector_slot, current_id)?;
    check_time_limit(&state)?;
    let source_items = store.list(state.source)?;
    validate_collect_state(&state, source_items.len())?;
    validate_cursor_in_source(&state, source_items.len())?;
    if state.cursor >= state.item_count {
        return Ok(None);
    }
    let page = copy_page_range(source_items, state.cursor, state.page_size)?;
    let page_len = page.len();
    Ok(Some((state, page, page_len)))
}

fn validate_cursor_in_source(
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

fn write_terminal_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let empty_page = Vec::<SlotValue>::new().into_boxed_slice();
    let _page_id = write_collected_page(run, store, collector_slot, empty_page)?;
    states.remove(run.run_id(), collector_slot);
    jump_to(run, done)
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
