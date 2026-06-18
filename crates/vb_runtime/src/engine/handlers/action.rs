#![forbid(unsafe_code)]

//! Action execution handlers: do and retry-check.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::frame::RunFrame;

use crate::engine::action::{
    execute_do, execute_do_without_contract, execute_retry_check, resolve_contract,
};
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

use super::util::read_attempt_from_slot;

// ── Do ───────────────────────────────────────────────────────────

pub(crate) fn handle_do(
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

// ── RetryCheck ───────────────────────────────────────────────────

pub(crate) fn handle_retry_check(
    run: &mut RunFrame,
    policy_slot: SlotIdx,
    body: StepIdx,
    exhausted: StepIdx,
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let current_attempt = read_attempt_from_slot(run, policy_slot)?;
    let target = execute_retry_check(current_attempt, retry_policy, body, exhausted);
    run.set_pc(target).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}
