//! Step state machine proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for step state verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StepState {
    Pending,
    Running,
    Waiting,
    Asking,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl StepState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StepState::Succeeded | StepState::Failed | StepState::Cancelled | StepState::Skipped
        )
    }
}

const VALID_TRANSITIONS: &[(StepState, StepState)] = &[
    // Pending transitions
    (StepState::Pending, StepState::Running),
    (StepState::Pending, StepState::Succeeded),
    (StepState::Pending, StepState::Failed),
    (StepState::Pending, StepState::Cancelled),
    (StepState::Pending, StepState::Skipped),
    // Running transitions
    (StepState::Running, StepState::Succeeded),
    (StepState::Running, StepState::Failed),
    (StepState::Running, StepState::Waiting),
    (StepState::Running, StepState::Asking),
    (StepState::Running, StepState::Cancelled),
    (StepState::Running, StepState::Skipped),
    // Waiting transitions
    (StepState::Waiting, StepState::Running),
    // Asking transitions
    (StepState::Asking, StepState::Running),
    // Terminal transitions (idempotent re-mark)
    (StepState::Succeeded, StepState::Succeeded),
    (StepState::Succeeded, StepState::Pending),
    (StepState::Failed, StepState::Failed),
    (StepState::Cancelled, StepState::Cancelled),
    (StepState::Skipped, StepState::Skipped),
];

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

pub fn validate_transition(from: StepState, to: StepState) -> Result<StepState, &'static str> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err("invalid_state_transition")
    }
}

pub fn next_states(from: StepState) -> Vec<StepState> {
    let mut result = vec![from];
    for &(f, t) in VALID_TRANSITIONS {
        if f == from && !result.contains(&t) {
            result.push(t);
        }
    }
    result
}

pub fn terminal_states() -> Vec<StepState> {
    vec![
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ]
}

pub fn non_terminal_states() -> Vec<StepState> {
    vec![
        StepState::Pending,
        StepState::Running,
        StepState::Waiting,
        StepState::Asking,
    ]
}

pub fn terminal_cannot_transition_to_non_terminal() -> bool {
    for terminal in terminal_states() {
        let next = next_states(terminal);
        // Succeeded is special: it can transition to Pending for loop body re-entry
        if terminal == StepState::Succeeded {
            let valid = matches!(
                next.as_slice(),
                [StepState::Succeeded]
                    | [StepState::Succeeded, StepState::Pending]
                    | [StepState::Pending, StepState::Succeeded]
            );
            if !valid {
                return false;
            }
        } else if !matches!(next.as_slice(), [only] if *only == terminal) {
            return false;
        }
    }
    true
}

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

#[cfg(test)]
mod tests;
