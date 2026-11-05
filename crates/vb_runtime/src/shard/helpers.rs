#![forbid(unsafe_code)]
//! Pure helper functions for shard operations.

use vb_core::action::ActionTicket;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::primitives::collect::CollectStates;

use crate::engine::RetryPolicy;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::InspectSnapshot;

mod completion;
mod inputs;

pub use completion::{
    action_input_slot, action_output_slot, advance_after_action_completion,
    advance_after_timer_fire, normalize_scheduled_ticket, timer_registration_required,
    validate_action_completion, validate_timer_fire,
};
pub use inputs::seed_input_slots;

/// Creates a new action attempts tracker.
pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

pub fn make_run_state(
    workflow: CompiledWorkflow,
    run_id: RunId,
) -> Option<crate::shard::types::RunState> {
    let step_count = workflow.node_count();
    let slot_count = workflow.slot_count();
    let frame = RunFrame::new(run_id, workflow.entry(), step_count, slot_count).ok()?;
    Some(crate::shard::types::RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame,
        run_attempt: core::num::NonZeroU16::MIN,
        workflow,
        store: ValueStore::new(),
        action_attempts: new_action_attempts(step_count),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
    })
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
    let Some(next) = node.routing.next.step() else {
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
#[path = "helpers/tests.rs"]
mod tests;
