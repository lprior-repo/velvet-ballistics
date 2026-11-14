use super::*;

pub(super) fn finish_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    plan: CollectStartPlan,
    finish: CollectStartFinishRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    if plan.page.is_empty() {
        return finish_empty_collect_start(
            run,
            store,
            states,
            plan.collector,
            plan.source_taint,
            finish.done,
        );
    }
    finish_nonempty_collect_start(run, store, states, plan, finish)
}

fn finish_nonempty_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    mut plan: CollectStartPlan,
    finish: CollectStartFinishRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current_page = write_initial_collect_page(run, store, &mut plan)?;
    finish_collect_start_page(
        run,
        states,
        plan,
        current_page,
        finish.time_limit_ms,
        finish.body,
        finish.done,
    )
}

pub(super) struct PreparedCollectSource {
    pub(super) list_id: ListId,
    pub(super) page: Box<[SlotValue]>,
    pub(super) page_size: usize,
    pub(super) item_count: usize,
}

pub(super) fn prepare_collect_source(
    run: &RunFrame,
    store: &ValueStore,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
) -> Result<PreparedCollectSource, EngineError> {
    let list_id = expect_list(*run.read_slot(source)?)?;
    let page_size = page_size_from(page_size)?;
    validate_page_bound(page_size, limit)?;
    let items = store.list(list_id)?;
    validate_item_limit(items.len(), limit)?;
    let page = copy_prefix(items, page_size)?;
    Ok(PreparedCollectSource {
        list_id,
        page,
        page_size,
        item_count: items.len(),
    })
}

pub(super) fn finish_collect_next_plan<P: crate::primitives::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    finish: CollectNextFinishRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    let CollectNextFinishRequest {
        collector_slot,
        plan,
        body,
        done,
    } = finish;
    let Some((state, page, page_len)) = plan else {
        return finish_collect_terminal(run, states, collector_slot, done);
    };
    commit_collect_next_page(
        run,
        store,
        states,
        CollectNextPageCommit::new(collector_slot, state, page, page_len),
    )?;
    P::jump(run, body)
}

struct CollectNextPageCommit {
    collector_slot: SlotIdx,
    state: CollectPaginationState,
    page: Box<[SlotValue]>,
    page_len: usize,
}

impl CollectNextPageCommit {
    fn new(
        collector_slot: SlotIdx,
        state: CollectPaginationState,
        page: Box<[SlotValue]>,
        page_len: usize,
    ) -> Self {
        Self {
            collector_slot,
            state,
            page,
            page_len,
        }
    }

    fn next_cursor(cursor: usize, page_len: usize) -> Result<usize, EngineError> {
        checked_add_usize(cursor, page_len, "collect cursor overflow")
    }
}

fn commit_collect_next_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    commit: CollectNextPageCommit,
) -> Result<(), EngineError> {
    let current_page = write_collected_page_with_taint(
        run,
        store,
        commit.collector_slot,
        commit.page,
        run.read_taint(commit.collector_slot)?,
        commit.state.source,
        commit.state.cursor,
    )?;
    let cursor = CollectNextPageCommit::next_cursor(commit.state.cursor, commit.page_len)?;
    states.upsert(CollectPaginationState {
        current_page,
        cursor,
        ..commit.state
    })?;
    Ok(())
}
pub(super) fn assemble_collect_start_plan(
    prepared: PreparedCollectSource,
    collector: SlotIdx,
    source_taint: Taint,
    limit: u32,
) -> Result<CollectStartPlan, EngineError> {
    let limit = usize::try_from(limit).map_err(|_| EngineError::CollectItemLimitExceeded)?;
    let page_len = prepared.page.len();
    Ok(CollectStartPlan {
        list_id: prepared.list_id,
        source_taint,
        page: prepared.page,
        page_len,
        item_count: prepared.item_count,
        collector,
        page_size: prepared.page_size,
        limit,
    })
}

fn write_initial_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &mut CollectStartPlan,
) -> Result<ListId, EngineError> {
    let page = core::mem::take(&mut plan.page);
    write_collected_page_with_taint(
        run,
        store,
        plan.collector,
        page,
        plan.source_taint,
        plan.list_id,
        0,
    )
}
