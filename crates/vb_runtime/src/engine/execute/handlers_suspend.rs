#![forbid(unsafe_code)]

//! Suspension and action handlers: Wait/Ask, Do, ErrorHandler.
//! These produce runtime signals that suspend the drive loop
//! or route execution to an error-handler body.

use crate::engine::execute::dispatch::action_signal::ActionDispatchContext;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};

use crate::engine::action::{execute_do, execute_do_without_contract, resolve_contract};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

// ── Wait/Ask handlers ────────────────────────────────────────────
pub(super) fn handle_wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_until(run, deadline_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_wait_event(
    run: &mut RunFrame,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_event(run, event, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_ask(
    run: &mut RunFrame,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask(run, prompt, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_ask_resume(
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

// ── Do handler ───────────────────────────────────────────────────
pub(super) fn handle_do(
    ctx: &ActionDispatchContext<'_>,
    action: ActionId,
    input: SlotIdx,
    node_id: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(ctx.run.executed());
    if ctx.contracts.is_empty() {
        execute_do_without_contract(
            ctx.run,
            node_id,
            action,
            input,
            seq,
            ctx.granted,
            ctx.retry_policy,
        )
    } else {
        execute_do(
            ctx.run,
            node_id,
            action,
            input,
            seq,
            resolve_contract(action, ctx.contracts)?,
            ctx.contracts,
            ctx.granted,
            ctx.retry_policy,
            ctx.state_run_attempt,
        )
    }
}

// ── ErrorHandler handler ─────────────────────────────────────────
pub(super) fn handle_error_handler(
    run: &mut RunFrame,
    handler_body: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    run.set_pc(handler_body).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}
