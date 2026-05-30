#![forbid(unsafe_code)]
//! Pure helper functions for shard operations.

use vb_core::action::ActionTicket;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::RetryPolicy;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{InspectSnapshot, PendingTimer, PendingTimerKind};

/// Seeds input slots on a frame before deterministic execution.
pub fn seed_input_slots(
    frame: &mut RunFrame,
    inputs: &[(SlotIdx, SlotValue)],
) -> RuntimeResult<()> {
    for (slot, value) in inputs {
        frame
            .write_slot_with_taint(*slot, *value, Taint::Clean)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    }
    Ok(())
}

/// Validates that an action completion matches the expected ticket.
pub fn validate_action_completion(
    state: &crate::shard::types::RunState,
    ticket: ActionTicket,
) -> RuntimeResult<()> {
    validate_ticket_attempt(state, ticket)?;
    if state.frame.step_state(ticket.step) != Ok(StepState::Running) {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    let Some(node) = state.workflow.node(ticket.step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.kind {
        CompiledNodeKind::Do { action, .. } if action == ticket.action => Ok(()),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

/// Returns the input slot for a suspended Do step.
pub fn action_input_slot(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<SlotIdx> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.kind {
        CompiledNodeKind::Do { input, .. } => Ok(input),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

/// Returns the output slot for a suspended Do step.
pub fn action_output_slot(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<SlotIdx> {
    state
        .workflow
        .node(step)
        .and_then(|node| node.output)
        .ok_or(RuntimeError::InvalidActionCompletion)
}

fn validate_ticket_attempt(
    state: &crate::shard::types::RunState,
    ticket: ActionTicket,
) -> RuntimeResult<()> {
    if ticket.attempt == 0 || ticket.capacity == 0 || ticket.attempt > ticket.capacity {
        return Err(RuntimeError::AttemptBeyondMax {
            attempt: ticket.attempt,
            max: ticket.capacity,
        });
    }
    let current = state
        .action_attempts
        .get(ticket.step.as_usize())
        .copied()
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    if ticket.attempt < current {
        return Err(RuntimeError::StaleAttempt {
            incoming: ticket.attempt,
            current,
        });
    }
    Ok(())
}

/// Promotes an engine-issued ticket to the live per-step attempt counter.
pub fn normalize_scheduled_ticket(
    state: &crate::shard::types::RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionTicket> {
    let current = state
        .action_attempts
        .get(ticket.step.as_usize())
        .copied()
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    let attempt = current.max(ticket.attempt).max(1);
    if ticket.capacity == 0 || attempt > ticket.capacity {
        return Err(RuntimeError::AttemptBeyondMax {
            attempt,
            max: ticket.capacity,
        });
    }
    Ok(ActionTicket { attempt, ..ticket })
}

/// Advances PC after an action completes successfully.
pub fn advance_after_action_completion(
    state: &mut crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.next {
        Some(next) => {
            state
                .frame
                .set_pc(next)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(())
        }
        None => Ok(()),
    }
}

/// Returns true if a timer must be registered for the given step.
pub fn timer_registration_required(state: &crate::shard::types::RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    match node.kind {
        CompiledNodeKind::WaitUntil { .. } => true,
        CompiledNodeKind::WaitEvent { timeout_slot, .. }
        | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.is_some(),
        _ => false,
    }
}

/// Advances state after a timer fires.
pub fn advance_after_timer_fire(
    state: &mut crate::shard::types::RunState,
    timer: PendingTimer,
) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) {
        (
            PendingTimerKind::Wait,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. },
        )
        | (PendingTimerKind::Ask, CompiledNodeKind::Ask { .. }) => {}
        _ => return Err(RuntimeError::InvalidTimerFire),
    }
    state
        .frame
        .mark_running(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    state
        .frame
        .mark_succeeded(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    let Some(next) = node.next else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    state
        .frame
        .set_pc(next)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    Ok(())
}

/// Creates a new action attempts tracker.
pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

/// Records a scheduled action attempt.
pub fn record_scheduled_attempt(state: &mut crate::shard::types::RunState, ticket: ActionTicket) {
    if ticket.attempt == 0 {
        return;
    }
    if let Some(attempt) = state.action_attempts.get_mut(ticket.step.as_usize())
        && (*attempt == 0 || *attempt < ticket.attempt)
    {
        *attempt = ticket.attempt;
    }
}

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
    let Some(next) = node.next else {
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

/// Finds the error handler step and error slot for a failed step.
pub fn find_error_handler_for_failure(
    workflow: &CompiledWorkflow,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    if let Some(result) = error_handler_on_node(workflow, failed, failed) {
        return Some(result);
    }

    if failed.get() > 0 {
        let previous = StepIdx::new(failed.get().saturating_sub(1));
        if let Some(result) = error_handler_on_node(workflow, previous, failed) {
            return Some(result);
        }
    }

    let mut index = 0usize;
    let count = usize::from(workflow.node_count());
    while index < count {
        let Ok(raw) = u16::try_from(index) else {
            return None;
        };
        if let Some(result) = error_handler_on_node(workflow, StepIdx::new(raw), failed) {
            return Some(result);
        }
        index = index.checked_add(1)?;
    }

    None
}

fn error_handler_on_node(
    workflow: &CompiledWorkflow,
    candidate: StepIdx,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    let node = workflow.node(candidate)?;
    match node.kind {
        CompiledNodeKind::ErrorHandler {
            body,
            handler,
            error_slot,
        } if candidate == failed || body == failed => Some((handler, error_slot)),
        _ => None,
    }
}

/// Returns the result slot for a finished run.
pub fn result_slot_for_finished_run(state: &crate::shard::types::RunState) -> Option<SlotIdx> {
    state
        .workflow
        .node(state.frame.pc())
        .and_then(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
}

/// Creates a snapshot from run state.
pub fn snapshot_from_state(
    run: RunId,
    correlation: u64,
    state: &crate::shard::types::RunState,
) -> InspectSnapshot {
    InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}


#[cfg(test)]
#[path = "shard/helpers/tests.rs"]
mod tests;
