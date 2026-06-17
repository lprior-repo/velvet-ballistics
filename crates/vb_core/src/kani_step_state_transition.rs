#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani parity harness binding the runtime step-state predicate to the
//! Verus/TLA transition contract over all state pairs.

use crate::frame::{StepState, is_valid_step_state_transition};

impl kani::Arbitrary for StepState {
    fn any() -> Self {
        match kani::any::<u8>() {
            0 => Self::Pending,
            1 => Self::Running,
            2 => Self::Succeeded,
            3 => Self::Failed,
            4 => Self::Skipped,
            5 => Self::Waiting,
            6 => Self::Asking,
            _ => Self::Cancelled,
        }
    }
}

#[kani::proof]
fn kani_step_state_transition_matches_contract() {
    let current: StepState = kani::any();
    let next: StepState = kani::any();

    kani::cover(
        current == StepState::Pending && next == StepState::Pending,
        "pending idempotent transition covered",
    );
    kani::cover(
        current == StepState::Succeeded && next == StepState::Running,
        "terminal->running invalid edge covered (must be blocked)",
    );
    kani::cover(
        current == StepState::Waiting && next == StepState::Running,
        "suspended resume transition covered",
    );

    kani::assert(is_valid_step_state_transition(current, next) == transition_contract(current, next),
        "runtime StepState transition predicate matches formal contract",
    );
}

#[allow(clippy::match_same_arms)] // Arms preserve the reviewed transition matrix.
fn transition_contract(current: StepState, next: StepState) -> bool {
    match (current, next) {
        (state, target) if state == target => true,
        (StepState::Pending, StepState::Running) => true,
        (
            StepState::Pending,
            StepState::Succeeded | StepState::Failed | StepState::Cancelled | StepState::Skipped,
        ) => true,
        (
            StepState::Running,
            StepState::Succeeded
            | StepState::Failed
            | StepState::Waiting
            | StepState::Asking
            | StepState::Cancelled
            | StepState::Skipped,
        ) => true,
        (StepState::Waiting | StepState::Asking, StepState::Running) => true,
        // All terminal states are absorbing; no terminal->Running edge is
        // admitted. Loop body re-entry uses the explicit Succeeded->Pending
        // admission path in RunFrame::mark_pending before mark_running.
        _ => false,
    }
}
