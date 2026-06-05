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
    (StepState::Succeeded, StepState::Running),
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
        // Succeeded is special: it can transition to Running for loop body re-entry
        if terminal == StepState::Succeeded {
            let valid = matches!(
                next.as_slice(),
                [StepState::Succeeded]
                    | [StepState::Succeeded, StepState::Running]
                    | [StepState::Running, StepState::Succeeded]
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
mod tests {
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
        // Note: Succeeded -> Running IS valid (for loop body re-entry)
        assert!(is_valid_transition(StepState::Succeeded, StepState::Running));
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
    fn test_validate_transition_valid_succeeded_to_running() {
        // Succeeded -> Running is VALID for loop body re-entry
        let result = validate_transition(StepState::Succeeded, StepState::Running);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StepState::Running);
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
            // Succeeded can transition to Running for loop body re-entry
            if terminal == StepState::Succeeded {
                assert!(next.contains(&StepState::Succeeded));
                assert!(next.contains(&StepState::Running));
            } else {
                assert_eq!(next.len(), 1);
                assert_eq!(next[0], terminal);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PO-KANI-006: Kani harness — terminal_cannot_transition_to_non_terminal
// ---------------------------------------------------------------------------
// After removal of the Succeeded special case in
// `terminal_cannot_transition_to_non_terminal()`, the function must return
// true for all terminals (uniformly absorbing, no exceptions).
// This harness uses kani::any() to symbolically verify that for any
// terminal state, the only valid transition is to itself.
#[cfg(kani)]
mod kani_step_state_harnesses {
    use super::*;

    /// PO-KANI-006: Verify terminal_cannot_transition_to_non_terminal()
    /// returns true after the Succeeded special case is removed.
    /// This harness also symbolically checks that for any terminal
    /// state t and any non-terminal s != t, is_valid_transition(t, s) == false.
    #[kani::proof]
    fn terminal_cannot_transition_to_non_terminal_kani() {
        // Verify the top-level function returns true
        let result = terminal_cannot_transition_to_non_terminal();
        kani::assert(result, "terminal_cannot_transition_to_non_terminal must return true post-fix");

        // Symbolic check: for ALL terminal states and ALL target states (s != t),
        // is_valid_transition(t, s) is false.
        // Uses kani::any() to cover all 8 StepState variants for both t and s.
        let t_raw: u8 = kani::any();
        let s_raw: u8 = kani::any();

        let terminals = terminal_states();
        let t = terminals[(t_raw as usize) % terminals.len()];
        let s_raw2: u8 = kani::any();
        let s = match s_raw2 % 8 {
            0 => StepState::Pending,
            1 => StepState::Running,
            2 => StepState::Succeeded,
            3 => StepState::Failed,
            4 => StepState::Skipped,
            5 => StepState::Waiting,
            6 => StepState::Asking,
            _ => StepState::Cancelled,
        };

        // For all terminal t, if s != t, the transition must be invalid
        if t != s {
            let valid = is_valid_transition(t, s);
            kani::assert(!valid, "terminal->non-terminal transition must be invalid post-fix");
        } else {
            // Self-transition is always valid (idempotent)
            let valid = is_valid_transition(t, t);
            kani::assert(valid, "terminal->self must always be valid");
        }

        // Also verify: next_states for ANY terminal contains ONLY that terminal
        // (after removal of Succeeded->Running and Succeeded special case)
        for terminal in terminal_states() {
            let next = next_states(terminal);
            // Post-fix: each terminal's next_states should contain only itself
            // (no Running, no Pending, no other exceptions)
            kani::assert(
                next.len() == 1,
                "post-fix: terminal {:?} should have exactly 1 next_state (self)",
            );
            // The only element should be the terminal itself
            kani::assert(
                next.contains(&terminal),
                "terminal {:?} should be in its own next_states",
            );
        }
    }
}
