//! Collect pagination primitive handlers.

use std::collections::HashMap;
use std::time::SystemTime;

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

use super::helpers::{expect_list, jump_to, jump_to_next, require_output};

/// Per-run pagination state stored in a side table keyed by (RunId, SlotIdx).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
mod tests {
    use super::*;
    use crate::test_harness::list_in_slot;
    use vb_core::value_store::ValueStore;

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 8)
    }

    fn fresh_states() -> CollectStates {
        CollectStates::new()
    }

    fn assert_slot_list_items(
        run: &RunFrame,
        store: &ValueStore,
        slot: SlotIdx,
        expected: &[SlotValue],
    ) {
        match *run
            .read_slot(slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::List(id) => {
                let items = store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("list read must succeed"));
                assert_eq!(items, expected);
            }
            other => {
                assert_eq!(other, SlotValue::Null);
            }
        }
    }

    #[test]
    fn collect_start_initializes_collector() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
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
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(output),
            None,
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
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(10)]);
        let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_next_advances_to_next_page_while_page_has_items() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(5), SlotValue::I64(6), SlotValue::I64(7)],
        );
        let start = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector),
            None,
        );
        assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
        assert_slot_list_items(
            &run,
            &store,
            collector,
            &[SlotValue::I64(5), SlotValue::I64(6)],
        );
        let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(7)]);
    }

    #[test]
    fn collect_finish_materializes_output() {
        let mut run = fresh_frame();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(collector, SlotValue::I64(99))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));
        let result = collect_finish(
            &mut run,
            &mut states,
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

    #[test]
    fn collect_start_returns_error_when_source_is_not_list() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        run.write_slot(source, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
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
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            3,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
        match result {
            Err(EngineError::CollectItemLimitExceeded) => {}
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn collect_start_returns_error_when_output_missing() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            None,
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            0,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            done,
            Some(output),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_next_returns_done_when_remaining_empty() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, collector, vec![]);
        let result = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            done,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_finish_returns_error_when_output_missing() {
        let mut run = fresh_frame();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::I64(1))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let result = collect_finish(
            &mut run,
            &mut states,
            collector,
            None,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
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
        let mut run = fresh_frame();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        run.write_slot(collector, SlotValue::I64(1))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let result = collect_finish(
            &mut run,
            &mut states,
            collector,
            Some(output),
            None,
            StepIdx::ZERO,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let result = collect_page(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        let before = run.executed();
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_increments_executed_with_pagination_state() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );
        let start = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            1,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(collector),
            None,
        );
        assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
        let before = run.executed();
        let result = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_page_increments_executed_counter() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
        let before = run.executed();
        let result = collect_page(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_finish_increments_executed_counter() {
        let mut run = fresh_frame();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        run.write_slot(collector, SlotValue::I64(99))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        let result = collect_finish(
            &mut run,
            &mut states,
            collector,
            Some(output),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn collect_next_rejects_nonempty_current_page_without_state() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        list_in_slot(
            &mut run,
            &mut store,
            collector,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        let result = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
        assert_eq!(
            result,
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state missing",
            })
        );
        assert_eq!(run.pc(), StepIdx::ZERO);
    }

    #[test]
    fn collect_next_returns_error_when_not_list() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let collector = SlotIdx::new(0);
        run.write_slot(collector, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let result = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector,
            StepIdx::new(1),
            StepIdx::new(2),
        );
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
    fn collect_start_zero_items_with_nonzero_limit_goes_to_done() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            done,
            Some(output),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn collect_start_page_size_zero_returns_error_even_for_empty_list() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(&mut run, &mut store, source, vec![]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            0,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let body = StepIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            3,
            2,
            body,
            StepIdx::new(2),
            Some(output),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn collect_start_items_exceeding_limit_by_one() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
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
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            3,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
        match result {
            Err(EngineError::CollectItemLimitExceeded) => {}
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn collect_start_first_page_smaller_than_total() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
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
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(42), SlotValue::I64(99)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            10,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
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
    fn collect_next_progresses_pages_then_jumps_done() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        let start = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector),
            None,
        );
        assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
        assert_slot_list_items(
            &run,
            &store,
            collector,
            &[SlotValue::I64(1), SlotValue::I64(2)],
        );
        let next = collect_next(&mut run, &mut store, &mut states, collector, body, done);
        assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(3)]);
        let finished = collect_next(&mut run, &mut store, &mut states, collector, body, done);
        assert_eq!(finished, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
        assert_slot_list_items(&run, &store, collector, &[]);
    }

    #[test]
    fn collect_start_null_source_returns_type_mismatch() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        run.write_slot(source, SlotValue::Null)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
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
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            1,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
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
        let mut states = fresh_states();
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
            &mut states,
            source,
            1,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
        assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
        assert_eq!(run.pc(), StepIdx::ZERO);
    }

    #[test]
    fn collect_start_page_size_u32_max_returns_error() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            u32::MAX,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
        assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
    }

    #[test]
    fn collect_start_page_size_at_limit_boundary() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            2,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), StepIdx::new(1));
    }

    #[test]
    fn collect_start_page_size_exactly_one_over_limit() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            1,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(1)),
            None,
        );
        assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
    }

    #[test]
    fn collect_start_with_time_limit_stores_limit_in_state() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
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
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector),
            Some(60000),
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        let state = states.entries.values().next();
        assert!(state.is_some());
        assert_eq!(state.unwrap().time_limit_ms, Some(60000));
    }

    #[test]
    fn collect_start_without_time_limit_stores_none() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );
        let result = collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector),
            None,
        );
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        let state = states.entries.values().next();
        assert!(state.is_some());
        assert_eq!(state.unwrap().time_limit_ms, None);
    }

    #[test]
    fn check_time_limit_returns_error_when_exceeded() {
        let state = CollectPaginationState {
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(0),
            source: ListId::new(0),
            current_page: ListId::new(0),
            cursor: 0,
            page_size: 10,
            item_count: 10,
            limit: 100,
            time_limit_ms: Some(1),
            start_millis: 0,
        };
        assert_eq!(
            check_time_limit(&state),
            Err(EngineError::CollectTimeLimitExceeded)
        );
    }

    #[test]
    fn check_time_limit_ok_when_not_exceeded() {
        let now = millis_since_epoch();
        let state = CollectPaginationState {
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(0),
            source: ListId::new(0),
            current_page: ListId::new(0),
            cursor: 0,
            page_size: 10,
            item_count: 10,
            limit: 100,
            time_limit_ms: Some(60000),
            start_millis: now,
        };
        assert_eq!(check_time_limit(&state), Ok(()));
    }

    #[test]
    fn check_time_limit_ok_when_no_limit_set() {
        let state = CollectPaginationState {
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(0),
            source: ListId::new(0),
            current_page: ListId::new(0),
            cursor: 0,
            page_size: 10,
            item_count: 10,
            limit: 100,
            time_limit_ms: None,
            start_millis: 0,
        };
        assert_eq!(check_time_limit(&state), Ok(()));
    }
}
