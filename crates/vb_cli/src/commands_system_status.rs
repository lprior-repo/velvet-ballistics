//! System-status command output.
#![forbid(unsafe_code)]

use crate::args::{OutputFormat, SystemStatusOptions};

const NO_BACKEND_REASON: &str = "no live runtime or storage status backend is attached";

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

fn print_system_status_yaml(
    options: SystemStatusOptions,
    version: &str,
) -> Result<(), crate::OutputError> {
    let config = vb_runtime::shard::ShardConfig::default();
    crate::write_stdout_line_checked(format_args!(
        "schema_version: {}",
        crate::cli_envelope::SCHEMA_VERSION
    ))?;
    crate::write_stdout_line_checked(format_args!("kind: SystemStatus"))?;
    crate::write_stdout_line_checked(format_args!("profile: {}", options.profile.as_str()))?;
    crate::write_stdout_line_checked(format_args!("server: {}", options.server.as_str()))?;
    crate::write_stdout_line_checked(format_args!("connected: false"))?;
    crate::write_stdout_line_checked(format_args!("reason: no-backend"))?;
    crate::write_stdout_line_checked(format_args!("status:"))?;
    crate::write_stdout_line_checked(format_args!("  health: degraded"))?;
    crate::write_stdout_line_checked(format_args!("  backend: no-backend"))?;
    crate::write_stdout_line_checked(format_args!("  storage_health: Degraded"))?;
    crate::write_stdout_line_checked(format_args!("  writer_queue_depth: 0"))?;
    crate::write_stdout_line_checked(format_args!("  journal_batch_healthy: false"))?;
    crate::write_stdout_line_checked(format_args!("  snapshot_seq: null"))?;
    crate::write_stdout_line_checked(format_args!("  blob_store_ok: false"))?;
    crate::write_stdout_line_checked(format_args!("  index_healthy: false"))?;
    crate::write_stdout_line_checked(format_args!("  uptime_seconds: 0"))?;
    crate::write_stdout_line_checked(format_args!("  active_run_count: 0"))?;
    crate::write_stdout_line_checked(format_args!("runtime:"))?;
    crate::write_stdout_line_checked(format_args!("  shard_state: not_connected"))?;
    crate::write_stdout_line_checked(format_args!("  command_queue_depth: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  command_queue_capacity: {}",
        config.command_queue_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!("gate:"))?;
    crate::write_stdout_line_checked(format_args!("  cli_version: {version}"))
}

#[must_use]
pub(crate) fn system_status_payload(
    options: SystemStatusOptions,
    version: &str,
) -> serde_json::Value {
    let config = vb_runtime::shard::ShardConfig::default();
    serde_json::json!({
        "success": true,
        "profile": options.profile.as_str(),
        "server": options.server.as_str(),
        "connected": false,
        "reason": NO_BACKEND_REASON,
        "status": {
            "storage_health": "Degraded",
            "writer_queue_depth": 0,
            "journal_batch_healthy": false,
            "snapshot_seq": null,
            "blob_store_ok": false,
            "index_healthy": false,
            "uptime_seconds": 0,
            "active_run_count": 0
        },
        "runtime": {
            "shard_state": "not_connected",
            "command_queue_depth": 0,
            "command_queue_capacity": config.command_queue_capacity,
            "active_runs": 0,
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

fn print_text(options: SystemStatusOptions, version: &str) {
    let config = vb_runtime::shard::ShardConfig::default();
    crate::write_stdout_line(format_args!("system_status: degraded"));
    crate::write_stdout_line(format_args!("connected: false"));
    crate::write_stdout_line(format_args!("reason: {NO_BACKEND_REASON}"));
    crate::write_stdout_line(format_args!("profile: {}", options.profile.as_str()));
    crate::write_stdout_line(format_args!("server: {}", options.server.as_str()));
    crate::write_stdout_line(format_args!("storage_health: Degraded"));
    crate::write_stdout_line(format_args!("journal_batch_healthy: false"));
    crate::write_stdout_line(format_args!("blob_store_ok: false"));
    crate::write_stdout_line(format_args!("index_healthy: false"));
    crate::write_stdout_line(format_args!("writer_queue_depth: 0"));
    crate::write_stdout_line(format_args!("active_run_count: 0"));
    crate::write_stdout_line(format_args!(
        "command_queue_capacity: {}",
        config.command_queue_capacity
    ));
    crate::write_stdout_line(format_args!("max_active_runs: {}", config.max_active_runs));
    crate::write_stdout_line(format_args!("cli_version: {version}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{DurabilityMode, VerifyProfile};

    #[test]
    fn system_status_payload_reports_degraded_when_no_backend_is_attached() {
        let payload = system_status_payload(SystemStatusOptions::default(), "0.1.0");
        let status = &payload["status"];

        assert_eq!(payload["connected"], serde_json::json!(false));
        assert_eq!(status["storage_health"], serde_json::json!("Degraded"));
        assert_eq!(status["journal_batch_healthy"], serde_json::json!(false));
        assert_eq!(status["blob_store_ok"], serde_json::json!(false));
        assert_eq!(status["index_healthy"], serde_json::json!(false));
    }

    #[test]
    fn system_status_payload_preserves_selected_profile_and_server() {
        let payload = system_status_payload(
            SystemStatusOptions {
                profile: VerifyProfile::Full,
                server: DurabilityMode::Journaled,
                emit_yaml: false,
            },
            "0.1.0",
        );

        assert_eq!(payload["profile"], serde_json::json!("full"));
        assert_eq!(payload["server"], serde_json::json!("journaled"));
    }
}
