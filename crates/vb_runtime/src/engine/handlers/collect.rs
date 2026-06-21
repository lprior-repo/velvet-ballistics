#![forbid(unsafe_code)]

//! Collect node handlers: paginated collection accumulation.

use vb_core::frame::RunFrame;
use vb_core::ids::{SeqNo, SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

// ── Collect Start ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let run_id = run.run_id();
    crate::primitives::collect::collect_start(
        run, store, cs, source, limit, page_size, body, done, output, None,
    )
    .map_err(RuntimeEngineError::Core)
    .map(|signal| runtime_from_core(run_id, SeqNo::ZERO, signal))
}

// ── Collect Page ─────────────────────────────────────────────────

pub(crate) fn handle_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let run_id = run.run_id();
    crate::primitives::collect::collect_page(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(|signal| runtime_from_core(run_id, SeqNo::ZERO, signal))
}

// ── Collect Next ─────────────────────────────────────────────────

pub(crate) fn handle_collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let run_id = run.run_id();
    crate::primitives::collect::collect_next(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(|signal| runtime_from_core(run_id, SeqNo::ZERO, signal))
}

// ── Collect Finish ───────────────────────────────────────────────

pub(crate) fn handle_collect_finish(
    run: &mut RunFrame,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let run_id = run.run_id();
    match crate::primitives::collect::collect_finish(run, cs, collector_slot, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(run_id, SeqNo::ZERO, signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
