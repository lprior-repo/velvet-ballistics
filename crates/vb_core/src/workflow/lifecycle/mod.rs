#![forbid(unsafe_code)]
//! Lifecycle state machine for workflow runs.
//!
//! This module is a thin coordination layer that exposes the FSM types,
//! transition checker, and run state snapshot.
//!
//! ## Modules
//!
//! - [`state`]: Lifecycle FSM types (states and commands).
//! - [`transition`]: Transition validation logic.
//! - [`run_state`]: Run state snapshot struct.

pub mod run_state;
pub mod state;
pub mod transition;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// ───────────────────────────────────────────────────────────────────────────
// Re-exports for ergonomic public API
// ───────────────────────────────────────────────────────────────────────────

pub use run_state::RunState;
pub use state::{LifecycleCommand, LifecycleState};
pub use transition::check_lifecycle_transition;

// ───────────────────────────────────────────────────────────────────────────
// Verus annotations for lifecycle state machine (compiled under verus toolchain only)
// ───────────────────────────────────────────────────────────────────────────

#[cfg(verus)]
verus! {
    use vstd::prelude::*;

    use super::state::{LifecycleCommand, LifecycleState};
    use super::transition::check_lifecycle_transition;

    /// Spec: which commands are valid from a given state.
    pub closed spec fn spec_check_lifecycle_transition(state: LifecycleState, cmd: LifecycleCommand) -> bool {
        matches!(
            (state, cmd),
            (LifecycleState::Active, LifecycleCommand::Cancel)
                | (LifecycleState::WaitingAnswer, LifecycleCommand::Cancel)
                | (LifecycleState::WaitingAnswer, LifecycleCommand::Resume)
                | (LifecycleState::Failed, LifecycleCommand::Retry)
                | (LifecycleState::WaitingAnswer, LifecycleCommand::Answer)
        )
    }

    /// Spec: terminal states are Cancelled and Completed.
    pub closed spec fn spec_is_terminal(lifecycle: LifecycleState) -> bool {
        matches!(lifecycle, LifecycleState::Cancelled | LifecycleState::Completed)
    }

    /// Proof: production check_lifecycle_transition equals the spec.
    pub proof fn lemma_check_lifecycle_transition_matches_spec(state: LifecycleState, cmd: LifecycleCommand)
        ensures
            spec_check_lifecycle_transition(state, cmd) == check_lifecycle_transition(state, cmd),
    {
        reveal_with_fuel(check_lifecycle_transition, 1);
        reveal(spec_check_lifecycle_transition);
        assert(spec_check_lifecycle_transition(state, cmd) == check_lifecycle_transition(state, cmd));
    }

    /// Proof: terminal states cannot accept any lifecycle command (absorbing).
    /// Cancelled and Completed have no outgoing transitions in the lifecycle FSM.
    pub proof fn lemma_terminal_states_absorbing()
        ensures
            forall|l: LifecycleState, c: LifecycleCommand|
                spec_is_terminal(l) ==> !spec_check_lifecycle_transition(l, c),
    {
        assert forall|l: LifecycleState, c: LifecycleCommand|
            spec_is_terminal(l) ==> !spec_check_lifecycle_transition(l, c) by {
            if spec_is_terminal(l) {
                reveal(spec_is_terminal);
                reveal(spec_check_lifecycle_transition);
                // Cancelled and Completed are never in the source positions of valid transitions.
                assert(!spec_check_lifecycle_transition(l, c));
            }
        };
    }

    /// Proof: Failed is NOT terminal (retry can transition from it).
    pub proof fn lemma_failed_is_not_terminal()
        ensures
            !spec_is_terminal(LifecycleState::Failed),
    {
        reveal(spec_is_terminal);
        assert(!spec_is_terminal(LifecycleState::Failed));
    }

    /// Proof: Active is the entry state for new runs (not terminal, accepts Cancel).
    pub proof fn lemma_active_is_entry_and_accepts_cancel()
        ensures
            !spec_is_terminal(LifecycleState::Active)
                && spec_check_lifecycle_transition(LifecycleState::Active, LifecycleCommand::Cancel),
    {
        reveal(spec_is_terminal);
        reveal(spec_check_lifecycle_transition);
        assert(!spec_is_terminal(LifecycleState::Active));
        assert(spec_check_lifecycle_transition(LifecycleState::Active, LifecycleCommand::Cancel));
    }
}
