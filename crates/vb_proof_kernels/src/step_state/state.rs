//! Step state enum definition.
//!
//! This is a local-only mirror of the production `vb_core::frame::step_state::StepState`
//! and is **not** bound to production code. Verus checks on this type are local model
//! evidence only.

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus enum ──────────────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {
    #[derive(Clone, Copy)]
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
        // Exec-mode equality used in exec fn bodies.
        pub fn eq(&self, other: &StepState) -> (result: bool) {
            match (self, other) {
                (StepState::Pending, StepState::Pending)
                | (StepState::Running, StepState::Running)
                | (StepState::Waiting, StepState::Waiting)
                | (StepState::Asking, StepState::Asking)
                | (StepState::Succeeded, StepState::Succeeded)
                | (StepState::Failed, StepState::Failed)
                | (StepState::Cancelled, StepState::Cancelled)
                | (StepState::Skipped, StepState::Skipped) => true,
                _ => false,
            }
        }
    }
} // verus!

// ── Cargo kernel enum ───────────────────────────────────────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    /// Step states for the proof kernel state machine.
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
        #[must_use]
        pub fn is_terminal(&self) -> bool {
            matches!(
                self,
                StepState::Succeeded
                    | StepState::Failed
                    | StepState::Cancelled
                    | StepState::Skipped
            )
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::StepState;
