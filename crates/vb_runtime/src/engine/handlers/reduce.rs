#![forbid(unsafe_code)]

//! Reduce node handlers: fold over collections with accumulator.

use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

// ── Reduce Start ─────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_start(
        plan,
        run,
        store,
        input,
        accumulator,
        initial,
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

// ── Reduce Next ──────────────────────────────────────────────────

pub(crate) fn handle_reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_next(
        run,
        store,
        iterator_slot,
        accumulator,
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

// ── Reduce Finish ────────────────────────────────────────────────

pub(crate) fn handle_reduce_finish(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::reduce::reduce_finish(run, accumulator, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
