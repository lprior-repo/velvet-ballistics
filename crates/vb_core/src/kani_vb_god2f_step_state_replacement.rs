#![cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
#![forbid(unsafe_code)]

//! HVR-PO-CORE-003: production step-state transition replacement harness.

use crate::frame::{StepState, is_valid_step_state_transition};

fn state_from_symbol(symbol: u8) -> StepState {
    match symbol % 8 {
        0 => StepState::Pending,
        1 => StepState::Running,
        2 => StepState::Succeeded,
        3 => StepState::Failed,
        4 => StepState::Skipped,
        5 => StepState::Waiting,
        6 => StepState::Asking,
        _ => StepState::Cancelled,
    }
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

fn terminal_state(state: StepState) -> bool {
    matches!(
        state,
        StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled
    )
}

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_core_step_state_transition_replacement() {
    let current = state_from_symbol(kani::any());
    let new = state_from_symbol(kani::any());
    let observed = is_valid_step_state_transition(current, new);

    kani::cover!(current == new, "self-transition branch covered");
    kani::cover!(
        current == StepState::Pending && new == StepState::Running,
        "Pending to Running branch covered"
    );
    kani::cover!(
        current == StepState::Running && new == StepState::Succeeded,
        "Running to terminal branch covered"
    );
    kani::cover!(
        terminal_state(current) && new == StepState::Running,
        "terminal to Running rejection branch covered"
    );

    kani::assert(
        observed == expected_transition(current, new),
        "production step-state predicate matches independent transition table",
    );
    if terminal_state(current) && current != new {
        kani::assert(!observed, "terminal states are self-only");
    }
}
