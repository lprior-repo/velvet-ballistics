#![forbid(unsafe_code)]

//! Retry-budget handlers. Owns the attempt-counter read and the
//! in-handler write-back that advances the retry budget on each
//! `RetryCheck` visit.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use crate::engine::action::execute_retry_check;
use crate::engine::execute::handlers::read_attempt_from_slot;
use crate::engine::types::{
    RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};

/// Handles a `RetryCheck` node. RE-003: an uninitialized policy slot is
/// the first-visit case (attempt = 0) — the counter is advanced via
/// in-handler write-back so subsequent visits can terminate even when
/// the body does not write back. `checked_add` returns a typed overflow
/// error rather than silently saturating.
pub(super) fn handle_retry_check(
    run: &mut RunFrame,
    policy_slot: SlotIdx,
    body: StepIdx,
    exhausted: StepIdx,
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let current_attempt = read_attempt_from_slot(run, policy_slot)?.unwrap_or(0);
    let target = execute_retry_check(current_attempt, retry_policy, body, exhausted);
    let next_attempt = current_attempt
        .checked_add(1)
        .ok_or(RuntimeEngineError::Core(
            EngineError::InternalInvariantViolation {
                reason: "retry_attempt_overflow",
            },
        ))?;
    run.write_slot(policy_slot, SlotValue::I64(i64::from(next_attempt)))
        .map_err(RuntimeEngineError::Core)?;
    run.set_pc(target).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}