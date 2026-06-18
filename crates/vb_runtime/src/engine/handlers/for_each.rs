#![forbid(unsafe_code)]

//! ForEach node handlers: iterate over collections.

use vb_core::ids::{FanoutLimit, SlotIdx, StepIdx};
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

// ── ForEach Start ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_start(
        run,
        store,
        input,
        item_slot,
        FanoutLimit::new(limit),
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

// ── ForEach Next ─────────────────────────────────────────────────

pub(crate) fn handle_for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_next(run, store, iterator_slot, body, done, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── ForEach Join ─────────────────────────────────────────────────

pub(crate) fn handle_for_each_join(
    run: &mut RunFrame,
    output: SlotIdx,
    node_output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::for_each::for_each_join(run, output, node_output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
