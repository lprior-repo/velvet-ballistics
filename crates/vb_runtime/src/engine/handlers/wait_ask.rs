#![forbid(unsafe_code)]

//! Wait/Ask node handlers: synchronization and user-prompt primitives.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::frame::RunFrame;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

// ── Wait Until ───────────────────────────────────────────────────

pub(crate) fn handle_wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_until(run, deadline_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── Wait Event ───────────────────────────────────────────────────

pub(crate) fn handle_wait_event(
    run: &mut RunFrame,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_event(run, event, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── Ask ──────────────────────────────────────────────────────────

pub(crate) fn handle_ask(
    run: &mut RunFrame,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask(run, prompt, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── Ask Resume ───────────────────────────────────────────────────

pub(crate) fn handle_ask_resume(
    run: &mut RunFrame,
    answer: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask_resume(run, answer, output, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}
