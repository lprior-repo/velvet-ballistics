//! Step state machine proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for step state verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    (StepState::Failed, StepState::Failed),
    (StepState::Cancelled, StepState::Cancelled),
    (StepState::Skipped, StepState::Skipped),
];

pub fn is_valid_transition(from: StepState, to: StepState) -> bool {
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
    let mut result = Vec::new();
    for &(f, t) in VALID_TRANSITIONS {
        if f == from {
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
        if next.len() != 1 || next[0] != terminal {
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
mod tests {
    use super::*;

    #[test]
    fn test_pending_valid_transitions() {
        let next = next_states(StepState::Pending);
        assert!(next.contains(&StepState::Running));
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 5);
    }

    #[test]
    fn test_running_valid_transitions() {
        let next = next_states(StepState::Running);
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Waiting));
        assert!(next.contains(&StepState::Asking));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 6);
    }

    #[test]
    fn test_waiting_to_running() {
        assert!(is_valid_transition(StepState::Waiting, StepState::Running));
    }

    #[test]
    fn test_asking_to_running() {
        assert!(is_valid_transition(StepState::Asking, StepState::Running));
    }

    #[test]
    fn test_terminal_self_transition() {
        assert!(is_valid_transition(StepState::Succeeded, StepState::Succeeded));
        assert!(is_valid_transition(StepState::Failed, StepState::Failed));
        assert!(is_valid_transition(StepState::Cancelled, StepState::Cancelled));
        assert!(is_valid_transition(StepState::Skipped, StepState::Skipped));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!is_valid_transition(StepState::Pending, StepState::Pending));
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
        assert!(!is_valid_transition(StepState::Succeeded, StepState::Running));
        assert!(!is_valid_transition(StepState::Failed, StepState::Running));
    }

    #[test]
    fn test_terminal_immutable() {
        assert!(terminal_cannot_transition_to_non_terminal());
    }

    #[test]
    fn test_terminal_states() {
        let terminals = terminal_states();
        assert_eq!(terminals.len(), 4);
        for t in terminals {
            assert!(t.is_terminal());
        }
    }

    #[test]
    fn test_non_terminal_states() {
        let non_terminals = non_terminal_states();
        assert_eq!(non_terminals.len(), 4);
        for t in non_terminals {
            assert!(!t.is_terminal());
        }
    }
}
