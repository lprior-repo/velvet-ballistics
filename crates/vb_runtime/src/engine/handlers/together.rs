#![forbid(unsafe_code)]

//! Together node handlers: fan-out to parallel branches and join results.

use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

// ── Together Start ───────────────────────────────────────────────

pub(crate) fn handle_together_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branches: &[StepIdx],
    join: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_start(run, store, branches, join, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── Together Branch ──────────────────────────────────────────────

pub(crate) fn handle_together_branch(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch: u16,
    entry: StepIdx,
    join: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_branch(
        run,
        store,
        branch,
        entry,
        join,
        accumulator,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

// ── Together Join ────────────────────────────────────────────────

pub(crate) fn handle_together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch_count: u16,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::together::together_join(
        run,
        store,
        branch_count,
        accumulator,
        output,
        next,
        step,
    ) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
