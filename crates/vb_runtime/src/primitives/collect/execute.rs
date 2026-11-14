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
    let request = CollectStartRequest {
        source,
        output,
        limit,
        page_size,
        time_limit_ms,
        body,
        done,
    };
    collect_start_with_request(run, store, states, request)
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

struct CollectStartRequest {
    source: SlotIdx,
    output: Option<SlotIdx>,
    limit: u32,
    page_size: u32,
    time_limit_ms: Option<u64>,
    body: StepIdx,
    done: StepIdx,
}

struct CollectStartFinishRequest {
    time_limit_ms: Option<u64>,
    body: StepIdx,
    done: StepIdx,
}

fn collect_start_with_request(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    request: CollectStartRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    let plan = build_collect_start_plan(run, store, &request)?;
    finish_collect_start(
        run,
        store,
        states,
        plan,
        CollectStartFinishRequest {
            time_limit_ms: request.time_limit_ms,
            body: request.body,
            done: request.done,
        },
    )
}

fn build_collect_start_plan(
    run: &RunFrame,
    store: &ValueStore,
    request: &CollectStartRequest,
) -> Result<CollectStartPlan, EngineError> {
    let prepared =
        prepare_collect_source(run, store, request.source, request.limit, request.page_size)?;
    let collector = request.output.unwrap_or(request.source);
    let source_taint = run.read_taint(request.source)?;
    assemble_collect_start_plan(prepared, collector, source_taint, request.limit)
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
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let cursor = checked_add_usize(0, plan.page_len, "collect cursor overflow")?;
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
pub(crate) fn collect_next_with_policy<P: super::body_entry::BodyEntryPolicy>(
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
        return accept_empty_collect_page(run, states, collector_slot, current_id, done);
    }
    let plan = build_collect_next_plan(run, store, states, collector_slot, current_id)?;
    finish_collect_next_plan::<P>(
        run,
        store,
        states,
        CollectNextFinishRequest {
            collector_slot,
            plan,
            body,
            done,
        },
    )
}

/// Legacy entry-only state reset; this API does not validate a complete region.
pub fn collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    collect_next_with_policy::<super::body_entry::LegacyEntry>(
        run,
        store,
        states,
        collector_slot,
        body,
        done,
    )
}

fn accept_empty_collect_page(
    run: &mut RunFrame,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    current_id: ListId,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    if states.capture_state(run.run_id(), collector_slot).is_some() {
        let state = states.require_current_page(run.run_id(), collector_slot, current_id)?;
        if state.cursor < state.item_count {
            return Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect empty page before terminal cursor",
            });
        }
        states.remove(run.run_id(), collector_slot);
    }
    jump_to(run, done)
}

type CollectNextPlan = Option<(CollectPaginationState, Box<[SlotValue]>, usize)>;
struct CollectNextFinishRequest {
    collector_slot: SlotIdx,
    plan: CollectNextPlan,
    body: StepIdx,
    done: StepIdx,
}

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
            reason: "collect cursor beyond source items".into(),
        });
    }
    Ok(())
}

fn finish_collect_terminal(
    run: &mut RunFrame,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let state = states.capture_state(run.run_id(), collector_slot).ok_or(
        EngineError::InternalInvariantViolation {
            reason: "collect terminal without pagination state".into(),
        },
    )?;
    let taint = run.read_taint(collector_slot)?;
    run.write_slot_with_taint(collector_slot, SlotValue::List(state.source), taint)?;
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
#[path = "execute_helpers.rs"]
mod execute_helpers;
use execute_helpers::{
    assemble_collect_start_plan, finish_collect_next_plan, finish_collect_start,
    prepare_collect_source,
};
