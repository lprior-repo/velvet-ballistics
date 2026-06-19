//! Transition logic for the step state machine.
//!
//! Contains the canonical transition relation, validation helpers, and
//! state-set queries. No Verus spec lives here — only pure Rust.

#[cfg(not(verus_keep_ghost))]
use super::state::StepState;

#[cfg(not(verus_keep_ghost))]
pub const VALID_TRANSITIONS: &[(StepState, StepState)] = &[
    (StepState::Pending, StepState::Running),
    (StepState::Pending, StepState::Succeeded),
    (StepState::Pending, StepState::Failed),
    (StepState::Pending, StepState::Cancelled),
    (StepState::Pending, StepState::Skipped),
    (StepState::Running, StepState::Succeeded),
    (StepState::Running, StepState::Failed),
    (StepState::Running, StepState::Waiting),
    (StepState::Running, StepState::Asking),
    (StepState::Running, StepState::Cancelled),
    (StepState::Running, StepState::Skipped),
    (StepState::Waiting, StepState::Running),
    (StepState::Asking, StepState::Running),
    (StepState::Succeeded, StepState::Succeeded),
    (StepState::Failed, StepState::Failed),
    (StepState::Cancelled, StepState::Cancelled),
    (StepState::Skipped, StepState::Skipped),
];

#[cfg(not(verus_keep_ghost))]
/// Pure transition predicate: is `from → to` allowed?
#[must_use]
pub fn is_valid_transition(from: StepState, to: StepState) -> bool {
    if from == to {
        return true;
    }
    for &(f, t) in VALID_TRANSITIONS {
        if f == from && t == to {
            return true;
        }
    }
    false
}

#[cfg(not(verus_keep_ghost))]
/// Result-based validator for callers that need an error path.
#[must_use = "transition validation returns the accepted state or an error"]
pub fn validate_transition(from: StepState, to: StepState) -> Result<StepState, &'static str> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err("invalid_state_transition")
    }
}

#[cfg(not(verus_keep_ghost))]
/// All reachable states (including self) from `from`.
#[must_use]
pub fn next_states(from: StepState) -> Vec<StepState> {
    let mut result = vec![from];
    for &(f, t) in VALID_TRANSITIONS {
        if f == from && !result.contains(&t) {
            result.push(t);
        }
    }
    result
}

#[cfg(not(verus_keep_ghost))]
/// The four terminal states.
#[must_use]
pub fn terminal_states() -> Vec<StepState> {
    vec![
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ]
}

#[cfg(not(verus_keep_ghost))]
/// The four non-terminal (live) states.
#[must_use]
pub fn non_terminal_states() -> Vec<StepState> {
    vec![
        StepState::Pending,
        StepState::Running,
        StepState::Waiting,
        StepState::Asking,
    ]
}

#[cfg(not(verus_keep_ghost))]
/// Invariant: no terminal state may transition to a non-terminal one.
#[must_use]
pub fn terminal_cannot_transition_to_non_terminal() -> bool {
    for terminal in terminal_states() {
        let next = next_states(terminal);
        if next.len() != 1 || next.first() != Some(&terminal) {
            return false;
        }
    }
    true
}

#[cfg(not(verus_keep_ghost))]
/// Coverage check: every state in `terminal_states()` is terminal and vice versa.
#[must_use]
pub fn all_transitions_exhaustive() -> bool {
    for terminal in terminal_states() {
        if !terminal.is_terminal() {
            return false;
        }
    }
    for non_terminal in non_terminal_states() {
        if non_terminal.is_terminal() {
            return false;
        }
    }
    true
}
