#![forbid(unsafe_code)]
//! Action and ticket validation helpers.

use vb_core::action::ActionTicket;
use vb_core::frame::StepState;
use vb_core::ids::SlotIdx;
use vb_core::value::Taint;
use vb_core::workflow::CompiledNodeKind;

use crate::primitives::collect::CollectStates;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::RunState;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

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
pub fn validate_action_completion(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()> {
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
pub fn action_input_slot(state: &RunState, step: StepIdx) -> RuntimeResult<SlotIdx> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.kind {
        CompiledNodeKind::Do { input, .. } => Ok(input),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

/// Returns the output slot for a suspended Do step.
pub fn action_output_slot(state: &RunState, step: StepIdx) -> RuntimeResult<SlotIdx> {
    state
        .workflow
        .node(step)
        .and_then(|node| node.output)
        .ok_or(RuntimeError::InvalidActionCompletion)
}

fn validate_ticket_attempt(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()> {
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
    // A zero current attempt means no ticket has been issued for this step yet.
    if ticket.attempt > current {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    Ok(())
}

/// Promotes an engine-issued ticket to the live per-step attempt counter.
pub fn normalize_scheduled_ticket(
    state: &RunState,
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

/// Records a scheduled action attempt.
pub fn record_scheduled_attempt(state: &mut RunState, ticket: ActionTicket) {
    if ticket.attempt == 0 {
        return;
    }
    if let Some(attempt) = state.action_attempts.get_mut(ticket.step.as_usize())
        && (*attempt == 0 || *attempt < ticket.attempt)
    {
        *attempt = ticket.attempt;
    }
}

/// Creates a new action attempts tracker.
pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

/// Creates a new run state from a compiled workflow.
pub fn make_run_state(workflow: CompiledWorkflow, run_id: RunId) -> Option<RunState> {
    let step_count = workflow.node_count();
    let slot_count = workflow.slot_count();
    let frame = RunFrame::new(run_id, workflow.entry(), step_count, slot_count).ok()?;
    Some(RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: new_action_attempts(step_count),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
    })
}
