#![forbid(unsafe_code)]
//! Lifecycle transition checker.
//!
//! Pure function that validates whether a command is allowed from a given state.

use super::state::{LifecycleCommand, LifecycleState};

/// Checks if a lifecycle state transition is valid for the given command.
///
/// This is the single authoritative decision function for the FSM.
/// All callers should use this to gate transitions before mutating state.
///
/// # Valid transitions
///
/// - `Active` → `Cancel` → `Cancelled`
/// - `WaitingAnswer` → `Cancel` → `Cancelled`
/// - `WaitingAnswer` → `Resume` → `Active`
/// - `Failed` → `Retry` → `Active`
/// - `WaitingAnswer` → `Answer` → `Completed`
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
