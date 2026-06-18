//! Output formatting for the runtime status command.
//!
//! Provides text, YAML, and JSON rendering of a `CliStatus` snapshot.

#![forbid(unsafe_code)]

use crate::args::OutputFormat;
use crate::cli_envelope;
use crate::OutputError;

use super::types::{CliStatus, DbProbeStatus, db_probe_status_name};

// ---------------------------------------------------------------------------
// Public API — single entry point used by the dispatcher.
// ---------------------------------------------------------------------------

pub(crate) fn print_status(
    status: &CliStatus,
    output: OutputFormat,
) -> Result<(), OutputError> {
    match output {
        OutputFormat::Text => {
            print_text(status);
            Ok(())
        }
        OutputFormat::Yaml => print_status_yaml(status),
        OutputFormat::Postcard => print_json(status, output),
    }
}

// ---------------------------------------------------------------------------
// YAML rendering
// ---------------------------------------------------------------------------

fn print_status_yaml(status: &CliStatus) -> Result<(), OutputError> {
    crate::write_stdout_line_checked(format_args!(
        "schema_version: velvet-ballistics/cli-output/v1"
    ))?;
    crate::write_stdout_line_checked(format_args!("kind: status"))?;
    crate::write_stdout_line_checked(format_args!("status: {}", status.health))?;
    crate::write_stdout_line_checked(format_args!("running: {}", status.running))?;
    crate::write_stdout_line_checked(format_args!(
        "shutting_down: {}",
        status.shutting_down
    ))?;
    crate::write_stdout_line_checked(format_args!("command_queue:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  depth: {}",
        status.command_queue_depth
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  capacity: {}",
        status.command_queue_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!("active_runs:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  active: {}",
        status.active_runs
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  max_active_runs: {}",
        status.max_active_runs
    ))?;
    crate::write_stdout_line_checked(format_args!("trace_ring:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  capacity: {}",
        status.trace_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  dropped: {}",
        status.trace_dropped
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "step_budget_per_tick: {}",
        status.step_budget_per_tick
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "runtime_policy: {}",
        status.runtime_policy
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "db_probe: {}",
        db_probe_status_name(status.db_probe_status)
    ))?;
    if !status.db_probe_reason.is_empty() {
        crate::write_stdout_line_checked(format_args!(
            "db_probe_reason: {}",
            status.db_probe_reason
        ))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

fn print_text(status: &CliStatus) {
    crate::write_stdout_line(format_args!("status: {}", status.health));
    crate::write_stdout_line(format_args!("running: {}", status.running));
    crate::write_stdout_line(format_args!(
        "shutting_down: {}",
        status.shutting_down
    ));
    crate::write_stdout_line(format_args!(
        "command_queue: depth={} capacity={}",
        status.command_queue_depth, status.command_queue_capacity
    ));
    crate::write_stdout_line(format_args!(
        "active_runs: active={} max_active_runs={}",
        status.active_runs, status.max_active_runs
    ));
    crate::write_stdout_line(format_args!(
        "trace_ring: capacity={} dropped={}",
        status.trace_capacity, status.trace_dropped
    ));
    crate::write_stdout_line(format_args!(
        "step_budget_per_tick: {}",
        status.step_budget_per_tick
    ));
    crate::write_stdout_line(format_args!("RuntimePolicy: {}", status.runtime_policy));
    crate::write_stdout_line(format_args!(
        "db_probe: {}",
        db_probe_status_name(status.db_probe_status)
    ));
    if !status.db_probe_reason.is_empty() {
        crate::write_stdout_line(format_args!(
            "db_probe_reason: {}",
            status.db_probe_reason
        ));
    }
}

// ---------------------------------------------------------------------------
// JSON rendering
// ---------------------------------------------------------------------------

fn print_json(status: &CliStatus, output: OutputFormat) -> Result<(), OutputError> {
    let payload = serde_json::json!({
        "status": status.health,
        "running": status.running,
        "shutting_down": status.shutting_down,
        "command_queue": {
            "depth": status.command_queue_depth,
            "capacity": status.command_queue_capacity
        },
        "active_runs": {
            "active": status.active_runs,
            "max_active_runs": status.max_active_runs
        },
        "trace_ring": {
            "capacity": status.trace_capacity,
            "dropped": status.trace_dropped
        },
        "step_budget_per_tick": status.step_budget_per_tick,
        "runtime_policy": status.runtime_policy,
        "db_probe": db_probe_status_name(status.db_probe_status),
        "db_probe_reason": status.db_probe_reason,
    });
    let envelope =
        cli_envelope::serialize_with_version(&payload, cli_envelope::Kind::CliStatus);
    crate::json_out(&envelope, output)
}
