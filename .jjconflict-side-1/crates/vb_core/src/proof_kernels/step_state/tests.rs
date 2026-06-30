use super::*;

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

// ── StepState::is_terminal exhaustive ─────────────────────────────────

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

// ── validate_transition ───────────────────────────────────────────────

#[test]
fn test_validate_transition_pending_to_running_ok() {
    let result = validate_transition(StepState::Pending, StepState::Running);
    assert_eq!(result.unwrap(), StepState::Running);
}

#[test]
fn test_validate_transition_pending_to_succeeded_ok() {
    let result = validate_transition(StepState::Pending, StepState::Succeeded);
    assert_eq!(result.unwrap(), StepState::Succeeded);
}

#[test]
fn test_validate_transition_pending_to_failed_ok() {
    let result = validate_transition(StepState::Pending, StepState::Failed);
    assert_eq!(result.unwrap(), StepState::Failed);
}

#[test]
fn test_validate_transition_running_to_waiting_ok() {
    let result = validate_transition(StepState::Running, StepState::Waiting);
    assert_eq!(result.unwrap(), StepState::Waiting);
}

#[test]
fn test_validate_transition_running_to_asking_ok() {
    let result = validate_transition(StepState::Running, StepState::Asking);
    assert_eq!(result.unwrap(), StepState::Asking);
}

#[test]
fn test_validate_transition_waiting_to_running_ok() {
    let result = validate_transition(StepState::Waiting, StepState::Running);
    assert_eq!(result.unwrap(), StepState::Running);
}

#[test]
fn test_validate_transition_asking_to_running_ok() {
    let result = validate_transition(StepState::Asking, StepState::Running);
    assert_eq!(result.unwrap(), StepState::Running);
}

#[test]
fn test_validate_transition_terminal_idempotent() {
    assert_eq!(
        validate_transition(StepState::Succeeded, StepState::Succeeded).unwrap(),
        StepState::Succeeded
    );
    assert_eq!(
        validate_transition(StepState::Failed, StepState::Failed).unwrap(),
        StepState::Failed
    );
    assert_eq!(
        validate_transition(StepState::Cancelled, StepState::Cancelled).unwrap(),
        StepState::Cancelled
    );
    assert_eq!(
        validate_transition(StepState::Skipped, StepState::Skipped).unwrap(),
        StepState::Skipped
    );
}

#[test]
fn test_validate_transition_invalid_pending_to_waiting() {
    let result = validate_transition(StepState::Pending, StepState::Waiting);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "invalid_state_transition");
}

#[test]
fn test_validate_transition_invalid_running_to_pending() {
    let result = validate_transition(StepState::Running, StepState::Pending);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "invalid_state_transition");
}

#[test]
fn test_validate_transition_invalid_terminal_to_non_terminal() {
    let result = validate_transition(StepState::Succeeded, StepState::Running);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "invalid_state_transition");
}

#[test]
fn test_validate_transition_invalid_waiting_to_succeeded() {
    let result = validate_transition(StepState::Waiting, StepState::Succeeded);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "invalid_state_transition");
}

#[test]
fn test_validate_transition_invalid_asking_to_failed() {
    let result = validate_transition(StepState::Asking, StepState::Failed);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "invalid_state_transition");
}

// ── all_transitions_exhaustive ───────────────────────────────────────

#[test]
fn test_all_transitions_exhaustive_returns_true() {
    assert!(all_transitions_exhaustive());
}

// ── StepState derived traits ───────────────────────────────────────────

#[test]
fn test_step_state_debug() {
    let state = StepState::Pending;
    let debug = format!("{:?}", state);
    assert_eq!(debug, "Pending");

    let state = StepState::Succeeded;
    assert_eq!(format!("{:?}", state), "Succeeded");

    let state = StepState::Failed;
    assert_eq!(format!("{:?}", state), "Failed");

    let state = StepState::Cancelled;
    assert_eq!(format!("{:?}", state), "Cancelled");

    let state = StepState::Skipped;
    assert_eq!(format!("{:?}", state), "Skipped");

    let state = StepState::Waiting;
    assert_eq!(format!("{:?}", state), "Waiting");

    let state = StepState::Asking;
    assert_eq!(format!("{:?}", state), "Asking");

    let state = StepState::Running;
    assert_eq!(format!("{:?}", state), "Running");
}

#[test]
fn test_step_state_clone() {
    let state = StepState::Running;
    let cloned = state.clone();
    assert_eq!(cloned, state);
}

#[test]
fn test_step_state_copy() {
    let state = StepState::Waiting;
    let _copied: StepState = state;
    assert_eq!(state, StepState::Waiting);
}

#[test]
fn test_step_state_partial_eq_positive() {
    assert_eq!(StepState::Pending, StepState::Pending);
    assert_eq!(StepState::Running, StepState::Running);
    assert_eq!(StepState::Waiting, StepState::Waiting);
    assert_eq!(StepState::Asking, StepState::Asking);
    assert_eq!(StepState::Succeeded, StepState::Succeeded);
    assert_eq!(StepState::Failed, StepState::Failed);
    assert_eq!(StepState::Cancelled, StepState::Cancelled);
    assert_eq!(StepState::Skipped, StepState::Skipped);
}

#[test]
fn test_step_state_partial_eq_negative() {
    assert_ne!(StepState::Pending, StepState::Running);
    assert_ne!(StepState::Running, StepState::Succeeded);
    assert_ne!(StepState::Waiting, StepState::Asking);
    assert_ne!(StepState::Succeeded, StepState::Failed);
    assert_ne!(StepState::Cancelled, StepState::Skipped);
}

#[test]
fn test_step_state_eq() {
    assert!(StepState::Pending == StepState::Pending);
    assert!(StepState::Running != StepState::Pending);
    assert!(StepState::Failed == StepState::Failed);
}

// ── is_valid_transition exhaustive idempotent ───────────────────────────

#[test]
fn test_is_valid_transition_all_idempotent() {
    assert!(is_valid_transition(StepState::Pending, StepState::Pending));
    assert!(is_valid_transition(StepState::Waiting, StepState::Waiting));
    assert!(is_valid_transition(StepState::Asking, StepState::Asking));
    assert!(is_valid_transition(StepState::Running, StepState::Running));
}

#[test]
fn test_is_valid_transition_waiting_asking_self() {
    assert!(is_valid_transition(StepState::Waiting, StepState::Waiting));
    assert!(is_valid_transition(StepState::Asking, StepState::Asking));
}

// ── next_states coverage ────────────────────────────────────────────────

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
        // Succeeded can transition to Pending for loop body re-entry
        if terminal == StepState::Succeeded {
            assert!(next.contains(&StepState::Succeeded));
            assert!(next.contains(&StepState::Pending));
        } else {
            assert_eq!(next.len(), 1);
            assert_eq!(next[0], terminal);
        }
    }
}
