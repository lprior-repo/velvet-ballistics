#![forbid(unsafe_code)]
//! Action completion and ticket helpers for shard operations.

use vb_core::action::ActionTicket;
use vb_core::frame::StepState;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::CompiledNodeKind;

use crate::{RuntimeError, RuntimeResult};

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
        .and_then(|node| node.output.slot())
        .ok_or(RuntimeError::InvalidActionCompletion)
}

fn validate_ticket_attempt(
    state: &crate::shard::types::RunState,
    ticket: ActionTicket,
) -> RuntimeResult<()> {
    // Run attempt authority: ticket must match current run attempt.
    let expected = state.run_attempt.get();
    if ticket.run_attempt != expected {
        return Err(RuntimeError::StaleAttempt {
            incoming: ticket.run_attempt,
            current: expected,
        });
    }
    // Per-action attempt bounds.
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
    // Future-attempt rejection (G005): only the scheduled attempt may complete.
    if ticket.attempt > current {
        return Err(RuntimeError::InvalidActionCompletion);
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
    match node.routing.next.step() {
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
