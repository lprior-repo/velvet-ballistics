#![forbid(unsafe_code)]
//! Retry-related helpers.

use vb_core::action::ActionTicket;
use vb_core::ids::StepIdx;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledNodeKind;

use crate::engine::RetryPolicy;
use crate::shard::types::RunState;
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
pub fn retry_metadata_exists(state: &RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    let Some(next) = node.next else {
        return false;
    };
    matches!(
        state.workflow.node(next).map(|next_node| &next_node.kind),
        Some(CompiledNodeKind::RetryCheck { .. })
    )
}

/// Extracts retry policy from the step's retry check node.
pub fn retry_policy_after_action(state: &RunState, step: StepIdx) -> RuntimeResult<RetryPolicy> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    let Some(next) = node.next else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        });
    };
    let Some(retry_node) = state.workflow.node(next) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    let CompiledNodeKind::RetryCheck { policy_slot, .. } = retry_node.kind else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        });
    };
    let SlotValue::I64(max_attempts) =
        *state
            .frame
            .read_slot(policy_slot)
            .map_err(|_| RuntimeError::UnsupportedOperation {
                operation: "retry_policy_slot_unreadable",
            })?
    else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_slot_not_i64",
        });
    };
    let max_attempts =
        u16::try_from(max_attempts).map_err(|_| RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_out_of_range",
        })?;
    if max_attempts == 0 {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero",
        });
    }
    Ok(RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    })
}

/// Records a retry attempt and returns true if more retries are allowed.
pub fn record_retry_attempt(
    state: &mut RunState,
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
