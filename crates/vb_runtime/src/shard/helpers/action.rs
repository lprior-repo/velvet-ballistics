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

/// Error variants for attempt-fence validation kernels.
/// Mirrors the decision logic of RuntimeError without coupling to the full
/// RuntimeError type, making these suitable for pure Verus spec binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptFenceError {
    StaleAttempt { incoming: u16, current: u16 },
    AttemptBeyondMax { attempt: u16, max: u16 },
    InvalidActionCompletion,
}

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
    let current = state.action_attempts.get(ticket.step.as_usize()).copied();
    classify_ticket_attempt(current, ticket.attempt, ticket.capacity).map_err(|e| match e {
        AttemptFenceError::StaleAttempt { incoming, current } => {
            RuntimeError::StaleAttempt { incoming, current }
        }
        AttemptFenceError::AttemptBeyondMax { attempt, max } => {
            RuntimeError::AttemptBeyondMax { attempt, max }
        }
        AttemptFenceError::InvalidActionCompletion => RuntimeError::InvalidActionCompletion,
    })
}

/// Promotes an engine-issued ticket to the live per-step attempt counter.
pub fn normalize_scheduled_ticket(
    state: &RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionTicket> {
    let current = state.action_attempts.get(ticket.step.as_usize()).copied();
    let attempt = normalize_scheduled_attempt(current, ticket.attempt, ticket.capacity)?;
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
    let slot = state.action_attempts.get_mut(ticket.step.as_usize());
    if let Some(attempt_slot) = slot {
        let next = scheduled_attempt_after(Some(*attempt_slot), ticket.attempt);
        if let Some(n) = next {
            *attempt_slot = n;
        }
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

// ===========================================================================
// Pure kernels — Verus/Flux binding surface
// ===========================================================================

/// Pure ticket classification kernel.
///
/// Classifies an incoming ticket attempt against a per-step counter.
/// Returns `Ok(())` only when the attempt matches the current counter
/// and both are within the ticket's capacity window.
///
/// Panics: NEVER (no unwrap, no panic paths)
/// I/O: NONE
/// Allocation: NONE
pub(crate) fn classify_ticket_attempt(
    current: Option<u16>,
    ticket_attempt: u16,
    ticket_capacity: u16,
) -> Result<(), AttemptFenceError> {
    if ticket_attempt == 0 || ticket_capacity == 0 || ticket_attempt > ticket_capacity {
        Err(AttemptFenceError::AttemptBeyondMax {
            attempt: ticket_attempt,
            max: ticket_capacity,
        })
    } else if current.is_none() {
        Err(AttemptFenceError::InvalidActionCompletion)
    } else {
        let c = current.unwrap();
        if ticket_attempt < c {
            Err(AttemptFenceError::StaleAttempt {
                incoming: ticket_attempt,
                current: c,
            })
        } else if ticket_attempt > c {
            Err(AttemptFenceError::InvalidActionCompletion)
        } else {
            Ok(())
        }
    }
}

/// Pure scheduled-ticket normalization kernel.
///
/// Promotes a scheduled ticket's attempt to at least 1, then checks
/// capacity bounds. Does NOT mutate state.
///
/// Panics: NEVER
/// I/O: NONE
/// Allocation: NONE
pub(crate) fn normalize_scheduled_attempt(
    current: Option<u16>,
    ticket_attempt: u16,
    ticket_capacity: u16,
) -> Result<u16, AttemptFenceError> {
    let Some(c) = current else {
        return Err(AttemptFenceError::InvalidActionCompletion);
    };
    let attempt = c.max(ticket_attempt).max(1);
    if ticket_capacity == 0 || attempt > ticket_capacity {
        Err(AttemptFenceError::AttemptBeyondMax {
            attempt,
            max: ticket_capacity,
        })
    } else {
        Ok(attempt)
    }
}

/// Pure scheduled-attempt recording kernel.
///
/// Computes the new per-step attempt counter value after a scheduling event.
/// Does NOT write to state.
///
/// Panics: NEVER
/// I/O: NONE
/// Allocation: NONE
pub(crate) fn scheduled_attempt_after(current: Option<u16>, ticket_attempt: u16) -> Option<u16> {
    if ticket_attempt == 0 {
        current
    } else if current.is_none() {
        Some(ticket_attempt)
    } else {
        let c = current.unwrap();
        if c == 0 || ticket_attempt > c {
            Some(ticket_attempt)
        } else {
            Some(c)
        }
    }
}
