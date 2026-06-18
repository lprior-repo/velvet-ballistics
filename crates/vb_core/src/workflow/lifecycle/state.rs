#![forbid(unsafe_code)]
//! Lifecycle FSM types: states and commands.
//!
//! This module defines the state machine types that describe a run's
//! lifecycle and the commands that drive state transitions.
//!
//! ## State Machine
//!
//! ```text
//!   Pending
//!      │
//!      ▼
//!   Active ──► Cancel ──► Cancelled (terminal)
//!      │
//!      ├──► WaitingAnswer ──► Resume ──┐
//!      │           │                  │
//!      │           ├──► Cancel ──┐    │
//!      │           └──► Answer ──┘    │
//!      │                              ▼
//!   Failed ──► Retry ──► (back to Active)
//!
//!   Active ──► Completed (terminal)
//! ```
//!
//! ## Valid Transitions
//!
//! | From State     | Command | To State    |
//! |----------------|---------|-------------|
//! | Active         | Cancel  | Cancelled   |
//! | WaitingAnswer  | Cancel  | Cancelled   |
//! | WaitingAnswer  | Resume  | Active      |
//! | Failed         | Retry   | Active      |
//! | WaitingAnswer  | Answer  | Completed   |

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
