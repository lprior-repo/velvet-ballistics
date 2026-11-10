#![allow(unreachable_pub)]
//! Crash/recovery lab (spec sections 36, 76.12, 77).
//!
//! `cargo xtask crash-lab --workflow <name> [--crash-at <point> | --all-crash-points]`
//! runs one deterministic fault-injection point: a minimal workflow is driven
//! through production shard paths to the crash point, the shard is dropped
//! without a graceful close, and recovery from the durable journal is driven
//! to completion. The workflow and crash point are parsed at the boundary into
//! `CrashWorkflow` and `CrashPoint`; the package and test filter each point
//! runs come from a fixed table, never from the raw strings.

#![allow(clippy::print_stderr)]

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use xtask::XtaskCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashWorkflow {
    Minimal,
}

impl CrashWorkflow {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    ActionDispatch,
}

impl CrashPoint {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ActionDispatch => "action_dispatch",
        }
    }

    #[must_use]
    pub const fn target(self) -> (&'static str, &'static str) {
        match self {
            Self::ActionDispatch => ("vb_runtime", "action_dispatch_crash_points"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashSelection {
    One(CrashPoint),
    All,
}

#[must_use]
pub fn available_workflows() -> &'static [&'static str] {
    &["minimal"]
}

#[must_use]
pub fn available_crash_points() -> &'static [&'static str] {
    &["action_dispatch"]
}

#[must_use]
pub fn all_crash_points() -> &'static [CrashPoint] {
    &[CrashPoint::ActionDispatch]
}

pub fn parse_workflow(name: &str) -> Result<CrashWorkflow, XtaskCommandError> {
    match name {
        "minimal" => Ok(CrashWorkflow::Minimal),
        _ => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "workflow".to_string(),
            reason: format!(
                "unknown crash-lab workflow: {name}; available workflows: {}",
                available_workflows().join(", ")
            ),
        }),
    }
}

pub fn parse_crash_point(name: &str) -> Result<CrashPoint, XtaskCommandError> {
    match name {
        "action_dispatch" => Ok(CrashPoint::ActionDispatch),
        _ => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: format!(
                "unknown crash-lab crash point: {name}; available crash points: {}",
                available_crash_points().join(", ")
            ),
        }),
    }
}

pub fn parse_selection(
    crash_at: Option<&str>,
    all_crash_points: bool,
) -> Result<CrashSelection, XtaskCommandError> {
    match (crash_at, all_crash_points) {
        (Some(point), false) => parse_crash_point(point).map(CrashSelection::One),
        (None, true) => Ok(CrashSelection::All),
        (Some(_), true) => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "pass exactly one of --crash-at <point> or --all-crash-points".to_string(),
        }),
        (None, false) => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "pass --crash-at <point> or --all-crash-points".to_string(),
        }),
    }
}

fn run_crash_point(
    workflow: CrashWorkflow,
    point: CrashPoint,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    let (package, filter) = point.target();
    eprintln!(
        "Running crash-lab point {} for workflow {} (cargo nextest run -p {} {})",
        point.name(),
        workflow.name(),
        package,
        filter
    );
    let status = Command::new("cargo")
        .arg("nextest")
        .arg("run")
        .arg("-p")
        .arg(package)
        .arg(filter)
        .current_dir(workspace_root)
        .status()
        .context("Failed to run crash-lab point (is cargo-nextest installed?)")?;
    if status.success() {
        eprintln!(
            "PASS: crash-lab point '{}' for workflow '{}' completed successfully",
            point.name(),
            workflow.name()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "crash-lab point '{}' for workflow '{}' failed with exit code {:?}",
            point.name(),
            workflow.name(),
            status.code()
        )
    }
}

pub fn cmd_crash_lab(
    workflow_name: &str,
    crash_at: Option<&str>,
    all_crash_points_flag: bool,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    let workflow = parse_workflow(workflow_name).map_err(anyhow::Error::msg)?;
    let selection = parse_selection(crash_at, all_crash_points_flag).map_err(anyhow::Error::msg)?;
    match selection {
        CrashSelection::One(point) => run_crash_point(workflow, point, workspace_root),
        CrashSelection::All => all_crash_points()
            .iter()
            .try_for_each(|point| run_crash_point(workflow, *point, workspace_root)),
    }
}

#[cfg(test)]
#[path = "crash_lab_tests.rs"]
mod tests;
