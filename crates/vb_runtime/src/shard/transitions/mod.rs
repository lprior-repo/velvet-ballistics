#![forbid(unsafe_code)]
//! Run state transition helpers, organized by responsibility.
//!
//! - `fsm` — `Shard::apply` FSM routing
//! - `continuation` — `keep_run`, `keep_run_with_snapshot`, `await_action`, `await_timer`
//! - `terminal` — `finish_run`, `fail_run_state`

mod continuation;
mod fsm;
mod terminal;

/// Outcome of a snapshot write attempt.
#[cfg_attr(kani, allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotWriteOutcome {
    /// Snapshot was written successfully.
    Written,
    /// Snapshot interval is disabled (`interval == 0`).
    SkippedDisabled,
    /// No storage journal available (volatile/noop journal): cannot write snapshots.
    SkippedNoStorage,
    /// Snapshot would not fire (step count has not reached the interval).
    SkippedNotReady,
    /// Snapshot write failed; the error was logged and the caller continues.
    Failed,
}
