#![forbid(unsafe_code)]
//! Retry-related helpers.

use vb_core::action::ActionTicket;
use vb_core::ids::StepIdx;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledNodeKind;

use crate::engine::RetryPolicy;
use crate::shard::helpers::action::AttemptFenceError;
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
    let slot = state
        .action_attempts
        .get_mut(ticket.step.as_usize())
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    let (next, can_retry) = retry_attempt_after(Some(*slot), ticket.attempt, policy.max_attempts)?;
    *slot = next;
    Ok(can_retry)
}

// ===========================================================================
// Pure kernel — Verus/Flux binding surface
// ===========================================================================

/// Pure retry-transition kernel.
///
/// Computes the next attempt counter and whether retry is available,
/// based on the current counter, ticket attempt, and policy max.
/// Does NOT mutate state.
///
/// Panics: NEVER (checked_add is fallible, returns Err on overflow)
/// I/O: NONE
/// Allocation: NONE
pub(crate) fn retry_attempt_after(
    current: Option<u16>,
    ticket_attempt: u16,
    max_attempts: u16,
) -> Result<(u16, bool), AttemptFenceError> {
    if max_attempts == 0 || ticket_attempt == 0 || ticket_attempt > max_attempts {
        return Err(AttemptFenceError::AttemptBeyondMax {
            attempt: ticket_attempt,
            max: max_attempts,
        });
    }
    let Some(c) = current else {
        return Err(AttemptFenceError::InvalidActionCompletion);
    };
    let base = c.max(ticket_attempt);
    if base >= max_attempts {
        Ok((base, false))
    } else {
        let next = base
            .checked_add(1)
            .ok_or(AttemptFenceError::InvalidActionCompletion)?;
        Ok((next, true))
    }
}
