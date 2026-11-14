#![forbid(unsafe_code)]
//! Retry metadata and attempt accounting helpers.

use vb_core::action::ActionTicket;
use vb_core::ids::StepIdx;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledNodeKind;

use crate::engine::RetryPolicy;
use crate::{RuntimeError, RuntimeResult};

fn validate_retry_attempt(ticket: ActionTicket, policy: RetryPolicy) -> RuntimeResult<()> {
    if policy.max_attempts == 0 || ticket.attempt == 0 || ticket.attempt > policy.max_attempts {
        return Err(RuntimeError::AttemptBeyondMax {
            attempt: ticket.attempt,
            max: policy.max_attempts,
        });
    }
    Ok(())
}

/// Returns true if retry metadata exists for the given step.
pub fn retry_metadata_exists(state: &crate::shard::types::RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    if node.retry.is_some() {
        return true;
    }
    let Some(next) = node.routing.next.step() else {
        return false;
    };
    matches!(
        state.workflow.node(next).map(|next_node| &next_node.kind),
        Some(CompiledNodeKind::RetryCheck { .. })
    )
}

/// Extracts retry policy from the step's retry check node.
pub fn retry_policy_after_action(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<RetryPolicy> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    if let Some(policy) = node.retry {
        return Ok(RetryPolicy {
            max_attempts: policy.max_attempts,
            base_delay_ms: policy.delay_ms,
            exponential_backoff: false,
        });
    }
    let Some(next) = node.routing.next.step() else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        });
    };
    Ok(RetryPolicy {
        max_attempts: retry_check_max_attempts(state, next)?,
        base_delay_ms: 0,
        exponential_backoff: false,
    })
}

fn retry_check_max_attempts(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<u16> {
    let policy_slot = retry_policy_slot(state, step)?;
    let value =
        state
            .frame
            .read_slot(policy_slot)
            .map_err(|_| RuntimeError::UnsupportedOperation {
                operation: "retry_policy_slot_unreadable",
            })?;
    let attempts = retry_policy_attempts(value)?;
    if attempts == 0 {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero",
        });
    }
    Ok(attempts)
}

fn retry_policy_slot(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<vb_core::ids::SlotIdx> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.kind {
        CompiledNodeKind::RetryCheck { policy_slot, .. } => Ok(policy_slot),
        _ => Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        }),
    }
}

fn retry_policy_attempts(value: &SlotValue) -> RuntimeResult<u16> {
    match value {
        SlotValue::I64(attempts) => {
            u16::try_from(*attempts).map_err(|_| RuntimeError::UnsupportedOperation {
                operation: "retry_policy_attempts_out_of_range",
            })
        }
        _ => Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_slot_not_i64",
        }),
    }
}

/// Records a retry attempt and returns true if more retries are allowed.
pub fn record_retry_attempt(
    state: &mut crate::shard::types::RunState,
    ticket: ActionTicket,
    policy: RetryPolicy,
) -> RuntimeResult<bool> {
    validate_retry_attempt(ticket, policy)?;
    let attempt = state
        .action_attempts
        .get_mut(ticket.step.as_usize())
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    *attempt = (*attempt).max(ticket.attempt);
    if *attempt >= policy.max_attempts {
        return Ok(false);
    }
    *attempt = attempt
        .checked_add(1)
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "retry_attempt_overflow",
        })?;
    Ok(true)
}
