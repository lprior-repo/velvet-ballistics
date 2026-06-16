#![forbid(unsafe_code)]
//! Lifecycle state machine for workflow runs.

use crate::ids::RunId;

// ───────────────────────────────────────────────────────────────────────────
// Verus annotations for lifecycle state machine (compiled under verus toolchain only)
// ───────────────────────────────────────────────────────────────────────────
#[cfg(verus)]
verus! {
    use vstd::prelude::*;

    use super::{LifecycleState, LifecycleCommand, check_lifecycle_transition};

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

// ============================================================================
// Lifecycle state machine
// ============================================================================

/// Lifecycle state of a run derived from journal event replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleState {
    /// Run accepted but not yet active.
    Pending,
    /// Run is actively executing.
    Active,
    /// Run is waiting for an external answer.
    WaitingAnswer,
    /// Run was cancelled.
    Cancelled,
    /// Run completed successfully.
    Completed,
    /// Run failed.
    Failed,
}

impl LifecycleState {
    /// Returns true if this is a terminal state.
    /// Note: Failed is NOT terminal because retry can transition from Failed.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Completed)
    }
}

/// Lifecycle command issued by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleCommand {
    /// Cancel the run.
    Cancel,
    /// Resume a waiting run.
    Resume,
    /// Retry a failed run.
    Retry,
    /// Answer a waiting run's question.
    Answer,
}

/// Checks if a lifecycle state transition is valid for the given command.
#[must_use]
pub const fn check_lifecycle_transition(state: LifecycleState, cmd: LifecycleCommand) -> bool {
    match (state, cmd) {
        // Cancel is valid from Active or WaitingAnswer
        (LifecycleState::Active, LifecycleCommand::Cancel) => true,
        (LifecycleState::WaitingAnswer, LifecycleCommand::Cancel) => true,
        // Resume is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Resume) => true,
        // Retry is valid from Failed
        (LifecycleState::Failed, LifecycleCommand::Retry) => true,
        // Answer is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Answer) => true,
        // All other transitions are invalid
        _ => false,
    }
}

/// Run state snapshot returned by replay.
#[derive(Debug, Clone)]
pub struct RunState {
    /// Current lifecycle state.
    pub lifecycle: LifecycleState,
    /// Run identifier.
    pub run_id: RunId,
}

impl RunState {
    /// Returns true if this run is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.lifecycle.is_terminal()
    }
}
