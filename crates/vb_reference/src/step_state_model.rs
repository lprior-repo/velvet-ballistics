//! Reference step state machine model.
//!
//! This is the canonical reference implementation for step state transitions.
//! Use this to verify the optimized implementation matches this behavior.

use vb_core::frame::StepState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateCategory {
    NonTerminal,
    Terminal,
}

impl StepState {
    pub fn category(&self) -> StateCategory {
        match self {
            StepState::Pending => StateCategory::NonTerminal,
            StepState::Running => StateCategory::NonTerminal,
            StepState::Waiting => StateCategory::NonTerminal,
            StepState::Asking => StateCategory::NonTerminal,
            StepState::Succeeded => StateCategory::Terminal,
            StepState::Failed => StateCategory::Terminal,
            StepState::Cancelled => StateCategory::Terminal,
            StepState::Skipped => StateCategory::Terminal,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.category() == StateCategory::Terminal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    InvalidTransition { from: StepState, to: StepState },
}

pub fn valid_transitions() -> Vec<(StepState, StepState)> {
    vec![
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
        // Terminal transitions (idempotent re-mark allowed)
        (StepState::Succeeded, StepState::Succeeded),
        (StepState::Failed, StepState::Failed),
        (StepState::Cancelled, StepState::Cancelled),
        (StepState::Skipped, StepState::Skipped),
    ]
}

pub fn is_valid_transition(from: StepState, to: StepState) -> bool {
    valid_transitions()
        .iter()
        .any(|&(f, t)| f == from && t == to)
}

pub fn validate_transition(from: StepState, to: StepState) -> Result<(), TransitionError> {
    if is_valid_transition(from, to) {
        Ok(())
    } else {
        Err(TransitionError::InvalidTransition { from, to })
    }
}

pub fn next_states(state: StepState) -> Vec<StepState> {
    valid_transitions()
        .iter()
        .filter(|&&(f, _t)| f == state)
        .map(|&(_f, t)| t)
        .collect()
}

pub fn is_terminal_state(state: StepState) -> bool {
    state.is_terminal()
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

pub struct StepStateMachine;

impl StepStateMachine {
    pub fn new() -> Self {
        StepStateMachine
    }

    pub fn can_transition(&self, from: StepState, to: StepState) -> bool {
        is_valid_transition(from, to)
    }

    pub fn transition(&self, from: StepState, to: StepState) -> Result<StepState, TransitionError> {
        validate_transition(from, to)?;
        Ok(to)
    }

    pub fn get_next_states(&self, state: StepState) -> Vec<StepState> {
        next_states(state)
    }

    pub fn is_terminal(&self, state: StepState) -> bool {
        is_terminal_state(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_valid_transitions() {
        let valid = next_states(StepState::Pending);
        assert!(valid.contains(&StepState::Running));
        assert!(valid.contains(&StepState::Succeeded));
        assert!(valid.contains(&StepState::Failed));
        assert!(valid.contains(&StepState::Cancelled));
        assert!(valid.contains(&StepState::Skipped));
        assert_eq!(valid.len(), 5);
    }

    #[test]
    fn test_running_valid_transitions() {
        let valid = next_states(StepState::Running);
        assert!(valid.contains(&StepState::Succeeded));
        assert!(valid.contains(&StepState::Failed));
        assert!(valid.contains(&StepState::Waiting));
        assert!(valid.contains(&StepState::Asking));
        assert!(valid.contains(&StepState::Cancelled));
        assert!(valid.contains(&StepState::Skipped));
        assert_eq!(valid.len(), 6);
    }

    #[test]
    fn test_terminal_states_immutable() {
        for &terminal in &terminal_states() {
            let next = next_states(terminal);
            assert_eq!(next.len(), 1, "Terminal {:?} should only transition to itself", terminal);
            assert_eq!(next[0], terminal, "Terminal {:?} should only transition to itself", terminal);
        }
    }

    #[test]
    fn test_invalid_transitions_rejected() {
        assert!(!is_valid_transition(StepState::Pending, StepState::Pending));
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
        assert!(!is_valid_transition(StepState::Succeeded, StepState::Running));
        assert!(!is_valid_transition(StepState::Failed, StepState::Running));
    }

    #[test]
    fn test_all_states_covered() {
        let all_states = vec![
            StepState::Pending,
            StepState::Running,
            StepState::Waiting,
            StepState::Asking,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Cancelled,
            StepState::Skipped,
        ];
        assert_eq!(all_states.len(), 8);
    }
}
