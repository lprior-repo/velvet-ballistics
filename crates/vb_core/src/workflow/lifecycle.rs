#![forbid(unsafe_code)]
//! Lifecycle state machine for workflow runs.

use crate::ids::RunId;

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
