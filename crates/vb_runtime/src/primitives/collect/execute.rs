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
    let current_page = write_collected_page_with_taint(
        run,
        store,
        plan.collector,
        page,
        plan.source_taint,
        plan.list_id,
        0,
    )?;
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
    let Some((state, page, page_len)) = plan else {
        return finish_collect_terminal(run, states, collector_slot, done);
    };
    let current_page = write_collected_page_with_taint(
        run,
        store,
        collector_slot,
        page,
        run.read_taint(collector_slot)?,
        state.source,
        state.cursor,
    )?;
    let cursor = checked_add_usize(state.cursor, page_len, "collect cursor overflow")?;
    states.upsert(CollectPaginationState {
        current_page,
        cursor,
        ..state
    })?;
    P::jump(run, body)
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

fn finish_collect_terminal(
    run: &mut RunFrame,
    states: &mut CollectStates,
    collector_slot: SlotIdx,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let state = states.capture_state(run.run_id(), collector_slot).ok_or(
        EngineError::InternalInvariantViolation {
            reason: "collect terminal without pagination state",
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
