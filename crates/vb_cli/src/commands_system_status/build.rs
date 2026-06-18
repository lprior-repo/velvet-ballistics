//! Report construction and health-label generation.
//!
//! This module owns the journal-probing logic and the pure-label
//! functions that derive display strings from `SystemStatusReport`.

#![forbid(unsafe_code)]

use crate::args::SystemStatusOptions;

use super::types::{SystemConnectionState, SystemStatusReport};

/// Assembles a `SystemStatusReport` from the CLI options.
pub(crate) fn system_status_report(options: &SystemStatusOptions) -> SystemStatusReport {
    match options.db.as_deref() {
        Some(path) => SystemStatusReport::from_live_journal(path),
        None => SystemStatusReport::not_requested(),
    }
}

/// Derives the overall system health label from the report.
pub(crate) fn system_health_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live if report.journal_batch_healthy => "healthy",
        SystemConnectionState::Live => "degraded",
        SystemConnectionState::Fallback => "degraded",
        SystemConnectionState::NotRequested => "degraded",
    }
}

/// Derives the storage-subsystem health label from the report.
pub(crate) fn storage_health_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live if report.journal_batch_healthy => "Healthy",
        SystemConnectionState::Live => "Degraded",
        SystemConnectionState::Fallback => "Degraded",
        SystemConnectionState::NotRequested => "Degraded",
    }
}

/// Derives the shard state label from the report.
pub(crate) fn shard_state_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live => "connected",
        SystemConnectionState::Fallback => "not_connected",
        SystemConnectionState::NotRequested => "not_connected",
    }
}

/// Derives the snapshot-sequence display token.
pub(crate) fn snapshot_seq_label(seq: Option<u64>) -> String {
    match seq {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    }
}
