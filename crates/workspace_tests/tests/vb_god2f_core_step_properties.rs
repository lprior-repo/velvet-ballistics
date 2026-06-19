#![forbid(unsafe_code)]

//! HVR-PO-CORE-005: generated behavior pressure for production StepState transitions.

use proptest::prelude::*;
use proptest::strategy::Strategy;
use vb_core::{StepState, frame::is_valid_step_state_transition};

fn step_state_strategy() -> impl Strategy<Value = StepState> {
    prop_oneof![
        Just(StepState::Pending),
        Just(StepState::Running),
        Just(StepState::Succeeded),
        Just(StepState::Failed),
        Just(StepState::Skipped),
        Just(StepState::Waiting),
        Just(StepState::Asking),
        Just(StepState::Cancelled),
    ]
}

fn expected_transition(current: StepState, new: StepState) -> bool {
    if current == new {
        return true;
    }
    matches!(
        (current, new),
        (StepState::Pending, StepState::Running)
            | (StepState::Pending, StepState::Succeeded)
            | (StepState::Pending, StepState::Failed)
            | (StepState::Pending, StepState::Cancelled)
            | (StepState::Pending, StepState::Skipped)
            | (StepState::Running, StepState::Succeeded)
            | (StepState::Running, StepState::Failed)
            | (StepState::Running, StepState::Waiting)
            | (StepState::Running, StepState::Asking)
            | (StepState::Running, StepState::Cancelled)
            | (StepState::Running, StepState::Skipped)
            | (StepState::Waiting, StepState::Running)
            | (StepState::Asking, StepState::Running)
    )
}

proptest! {
    #[test]
    fn vb_god2f_core_step_properties(current in step_state_strategy(), new in step_state_strategy()) {
        prop_assert_eq!(is_valid_step_state_transition(current, new), expected_transition(current, new));
        if matches!(current, StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled) {
            prop_assert_eq!(is_valid_step_state_transition(current, new), current == new);
        }
    }
}

#[test]
fn vb_god2f_core_step_matrix_matches_contract_text() {
    let states = [
        StepState::Pending,
        StepState::Running,
        StepState::Succeeded,
        StepState::Failed,
        StepState::Skipped,
        StepState::Waiting,
        StepState::Asking,
        StepState::Cancelled,
    ];
    for current in states {
        for new in states {
            assert_eq!(
                is_valid_step_state_transition(current, new),
                expected_transition(current, new)
            );
        }
    }
}
