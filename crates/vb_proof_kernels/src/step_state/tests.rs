//! Unit tests for the step state machine.
//!
//! Exercises transitions, invariants, and the validate_transition Result API.

#[cfg(all(not(verus_keep_ghost), test))]
mod tests {
    use super::super::state::StepState;
    use super::super::transition::{
        all_transitions_exhaustive, is_valid_transition, next_states, non_terminal_states,
        terminal_cannot_transition_to_non_terminal, terminal_states, validate_transition,
    };

    #[test]
    fn test_pending_valid_transitions() {
        let next = next_states(StepState::Pending);
        assert!(next.contains(&StepState::Pending));
        assert!(next.contains(&StepState::Running));
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 6);
    }

    #[test]
    fn test_running_valid_transitions() {
        let next = next_states(StepState::Running);
        assert!(next.contains(&StepState::Running));
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Waiting));
        assert!(next.contains(&StepState::Asking));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 7);
    }

    #[test]
    fn test_all_idempotent_transitions() {
        for state in [
            StepState::Pending,
            StepState::Running,
            StepState::Waiting,
            StepState::Asking,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Cancelled,
            StepState::Skipped,
        ] {
            assert!(is_valid_transition(state, state));
        }
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
        assert!(is_valid_transition(
            StepState::Succeeded,
            StepState::Succeeded
        ));
        assert!(is_valid_transition(StepState::Failed, StepState::Failed));
        assert!(is_valid_transition(
            StepState::Cancelled,
            StepState::Cancelled
        ));
        assert!(is_valid_transition(StepState::Skipped, StepState::Skipped));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
        assert!(!is_valid_transition(
            StepState::Succeeded,
            StepState::Running
        ));
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

    #[test]
    fn test_is_terminal_pending() {
        assert!(!StepState::Pending.is_terminal());
    }

    #[test]
    fn test_is_terminal_running() {
        assert!(!StepState::Running.is_terminal());
    }

    #[test]
    fn test_is_terminal_waiting() {
        assert!(!StepState::Waiting.is_terminal());
    }

    #[test]
    fn test_is_terminal_asking() {
        assert!(!StepState::Asking.is_terminal());
    }

    #[test]
    fn test_is_terminal_succeeded() {
        assert!(StepState::Succeeded.is_terminal());
    }

    #[test]
    fn test_is_terminal_failed() {
        assert!(StepState::Failed.is_terminal());
    }

    #[test]
    fn test_is_terminal_cancelled() {
        assert!(StepState::Cancelled.is_terminal());
    }

    #[test]
    fn test_is_terminal_skipped() {
        assert!(StepState::Skipped.is_terminal());
    }

    #[test]
    fn test_validate_transition_pending_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_pending_to_succeeded_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Succeeded),
            Ok(StepState::Succeeded)
        );
    }

    #[test]
    fn test_validate_transition_pending_to_failed_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Failed),
            Ok(StepState::Failed)
        );
    }

    #[test]
    fn test_validate_transition_running_to_waiting_ok() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Waiting),
            Ok(StepState::Waiting)
        );
    }

    #[test]
    fn test_validate_transition_running_to_asking_ok() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Asking),
            Ok(StepState::Asking)
        );
    }

    #[test]
    fn test_validate_transition_waiting_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Waiting, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_asking_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Asking, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_terminal_idempotent() {
        assert_eq!(
            validate_transition(StepState::Succeeded, StepState::Succeeded),
            Ok(StepState::Succeeded)
        );
        assert_eq!(
            validate_transition(StepState::Failed, StepState::Failed),
            Ok(StepState::Failed)
        );
        assert_eq!(
            validate_transition(StepState::Cancelled, StepState::Cancelled),
            Ok(StepState::Cancelled)
        );
        assert_eq!(
            validate_transition(StepState::Skipped, StepState::Skipped),
            Ok(StepState::Skipped)
        );
    }

    #[test]
    fn test_validate_transition_invalid_pending_to_waiting() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Waiting),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_running_to_pending() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Pending),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_waiting_to_succeeded() {
        assert_eq!(
            validate_transition(StepState::Waiting, StepState::Succeeded),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_asking_to_failed() {
        assert_eq!(
            validate_transition(StepState::Asking, StepState::Failed),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_all_transitions_exhaustive_returns_true() {
        assert!(all_transitions_exhaustive());
    }

    #[test]
    fn test_step_state_debug() {
        assert_eq!(format!("{:?}", StepState::Pending), "Pending");
        assert_eq!(format!("{:?}", StepState::Succeeded), "Succeeded");
        assert_eq!(format!("{:?}", StepState::Running), "Running");
    }

    #[test]
    fn test_step_state_clone_and_copy() {
        let state = StepState::Running;
        let _cloned = state;
        let _copied: StepState = state;
        assert_eq!(state, StepState::Running);
    }

    #[test]
    fn test_next_states_waiting() {
        let next = next_states(StepState::Waiting);
        assert!(next.contains(&StepState::Waiting));
        assert!(next.contains(&StepState::Running));
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_next_states_asking() {
        let next = next_states(StepState::Asking);
        assert!(next.contains(&StepState::Asking));
        assert!(next.contains(&StepState::Running));
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_next_states_terminal_unique() {
        for terminal in terminal_states() {
            let next = next_states(terminal);
            assert_eq!(next.len(), 1);
            assert_eq!(next.first().copied(), Some(terminal));
        }
    }
}
