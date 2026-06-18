#![forbid(unsafe_code)]
//! Run state snapshot returned by journal replay.

use crate::ids::RunId;

use super::state::LifecycleState;

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
