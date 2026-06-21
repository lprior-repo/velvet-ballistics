//! Snapshot building for the runtime status command.
//!
//! When `options.db` is `Some(path)`, the journal is opened and live storage
//! state (active run count, pending queue depth) is read from it. When the
//! open fails, the snapshot falls back to a transient shard view with a
//! non-empty `db_probe_reason` so operators can see the difference between
//! "no backend" and "backend could not be probed".
//!
//! When `options.db` is `None`, the snapshot is derived from a fresh
//! in-memory `Shard` (no storage attachment).

#![forbid(unsafe_code)]

use vb_runtime::shard::{Shard, ShardConfig, ShardHealth, ShardStatus};
use vb_storage::records::{KnownRunHeaderStatus, RunHeaderStatusClass};

use crate::args::StatusOptions;

use super::types::{CliStatus, DbProbeStatus};

/// Builds a status snapshot.
#[must_use]
pub(crate) fn build_status(options: StatusOptions) -> CliStatus {
    match options.db.as_deref() {
        Some(path) => build_status_with_journal(path),
        None => build_status_transient(options),
    }
}

fn build_status_transient(options: StatusOptions) -> CliStatus {
    let config = ShardConfig::default();
    let status = match Shard::new(config) {
        Ok(shard) => shard.status(),
        Err(_) => synthetic_shard_status(config),
    };
    from_shard_status(status, options, DbProbeStatus::NotRequested, String::new())
}

fn build_status_with_journal(path: &std::path::Path) -> CliStatus {
    match vb_storage::FjallJournal::open(path, None) {
        Ok(journal) => {
            let (active_runs, pending_runs) = match journal.run_headers() {
                Ok(headers) => {
                    let mut active = 0_usize;
                    let mut pending = 0_usize;
                    for header in &headers {
                        match header.run_header_status().classify() {
                            RunHeaderStatusClass::Known(KnownRunHeaderStatus::Pending)
                            | RunHeaderStatusClass::Known(KnownRunHeaderStatus::Accepted) => {
                                pending = pending.saturating_add(1);
                            }
                            RunHeaderStatusClass::Known(KnownRunHeaderStatus::Active) => {
                                active = active.saturating_add(1);
                            }
                            // Finished and any future-known status are
                            // counted as "not active, not pending".
                            RunHeaderStatusClass::Known(_) => {}
                            RunHeaderStatusClass::Unknown(_) => {}
                        }
                    }
                    (active, pending)
                }
                Err(_) => (0, 0),
            };
            let config = ShardConfig::default();
            let status = match Shard::new(config) {
                Ok(shard) => shard.status(),
                Err(_) => synthetic_shard_status(config),
            };
            // Build a status that overlays the live counters on top of the
            // transient-shard defaults; the transient shard supplies the
            // capacity / runtime policy / trace numbers that are not journal
            // state.
            let options = StatusOptions {
                active_runs: Some(active_runs),
                queue_depth: Some(pending_runs),
                db: None,
                ..StatusOptions::default()
            };
            from_shard_status(status, options, DbProbeStatus::Live, String::new())
        }
        Err(error) => {
            let config = ShardConfig::default();
            let status = match Shard::new(config) {
                Ok(shard) => shard.status(),
                Err(_) => synthetic_shard_status(config),
            };
            let reason = format!("journal open at {} failed: {error}", path.display());
            from_shard_status(
                status,
                StatusOptions::default(),
                DbProbeStatus::Fallback,
                reason,
            )
        }
    }
}

/// Builds a synthetic `ShardStatus` that mirrors what a freshly-created
/// shard with the same configuration would report, used as a fallback when
/// `Shard::new` itself fails so the status command can still surface
/// capacity / policy / trace numbers.
#[must_use]
fn synthetic_shard_status(config: ShardConfig) -> ShardStatus {
    ShardStatus {
        health: ShardHealth::Running,
        running: true,
        shutting_down: false,
        command_queue_depth: 0,
        command_queue_capacity: config.command_queue_capacity,
        active_runs: 0,
        max_active_runs: config.max_active_runs,
        snapshot_interval_steps: config.snapshot_interval_steps,
        trace_capacity: config.trace_capacity,
        trace_dropped: 0,
        step_budget_per_tick: config.step_budget_per_tick,
        runtime_policy: config.policy,
    }
}

#[must_use]
fn from_shard_status(
    status: ShardStatus,
    options: StatusOptions,
    db_probe_status: DbProbeStatus,
    db_probe_reason: String,
) -> CliStatus {
    CliStatus {
        health: health_name(status.health),
        running: status.running,
        shutting_down: status.shutting_down,
        command_queue_depth: match options.queue_depth {
            Some(depth) => depth,
            None => status.command_queue_depth,
        },
        command_queue_capacity: status.command_queue_capacity,
        active_runs: match options.active_runs {
            Some(active_runs) => active_runs,
            None => status.active_runs,
        },
        max_active_runs: status.max_active_runs,
        trace_capacity: status.trace_capacity,
        trace_dropped: match options.trace_dropped {
            Some(trace_dropped) => trace_dropped,
            None => status.trace_dropped,
        },
        step_budget_per_tick: status.step_budget_per_tick,
        runtime_policy: policy_name(status.runtime_policy),
        db_probe_status,
        db_probe_reason,
    }
}

#[must_use]
fn health_name(health: ShardHealth) -> &'static str {
    match health {
        ShardHealth::Running => "running",
        ShardHealth::ShuttingDown => "shutting_down",
        _ => "unknown",
    }
}

#[must_use]
fn policy_name(policy: vb_core::policy::RuntimePolicy) -> &'static str {
    match policy {
        vb_core::policy::RuntimePolicy::Strict => "Strict",
        vb_core::policy::RuntimePolicy::Journaled => "Journaled",
        vb_core::policy::RuntimePolicy::Relaxed => "Relaxed",
        _ => "unknown",
    }
}
