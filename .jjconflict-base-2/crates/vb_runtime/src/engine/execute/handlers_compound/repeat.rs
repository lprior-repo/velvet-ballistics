//! Repeat loop handlers.

use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::body_entry::BodyEntryPolicy;

pub(in crate::engine::execute) fn handle_repeat_start(
    run: &mut RunFrame,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_start(run, max_attempts, body, done, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(in crate::engine::execute) fn handle_repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_attempt(run, attempt_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(in crate::engine::execute) fn handle_repeat_check<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_check_with_policy::<P>(run, attempt_slot, done, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(in crate::engine::execute) fn handle_repeat_finish(
    run: &mut RunFrame,
    result: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_finish(run, result, output, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}
