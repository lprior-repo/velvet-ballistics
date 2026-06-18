//! CLI output rendering and payload assembly.
//!
//! This module owns `system_status_payload` (JSON payload builder) and
//! the three printer functions (text, YAML, JSON) dispatched by
//! `print_system_status`.

#![forbid(unsafe_code)]

use crate::args::{OutputFormat, SystemStatusOptions};
use crate::cli_envelope;

use super::build;
use super::types::SystemStatusReport;

/// Builds the JSON payload for a system-status snapshot.
#[must_use]
pub(crate) fn system_status_payload(
    options: SystemStatusOptions,
    version: &str,
) -> serde_json::Value {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = build::system_status_report(&options);
    let connected = matches!(report.state, super::types::SystemConnectionState::Live);
    serde_json::json!({
        "success": true,
        "profile": options.profile.as_str(),
        "server": options.server.as_str(),
        "connected": connected,
        "state": report.state.as_str(),
        "reason": report.reason,
        "status": {
            "health": build::system_health_label(&report),
            "backend": report.state.as_str(),
            "storage_health": build::storage_health_label(&report),
            "writer_queue_depth": 0,
            "journal_batch_healthy": report.journal_batch_healthy,
            "snapshot_seq": report.snapshot_seq,
            "blob_store_ok": report.blob_store_ok,
            "index_healthy": report.index_healthy,
            "uptime_seconds": 0,
            "active_run_count": report.active_run_count
        },
        "runtime": {
            "shard_state": build::shard_state_label(&report),
            "command_queue_depth": 0,
            "command_queue_capacity": config.command_queue_capacity,
            "active_runs": report.active_run_count,
            "max_active_runs": config.max_active_runs,
            "trace_capacity": config.trace_capacity,
            "trace_dropped": 0,
            "step_budget_per_tick": config.step_budget_per_tick
        },
        "gate": {
            "cli_version": version,
            "schema_version": crate::cli_envelope::SCHEMA_VERSION
        }
    })
}

/// Dispatches to the correct output renderer.
pub(crate) fn print_system_status(
    options: SystemStatusOptions,
    output: OutputFormat,
    version: &str,
) -> Result<(), crate::OutputError> {
    match output {
        OutputFormat::Text => {
            print_text(options, version);
            Ok(())
        }
        OutputFormat::Yaml => print_system_status_yaml(options, version),
        OutputFormat::Postcard => print_json(options, output, version),
    }
}

/// Renders a YAML system-status report to stdout.
fn print_system_status_yaml(
    options: SystemStatusOptions,
    version: &str,
) -> Result<(), crate::OutputError> {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = build::system_status_report(&options);
    crate::write_stdout_line_checked(format_args!(
        "schema_version: {}",
        crate::cli_envelope::SCHEMA_VERSION
    ))?;
    crate::write_stdout_line_checked(format_args!("kind: SystemStatus"))?;
    crate::write_stdout_line_checked(format_args!("profile: {}", options.profile.as_str()))?;
    crate::write_stdout_line_checked(format_args!("server: {}", options.server.as_str()))?;
    crate::write_stdout_line_checked(format_args!(
        "connected: {}",
        matches!(report.state, super::types::SystemConnectionState::Live)
    ))?;
    crate::write_stdout_line_checked(format_args!("state: {}", report.state.as_str()))?;
    crate::write_stdout_line_checked(format_args!("reason: {}", report.reason))?;
    crate::write_stdout_line_checked(format_args!("status:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  health: {}",
        build::system_health_label(&report)
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  backend: {}",
        report.state.as_str()
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  storage_health: {}",
        build::storage_health_label(&report)
    ))?;
    crate::write_stdout_line_checked(format_args!("  writer_queue_depth: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  journal_batch_healthy: {}",
        report.journal_batch_healthy
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  snapshot_seq: {}",
        build::snapshot_seq_label(report.snapshot_seq)
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  blob_store_ok: {}",
        report.blob_store_ok
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  index_healthy: {}",
        report.index_healthy
    ))?;
    crate::write_stdout_line_checked(format_args!("  uptime_seconds: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  active_run_count: {}",
        report.active_run_count
    ))?;
    crate::write_stdout_line_checked(format_args!("runtime:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  shard_state: {}",
        build::shard_state_label(&report)
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  command_queue_depth: 0"
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  command_queue_capacity: {}",
        config.command_queue_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!("gate:"))?;
    crate::write_stdout_line_checked(format_args!("  cli_version: {version}"))
}

/// Renders a JSON system-status report to stdout.
fn print_json(
    options: SystemStatusOptions,
    output: OutputFormat,
    version: &str,
) -> Result<(), crate::OutputError> {
    let payload = system_status_payload(options, version);
    let envelope = crate::cli_envelope::serialize_with_version(
        &payload,
        crate::cli_envelope::Kind::SystemStatus,
    );
    crate::json_out(&envelope, output)
}

/// Renders a plain-text system-status report to stdout.
fn print_text(options: SystemStatusOptions, version: &str) {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = build::system_status_report(&options);
    crate::write_stdout_line(format_args!(
        "system_status: {}",
        build::system_health_label(&report)
    ));
    crate::write_stdout_line(format_args!(
        "connected: {}",
        matches!(report.state, super::types::SystemConnectionState::Live)
    ));
    crate::write_stdout_line(format_args!("state: {}", report.state.as_str()));
    crate::write_stdout_line(format_args!("reason: {}", report.reason));
    crate::write_stdout_line(format_args!("profile: {}", options.profile.as_str()));
    crate::write_stdout_line(format_args!("server: {}", options.server.as_str()));
    crate::write_stdout_line(format_args!(
        "storage_health: {}",
        build::storage_health_label(&report)
    ));
    crate::write_stdout_line(format_args!(
        "journal_batch_healthy: {}",
        report.journal_batch_healthy
    ));
    crate::write_stdout_line(format_args!("blob_store_ok: {}", report.blob_store_ok));
    crate::write_stdout_line(format_args!("index_healthy: {}", report.index_healthy));
    crate::write_stdout_line(format_args!("writer_queue_depth: 0"));
    crate::write_stdout_line(format_args!(
        "active_run_count: {}",
        report.active_run_count
    ));
    crate::write_stdout_line(format_args!(
        "command_queue_capacity: {}",
        config.command_queue_capacity
    ));
    crate::write_stdout_line(format_args!("max_active_runs: {}", config.max_active_runs));
    crate::write_stdout_line(format_args!("cli_version: {version}"));
}
