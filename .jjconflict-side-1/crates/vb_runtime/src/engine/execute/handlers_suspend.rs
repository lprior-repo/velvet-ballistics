#![forbid(unsafe_code)]

//! Suspension and action handlers: Wait/Ask, Do, ErrorHandler. These
//! handlers produce the runtime signals that suspend the drive loop
//! (`AwaitingWait`, `AwaitingAsk`, `AwaitingAction`) or route execution
//! to an error-handler body.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};

use crate::engine::action::{execute_do, execute_do_without_contract, resolve_contract};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

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
    run: &mut RunFrame,
    action: ActionId,
    input: SlotIdx,
    contracts: &[ActionContract],
    granted: &CapabilitySet,
    retry_policy: RetryPolicy,
    node_id: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(run.executed());
    if contracts.is_empty() {
        execute_do_without_contract(run, node_id, action, input, seq, granted, retry_policy)
    } else {
        execute_do(
            run,
            node_id,
            action,
            input,
            seq,
            resolve_contract(action, contracts)?,
            contracts,
            granted,
            retry_policy,
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
