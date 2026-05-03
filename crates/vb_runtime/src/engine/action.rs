#![forbid(unsafe_code)]

//! Action execution helpers for runtime engine.

use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket,
    Idempotency, propagate_action_taint,
};
use vb_core::engine::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

/// Backward-compatible execute_do.
pub fn execute_do(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    _contract: &ActionContract,
    registry_contracts: &[ActionContract],
) -> RuntimeEngineResult<RuntimeSignal> {
    let action_index = usize::from(action.get());
    let resolved = registry_contracts
        .get(action_index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })?;

    let input_taint = run.read_taint(input).map_err(RuntimeEngineError::Core)?;
    if resolved.idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    let output_taint = propagate_action_taint(resolved.idempotency, input_taint);

    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };

    if output_taint == Taint::Clean && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    Ok(RuntimeSignal::AwaitingAction(ticket))
}

#[allow(clippy::unnecessary_wraps)]
pub fn execute_do_without_contract(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    _input: SlotIdx,
    seq: SeqNo,
) -> RuntimeEngineResult<RuntimeSignal> {
    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };
    Ok(RuntimeSignal::AwaitingAction(ticket))
}

/// Backward-compatible execute_retry_check.
pub fn execute_retry_check(
    current_attempt: u16,
    policy: RetryPolicy,
    body: StepIdx,
    exhausted: StepIdx,
) -> StepIdx {
    if current_attempt < policy.max_attempts {
        body
    } else {
        exhausted
    }
}

/// Backward-compatible execute_error_handler.
pub fn execute_error_handler(failure: &ActionFailure, handler: StepIdx, body: StepIdx) -> StepIdx {
    if failure.retry_policy == vb_core::action::RetryPolicy::Retryable
        || failure.code != ActionFailureCode::Unknown
    {
        handler
    } else {
        body
    }
}

/// Resumes an action outcome into the run frame.
pub fn resume_action_outcome(
    run: &mut RunFrame,
    outcome: &ActionOutcome,
) -> RuntimeEngineResult<RuntimeSignal> {
    match outcome {
        ActionOutcome::Ready(ready) => {
            run.write_slot_with_taint(ready.output_slot, ready.value, ready.taint)
                .map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        ActionOutcome::Suspended(ticket) => Ok(RuntimeSignal::AwaitingAction(*ticket)),
        ActionOutcome::Failed(failure) => {
            let step = run.pc();
            if failure.retry_policy == vb_core::action::RetryPolicy::Retryable {
                Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                    run: run.run_id(),
                    step,
                    seq: SeqNo::ZERO,
                    action: ActionId::new(0),
                    attempt: 1,
                    idempotency_key: 0,
                }))
            } else {
                Err(RuntimeEngineError::Core(
                    EngineError::UnsupportedPrimitive {
                        primitive: "action_failed_non_retryable",
                    },
                ))
            }
        }
    }
}

/// Computes a deterministic idempotency key from run, sequence, and action.
pub fn compute_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get()) << 64;
    let action_part = u128::from(u32::from(action.get())) << 80;
    match run_part.checked_add(seq_part) {
        Some(combined) => match combined.checked_add(action_part) {
            Some(key) => key,
            None => run_part,
        },
        None => run_part,
    }
}

/// Resolves an action contract from the registry.
pub fn resolve_contract(
    action: ActionId,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<&ActionContract> {
    let index = usize::from(action.get());
    contracts
        .get(index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })
        .map_err(RuntimeEngineError::Action)
}
