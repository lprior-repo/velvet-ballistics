//! Scheduling and run lifecycle management.
//!
//! This module contains the pure state-transition helpers for run management.
//! The actual Shard methods delegate to these functions.

use vb_core::action::ActionTicket;
use vb_core::engine::StepBudget;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::{RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full};
use crate::run_state::RunState;
use crate::{RuntimeError, RuntimeResult};

/// Seeds input slots in a frame with the given input values.
pub fn seed_input_slots(frame: &mut RunFrame, inputs: &[(SlotIdx, vb_core::value::SlotValue)]) -> RuntimeResult<()> {
    for (slot, value) in inputs {
        frame
            .write_slot_with_taint(*slot, *value, Taint::Clean)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    }
    Ok(())
}

/// Validates that an action completion matches the expected state.
pub fn validate_action_completion(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()> {
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

/// Advances the run state PC after an action completion.
pub fn advance_after_action_completion(state: &mut RunState, step: StepIdx) -> RuntimeResult<()> {
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

/// Records a scheduled action attempt in the run state.
pub fn record_scheduled_attempt(state: &mut RunState, ticket: ActionTicket) {
    let attempts = state.action_attempts_mut();
    if let Some(attempt) = attempts.get_mut(ticket.step.as_usize())
        && (*attempt == 0 || *attempt < ticket.attempt)
    {
        *attempt = ticket.attempt;
    }
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

/// Returns the retry policy for an action at the given step.
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
    let vb_core::value::SlotValue::I64(max_attempts) =
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
    let attempts = state.action_attempts_mut();
    let attempt = attempts
        .get_mut(ticket.step.as_usize())
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    if *attempt == 0 || *attempt < ticket.attempt {
        *attempt = ticket.attempt;
    }
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

/// Finds an error handler for a failed step.
pub fn find_error_handler_for_failure(workflow: &CompiledWorkflow, failed: StepIdx) -> Option<StepIdx> {
    if let Some(handler) = error_handler_on_node(workflow, failed, failed) {
        return Some(handler);
    }

    if failed.get() > 0 {
        let previous = StepIdx::new(failed.get().saturating_sub(1));
        if let Some(handler) = error_handler_on_node(workflow, previous, failed) {
            return Some(handler);
        }
    }

    let mut index = 0usize;
    let count = usize::from(workflow.node_count());
    while index < count {
        let Ok(raw) = u16::try_from(index) else {
            return None;
        };
        if let Some(handler) = error_handler_on_node(workflow, StepIdx::new(raw), failed) {
            return Some(handler);
        }
        index = index.checked_add(1)?;
    }

    None
}

/// Checks if a node is an error handler for the given failed step.
fn error_handler_on_node(
    workflow: &CompiledWorkflow,
    candidate: StepIdx,
    failed: StepIdx,
) -> Option<StepIdx> {
    let node = workflow.node(candidate)?;
    match node.kind {
        CompiledNodeKind::ErrorHandler { body, handler }
            if candidate == failed || body == failed =>
        {
            Some(handler)
        }
        _ => None,
    }
}

/// Returns the result slot for a finished run, if the final node has one.
pub fn result_slot_for_finished_run(state: &RunState) -> Option<SlotIdx> {
    state
        .workflow
        .node(state.frame.pc())
        .and_then(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
}

/// Creates a snapshot from run state for inspection.
pub fn snapshot_from_state(run: RunId, correlation: u64, state: &RunState) -> crate::command::InspectSnapshot {
    crate::command::InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}

/// Drives a run state with the given step budget.
pub fn drive_state(
    state: &mut RunState,
    step_budget_per_tick: u64,
) -> RuntimeEngineResult<RuntimeSignal> {
    let mut budget = StepBudget::new(step_budget_per_tick);
    drive_deterministic_full(
        &state.workflow,
        &mut state.frame,
        &mut budget,
        &mut state.store,
        &[],
        RetryPolicy::NEVER,
    )
}
