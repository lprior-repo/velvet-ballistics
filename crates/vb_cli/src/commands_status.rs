//! Runtime status command output.
#![forbid(unsafe_code)]

use vb_runtime::shard::{Shard, ShardConfig, ShardHealth, ShardStatus};

use crate::args::{OutputFormat, StatusOptions};
use crate::cli_envelope;

/// Serializable status view for CLI output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Builds a status snapshot from a transient shard. Optional values are diagnostic overlays for
/// no-runtime smoke tests and do not mutate a shard.
#[must_use]
pub(crate) fn build_status(options: StatusOptions) -> CliStatus {
    let shard = Shard::new(ShardConfig::default());
    let status = shard.status();
    from_shard_status(status, options)
}

#[must_use]
fn from_shard_status(status: ShardStatus, options: StatusOptions) -> CliStatus {
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

pub(crate) fn print_status(
    status: &CliStatus,
    output: OutputFormat,
) -> Result<(), crate::OutputError> {
    match output {
        OutputFormat::Text => {
            print_text(status);
            Ok(())
        }
        OutputFormat::Yaml => print_status_yaml(status),
        OutputFormat::Postcard => print_json(status, output),
    }
}

fn print_status_yaml(status: &CliStatus) -> Result<(), crate::OutputError> {
    crate::write_stdout_line_checked(format_args!(
        "schema_version: velvet-ballistics/cli-output/v1"
    ))?;
    crate::write_stdout_line_checked(format_args!("kind: status"))?;
    crate::write_stdout_line_checked(format_args!("status: {}", status.health))?;
    crate::write_stdout_line_checked(format_args!("running: {}", status.running))?;
    crate::write_stdout_line_checked(format_args!("shutting_down: {}", status.shutting_down))?;
    crate::write_stdout_line_checked(format_args!("command_queue:"))?;
    crate::write_stdout_line_checked(format_args!("  depth: {}", status.command_queue_depth))?;
    crate::write_stdout_line_checked(format_args!(
        "  capacity: {}",
        status.command_queue_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!("active_runs:"))?;
    crate::write_stdout_line_checked(format_args!("  active: {}", status.active_runs))?;
    crate::write_stdout_line_checked(format_args!(
        "  max_active_runs: {}",
        status.max_active_runs
    ))?;
    crate::write_stdout_line_checked(format_args!("trace_ring:"))?;
    crate::write_stdout_line_checked(format_args!("  capacity: {}", status.trace_capacity))?;
    crate::write_stdout_line_checked(format_args!("  dropped: {}", status.trace_dropped))?;
    crate::write_stdout_line_checked(format_args!(
        "step_budget_per_tick: {}",
        status.step_budget_per_tick
    ))?;
    crate::write_stdout_line_checked(format_args!("runtime_policy: {}", status.runtime_policy))
}

fn print_text(status: &CliStatus) {
    crate::write_stdout_line(format_args!("status: {}", status.health));
    crate::write_stdout_line(format_args!("running: {}", status.running));
    crate::write_stdout_line(format_args!("shutting_down: {}", status.shutting_down));
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
}

fn print_json(status: &CliStatus, output: OutputFormat) -> Result<(), crate::OutputError> {
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
        "runtime_policy": status.runtime_policy
    });
    let envelope = crate::cli_envelope::serialize_with_version(&payload, crate::cli_envelope::Kind::CliStatus);
    crate::json_out(&envelope, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_status_reports_default_no_runtime_shard() {
        let status = build_status(StatusOptions::default());
        assert_eq!(status.health, "running");
        assert!(status.running);
        assert!(!status.shutting_down);
        assert_eq!(status.command_queue_depth, 0);
        assert_eq!(status.command_queue_capacity, 1024);
        assert_eq!(status.active_runs, 0);
        assert_eq!(status.max_active_runs, 1024);
        assert_eq!(status.trace_capacity, 4096);
        assert_eq!(status.trace_dropped, 0);
        assert_eq!(status.step_budget_per_tick, 1000);
        assert_eq!(status.runtime_policy, "Strict");
    }

    #[test]
    fn build_status_applies_diagnostic_overlays_without_mutation() {
        let status = build_status(StatusOptions {
            active_runs: Some(5),
            queue_depth: Some(3),
            trace_dropped: Some(0),
            emit_yaml: false,
        });
        assert_eq!(status.active_runs, 5);
        assert_eq!(status.command_queue_depth, 3);
        assert_eq!(status.trace_dropped, 0);
    }

    #[test]
    fn build_status_reports_overlay_values_without_silent_clamping() {
        let status = build_status(StatusOptions {
            active_runs: Some(2048),
            queue_depth: Some(2048),
            trace_dropped: Some(7),
            emit_yaml: false,
        });
        assert_eq!(status.active_runs, 2048);
        assert_eq!(status.command_queue_depth, 2048);
        assert_eq!(status.trace_dropped, 7);
    }
}
