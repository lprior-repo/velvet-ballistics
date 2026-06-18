//! Types for the runtime status command.
//!
//! `DbProbeStatus` models the outcome of a `--db` journal probe.
//! `CliStatus` is the serialisable snapshot consumed by every output mode.

#![forbid(unsafe_code)]

use vb_runtime::shard::{Shard, ShardConfig, ShardHealth, ShardStatus};

use crate::args::{OutputFormat, StatusOptions};
use crate::cli_envelope;

// ---------------------------------------------------------------------------
// DbProbeStatus — the probe outcome that drives the `db_probe_status` field.
// ---------------------------------------------------------------------------

/// Probe outcome that drives the `db_probe_status` field on `CliStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DbProbeStatus {
    /// No `--db` was supplied; the snapshot is derived from a transient shard.
    NotRequested,
    /// `--db` was supplied and the journal opened; live state is reported.
    Live,
    /// `--db` was supplied but the journal could not be opened; the snapshot
    /// is derived from a transient shard with a diagnostic reason attached.
    Fallback,
}

/// Returns the canonical string label for a `DbProbeStatus`.
#[must_use]
pub(crate) const fn db_probe_status_name(status: DbProbeStatus) -> &'static str {
    match status {
        DbProbeStatus::NotRequested => "not_requested",
        DbProbeStatus::Live => "live",
        DbProbeStatus::Fallback => "fallback",
    }
}

// ---------------------------------------------------------------------------
// CliStatus — the serialisable view for CLI output.
// ---------------------------------------------------------------------------

/// Serializable status view for CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliStatus {
    pub(crate) health: &'static str,
    pub(crate) running: bool,
    pub(crate) shutting_down: bool,
    pub(crate) command_queue_depth: usize,
    pub(crate) command_queue_capacity: usize,
    pub(crate) active_runs: usize,
    pub(crate) max_active_runs: usize,
    pub(crate) trace_capacity: usize,
    pub(crate) trace_dropped: u64,
    pub(crate) step_budget_per_tick: u64,
    pub(crate) runtime_policy: &'static str,
    /// Live-storage probe outcome. Only meaningful when `--db` is supplied.
    pub(crate) db_probe_status: DbProbeStatus,
    /// Diagnostic reason when `db_probe_status` is `Fallback`. Empty otherwise.
    pub(crate) db_probe_reason: String,
}
