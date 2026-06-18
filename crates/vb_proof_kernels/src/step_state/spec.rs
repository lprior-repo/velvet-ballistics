//! Verus spec functions for the step state machine.
//!
//! Local-only model checks — not production deductive evidence.

#[cfg(verus_keep_ghost)]
verus! {
    use crate::step_state::state::StepState;

    // ── Spec: step state equality ──────────────────────────────────────
    pub open spec fn spec_step_state_eq(a: StepState, b: StepState) -> bool {
        matches!((a, b), (StepState::Pending, StepState::Pending)
            | (StepState::Running, StepState::Running)
            | (StepState::Waiting, StepState::Waiting)
            | (StepState::Asking, StepState::Asking)
            | (StepState::Succeeded, StepState::Succeeded)
            | (StepState::Failed, StepState::Failed)
            | (StepState::Cancelled, StepState::Cancelled)
            | (StepState::Skipped, StepState::Skipped))
    }

    // ── Spec: transition relation (canonical mathematical definition) ──
    pub open spec fn spec_valid_transition(from: StepState, to: StepState) -> bool {
        from == to
            || (from == StepState::Pending
                && (to == StepState::Running
                    || to == StepState::Succeeded
                    || to == StepState::Failed
                    || to == StepState::Cancelled
                    || to == StepState::Skipped))
            || (from == StepState::Running
                && (to == StepState::Succeeded
                    || to == StepState::Failed
                    || to == StepState::Waiting
                    || to == StepState::Asking
                    || to == StepState::Cancelled
                    || to == StepState::Skipped))
            || (from == StepState::Waiting && to == StepState::Running)
            || (from == StepState::Asking && to == StepState::Running)
            || (from == StepState::Succeeded && to == StepState::Succeeded)
            || (from == StepState::Failed && to == StepState::Failed)
            || (from == StepState::Cancelled && to == StepState::Cancelled)
            || (from == StepState::Skipped && to == StepState::Skipped)
    }

    // ── Spec: is_terminal ──────────────────────────────────────────────
    pub open spec fn spec_is_terminal(s: StepState) -> bool {
        matches!(
            s,
            StepState::Succeeded | StepState::Failed | StepState::Cancelled
                | StepState::Skipped
        )
    }
}

#[cfg(verus_keep_ghost)]
pub use vstd::prelude::*;
