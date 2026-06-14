#![forbid(unsafe_code)]

//! Action execution helpers for runtime engine.

use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket,
    Idempotency, propagate_action_taint,
};
use vb_core::capability::CapabilitySet;
use vb_core::engine::EngineError;
use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::admission::check_capability;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

#[allow(clippy::too_many_arguments)]
pub fn execute_do(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    _contract: &ActionContract,
    registry_contracts: &[ActionContract],
    granted: &CapabilitySet,
    retry_policy: RetryPolicy,
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

    for required in resolved.required_capabilities.iter() {
        if let Err(crate::admission::AdmissionError::CapabilityDenied {
            required: req,
            granted: grant,
            ..
        }) = check_capability(action, required, granted)
        {
            return Err(RuntimeEngineError::Core(EngineError::CapabilityDenied {
                action,
                required: req,
                granted: grant,
            }));
        }
    }

    let output_taint = propagate_action_taint(resolved.idempotency, input_taint);

    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
        capacity: retry_policy.max_attempts,
        ..Default::default()
    };

    if output_taint == Taint::Clean && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    Ok(RuntimeSignal::AwaitingAction(ticket))
}

pub fn execute_do_without_contract(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    _seq: SeqNo,
    granted: &CapabilitySet,
    _retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    // BH-FIX: Even without a contract, we must read input taint and enforce
    // taint checking. Without a contract we assume the most conservative
    // idempotency (DeterministicPure), which means secret inputs are rejected.
    // Uninitialized slots are treated as Clean (no data = no taint).
    let input_taint = match run.read_taint(input) {
        Ok(t) => t,
        Err(CoreError::SlotUninitialized { .. }) => Taint::Clean,
        Err(e) => return Err(RuntimeEngineError::Core(e)),
    };
    if input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    let required = vb_core::capability::Capability::new("__contract_required__".into(), action);
    Err(RuntimeEngineError::Core(EngineError::CapabilityDenied {
        action,
        required,
        granted: granted.clone(),
    }))
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
///
/// When a retryable failure occurs, the original ticket is used to build a
/// retry ticket with the correct action ID, incremented sequence number,
/// incremented attempt count, and recomputed idempotency key.
pub fn resume_action_outcome(
    original_ticket: &ActionTicket,
    outcome: ActionOutcome,
    _contract: &ActionContract,
) -> RuntimeEngineResult<RuntimeSignal> {
    match outcome {
        ActionOutcome::Ready(_ready) => Ok(RuntimeSignal::Continue),
        ActionOutcome::Suspended(ticket) => Ok(RuntimeSignal::AwaitingAction(ticket)),
        ActionOutcome::Failed(failure) => {
            if failure.retry_policy == vb_core::action::RetryPolicy::Retryable
                && original_ticket.attempt < original_ticket.capacity
            {
                let next_seq =
                    original_ticket
                        .seq
                        .checked_add(1)
                        .ok_or(RuntimeEngineError::Core(
                            EngineError::InternalInvariantViolation {
                                reason: "seq_overflow_on_retry",
                            },
                        ))?;
                let next_attempt =
                    original_ticket
                        .attempt
                        .checked_add(1)
                        .ok_or(RuntimeEngineError::Core(
                            EngineError::InternalInvariantViolation {
                                reason: "attempt_overflow_on_retry",
                            },
                        ))?;
                let idempotency_key =
                    compute_idempotency_key(original_ticket.run, next_seq, original_ticket.action);
                Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                    run: original_ticket.run,
                    step: original_ticket.step,
                    seq: next_seq,
                    action: original_ticket.action,
                    attempt: next_attempt,
                    idempotency_key,
                    capacity: original_ticket.capacity,
                    ..Default::default()
                }))
            } else if failure.retry_policy == vb_core::action::RetryPolicy::Retryable {
                Err(RuntimeEngineError::RetryExhausted {
                    action: original_ticket.action,
                    attempts: original_ticket.attempt,
                })
            } else {
                Err(RuntimeEngineError::Core(
                    EngineError::UnsupportedPrimitive {
                        primitive: "action_failed_non_retryable",
                    },
                ))
            }
        }
        // Handle any future ActionOutcome variants as an internal error.
        #[allow(unreachable_code)]
        _ => Err(RuntimeEngineError::Core(
            EngineError::InternalInvariantViolation {
                reason: "unknown_action_outcome",
            },
        )),
    }
}

/// Computes a deterministic idempotency key from run, sequence, and action.
///
/// Uses wrapping multiply-add hashing (FNV-1a-inspired) to mix all three
/// inputs into a u128 without bit-field overlap or silent fallback degradation.
pub fn compute_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    vb_core::action::compute_action_idempotency_key(run, seq, action)
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
