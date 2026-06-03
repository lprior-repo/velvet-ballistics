#![forbid(unsafe_code)]
//! Snapshot and state inspection helpers.

use vb_core::ids::RunId;

use crate::shard::types::{InspectSnapshot, RunState};

/// Creates a snapshot from run state.
pub fn snapshot_from_state(
    run: RunId,
    correlation: u64,
    state: &RunState,
) -> InspectSnapshot {
    InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}
