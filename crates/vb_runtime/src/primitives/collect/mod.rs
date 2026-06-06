#![forbid(unsafe_code)]
//! Collect pagination primitive handlers.

#[cfg(kani)]
mod kani;
mod state;
mod validation;

pub use state::{CollectPaginationState, CollectStates};
pub use vb_core::errors::CollectPageOrderViolationKind;

use std::time::SystemTime;

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_storage::JournalEvent;

use super::helpers::{expect_list, jump_to, jump_to_body, jump_to_next, require_output};
use state::CollectPaginationState as State;
use validation::{
    checked_add_usize, copy_page_range, copy_prefix, page_size_from, validate_collect_state,
    validate_cursor_in_source, validate_item_limit, validate_page_bound, write_collected_page,
    write_collected_page_with_taint, write_empty_collector,
};

/// Builds collect pagination state from durable journal events recovered for a run.
pub fn hydrate_collect_states_from_recovered_journal(
    events: &[JournalEvent],
) -> Result<CollectStates, EngineError> {
    let mut states = CollectStates::new();
    states.hydrate_journal_events(events)?;
    Ok(states)
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
    jump_to_body(run, body)
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
    if plan.page_len >= plan.item_count {
        states.remove(run.run_id(), plan.collector);
        return jump_to(run, done);
    }
    upsert_started_collect(run, states, &plan, current_page, time_limit_ms)?;
    jump_to(run, body)
}

fn upsert_started_collect(
    run: &RunFrame,
    states: &mut CollectStates,
    plan: &CollectStartPlan,
    current_page: ListId,
    time_limit_ms: Option<u64>,
) -> Result<(), EngineError> {
    let key = (run.run_id(), plan.collector);
    states.upsert(State {
        run_id: run.run_id(),
        collector_slot: plan.collector,
        source: plan.list_id,
        current_page,
        cursor: plan.page_len,
        page_size: plan.page_size,
        item_count: plan.item_count,
        limit: plan.limit,
        time_limit_ms,
        start_millis: match states.entries.get(&key) {
            Some(e) if e.from_journal => e.start_millis,
            _ => millis_since_epoch()?,
        },
        from_journal: matches!(states.entries.get(&key), Some(e) if e.from_journal),
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
    states.upsert(State {
        current_page,
        cursor,
        ..state
    })?;
    jump_to_body(run, body)
}

type CollectNextPlan = Option<(State, Box<[SlotValue]>, usize)>;

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

fn check_time_limit(state: &State) -> Result<(), EngineError> {
    // Skip wall-clock check during replay to preserve deterministic replay.
    // The original timeout outcome was recorded in the journal; re-checking
    // live wall-clock during replay would produce different elapsed time
    // than the original execution, breaking AC-3.
    if state.from_journal {
        return Ok(());
    }
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

#[cfg(test)]
mod tests;
