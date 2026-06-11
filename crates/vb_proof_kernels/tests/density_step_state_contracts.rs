#![forbid(unsafe_code)]

use vb_proof_kernels::step_state::{
    StepState, all_transitions_exhaustive, is_valid_transition, next_states, non_terminal_states,
    terminal_cannot_transition_to_non_terminal, terminal_states, validate_transition,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

ktest!(
    #[test]
    step_pending_is_not_terminal,
    {
        assert!(!StepState::Pending.is_terminal());
    }
);

ktest!(
    #[test]
    step_running_is_not_terminal,
    {
        assert!(!StepState::Running.is_terminal());
    }
);

ktest!(
    #[test]
    step_waiting_is_not_terminal,
    {
        assert!(!StepState::Waiting.is_terminal());
    }
);

ktest!(
    #[test]
    step_asking_is_not_terminal,
    {
        assert!(!StepState::Asking.is_terminal());
    }
);

ktest!(
    #[test]
    step_succeeded_is_terminal,
    {
        assert!(StepState::Succeeded.is_terminal());
    }
);

ktest!(
    #[test]
    step_failed_is_terminal,
    {
        assert!(StepState::Failed.is_terminal());
    }
);

ktest!(
    #[test]
    step_cancelled_is_terminal,
    {
        assert!(StepState::Cancelled.is_terminal());
    }
);

ktest!(
    #[test]
    step_skipped_is_terminal,
    {
        assert!(StepState::Skipped.is_terminal());
    }
);

ktest!(
    #[test]
    step_pending_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Pending, StepState::Running));
    }
);

ktest!(
    #[test]
    step_pending_to_succeeded_valid,
    {
        assert!(is_valid_transition(
            StepState::Pending,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_running_to_waiting_valid,
    {
        assert!(is_valid_transition(StepState::Running, StepState::Waiting));
    }
);

ktest!(
    #[test]
    step_running_to_asking_valid,
    {
        assert!(is_valid_transition(StepState::Running, StepState::Asking));
    }
);

ktest!(
    #[test]
    step_waiting_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Waiting, StepState::Running));
    }
);

ktest!(
    #[test]
    step_asking_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Asking, StepState::Running));
    }
);

ktest!(
    #[test]
    step_succeeded_self_valid,
    {
        assert!(is_valid_transition(
            StepState::Succeeded,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_failed_self_valid,
    {
        assert!(is_valid_transition(StepState::Failed, StepState::Failed));
    }
);

ktest!(
    #[test]
    step_cancelled_self_valid,
    {
        assert!(is_valid_transition(
            StepState::Cancelled,
            StepState::Cancelled
        ));
    }
);

ktest!(
    #[test]
    step_skipped_self_valid,
    {
        assert!(is_valid_transition(StepState::Skipped, StepState::Skipped));
    }
);

ktest!(
    #[test]
    step_running_to_pending_invalid,
    {
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
    }
);

ktest!(
    #[test]
    step_succeeded_to_running_invalid,
    {
        assert!(!is_valid_transition(
            StepState::Succeeded,
            StepState::Running
        ));
    }
);

ktest!(
    #[test]
    step_failed_to_pending_invalid,
    {
        assert!(!is_valid_transition(StepState::Failed, StepState::Pending));
    }
);

ktest!(
    #[test]
    step_waiting_to_succeeded_invalid,
    {
        assert!(!is_valid_transition(
            StepState::Waiting,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_validate_transition_returns_target_on_valid_edge,
    {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Running),
            Ok(StepState::Running)
        );
    }
);

ktest!(
    #[test]
    step_validate_transition_returns_error_on_invalid_edge,
    {
        assert_eq!(
            validate_transition(StepState::Failed, StepState::Running),
            Err("invalid_state_transition")
        );
    }
);

ktest!(
    #[test]
    step_next_states_pending_has_six_entries,
    {
        assert_eq!(next_states(StepState::Pending).len(), 6);
    }
);

ktest!(
    #[test]
    step_next_states_running_has_seven_entries,
    {
        assert_eq!(next_states(StepState::Running).len(), 7);
    }
);

ktest!(
    #[test]
    step_next_states_waiting_returns_self_and_running,
    {
        assert_eq!(
            next_states(StepState::Waiting),
            vec![StepState::Waiting, StepState::Running]
        );
    }
);

ktest!(
    #[test]
    step_terminal_states_has_four_entries,
    {
        assert_eq!(terminal_states().len(), 4);
    }
);

ktest!(
    #[test]
    step_non_terminal_states_has_four_entries,
    {
        assert_eq!(non_terminal_states().len(), 4);
    }
);

ktest!(
    #[test]
    step_terminal_cannot_transition_to_non_terminal_contract,
    {
        assert!(terminal_cannot_transition_to_non_terminal());
    }
);

ktest!(
    #[test]
    step_all_transitions_exhaustive_contract,
    {
        assert!(all_transitions_exhaustive());
    }
);
