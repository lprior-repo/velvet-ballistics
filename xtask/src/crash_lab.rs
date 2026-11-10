#![allow(unreachable_pub)]
//! Crash/recovery lab (spec section 76.12, VB-REPLAY-001).
//!
//! `cargo xtask crash-lab --workflow <name> [--crash-at <point> | --all-crash-points]`
//! runs the deterministic fault-injection harness for one workflow: the tests
//! that pin the section 76.12 assertions for each crash point (recovery
//! succeeds or blocks with typed reconciliation state, no duplicated
//! non-idempotent action, no lost durable completion, snapshot + tail
//! matches full replay). The workflow name and crash point are parsed at the
//! boundary into `CrashWorkflow` / `CrashPoint`; the package and test filter
//! each point runs come from a fixed table, never from the raw strings.

#![allow(clippy::print_stderr)]

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use xtask::XtaskCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashWorkflow {
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    ActionDispatch,
}

impl CrashWorkflow {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
        }
    }
}

impl CrashPoint {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ActionDispatch => "action_dispatch",
        }
    }

    #[must_use]
    pub const fn runs_for(
        self,
        workflow: CrashWorkflow,
    ) -> &'static [(&'static str, &'static str)] {
        match (workflow, self) {
            (CrashWorkflow::Minimal, Self::ActionDispatch) => &[
                ("vb_runtime", "journal_before_dispatch"),
                ("vb_runtime", "crash_recovery"),
                ("vb_storage", "snapshot_tail_matches_full_journal"),
            ],
        }
    }
}

#[must_use]
pub fn available_workflows() -> &'static [&'static str] {
    &["minimal"]
}

#[must_use]
pub fn available_points() -> &'static [&'static str] {
    &["action_dispatch"]
}

#[must_use]
pub fn all_points_for(workflow: CrashWorkflow) -> &'static [CrashPoint] {
    match workflow {
        CrashWorkflow::Minimal => &[CrashPoint::ActionDispatch],
    }
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

pub fn parse_point(name: &str) -> Result<CrashPoint, XtaskCommandError> {
    match name {
        "action_dispatch" => Ok(CrashPoint::ActionDispatch),
        _ => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: format!(
                "unknown crash-lab crash point: {name}; available crash points: {}",
                available_points().join(", ")
            ),
        }),
    }
}

pub fn resolve_points(
    workflow_name: &str,
    crash_at: Option<&str>,
    all_crash_points: bool,
) -> Result<(CrashWorkflow, Vec<CrashPoint>), XtaskCommandError> {
    let workflow = parse_workflow(workflow_name)?;
    match (crash_at, all_crash_points) {
        (Some(_), true) => Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "--crash-at and --all-crash-points are mutually exclusive".to_string(),
        }),
        (Some(point), false) => parse_point(point).map(|point| (workflow, vec![point])),
        (None, true) => Ok((workflow, all_points_for(workflow).to_vec())),
        (None, false) => Err(XtaskCommandError::MissingRequiredInput {
            command: "crash-lab".to_string(),
            input: "--crash-at or --all-crash-points".to_string(),
        }),
    }
}

fn run_nextest(package: &str, filter: &str, workspace_root: &Path) -> anyhow::Result<()> {
    eprintln!("Running nextest -p {package} {filter}");
    let status = Command::new("cargo")
        .arg("nextest")
        .arg("run")
        .arg("-p")
        .arg(package)
        .arg(filter)
        .current_dir(workspace_root)
        .status()
        .context("Failed to run crash-lab tests (is cargo-nextest installed?)")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "crash-lab tests failed: cargo nextest run -p {package} {filter} exited with {:?}",
            status.code()
        )
    }
}

fn run_point(
    workflow: CrashWorkflow,
    point: CrashPoint,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    eprintln!(
        "Running crash-lab workflow {} at crash point {}",
        workflow.name(),
        point.name()
    );
    point
        .runs_for(workflow)
        .iter()
        .try_for_each(|(package, filter)| run_nextest(package, filter, workspace_root))?;
    eprintln!(
        "PASS: crash-lab workflow {} at crash point {} completed successfully",
        workflow.name(),
        point.name()
    );
    Ok(())
}

pub fn cmd_crash_lab(
    workflow_name: &str,
    crash_at: Option<&str>,
    all_crash_points: bool,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    let (workflow, points) =
        resolve_points(workflow_name, crash_at, all_crash_points).map_err(anyhow::Error::msg)?;
    points
        .iter()
        .try_for_each(|point| run_point(workflow, *point, workspace_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use xtask::{XtaskCommand, parse_xtask_command};

    #[test]
    fn minimal_workflow_parses() {
        assert_eq!(parse_workflow("minimal"), Ok(CrashWorkflow::Minimal));
    }

    #[test]
    fn unknown_workflow_fails_closed_with_available_list() {
        assert_eq!(
            parse_workflow("nope"),
            Err(XtaskCommandError::InvalidInput {
                command: "crash-lab".to_string(),
                input: "workflow".to_string(),
                reason: "unknown crash-lab workflow: nope; available workflows: minimal"
                    .to_string(),
            })
        );
    }

    #[test]
    fn action_dispatch_point_parses() {
        assert_eq!(
            parse_point("action_dispatch"),
            Ok(CrashPoint::ActionDispatch)
        );
    }

    #[test]
    fn unknown_point_fails_closed_with_available_list() {
        assert_eq!(
            parse_point("nope"),
            Err(XtaskCommandError::InvalidInput {
                command: "crash-lab".to_string(),
                input: "crash-at".to_string(),
                reason:
                    "unknown crash-lab crash point: nope; available crash points: action_dispatch"
                        .to_string(),
            })
        );
    }

    #[test]
    fn names_are_case_sensitive() {
        assert!(parse_workflow("MINIMAL").is_err());
        assert!(parse_workflow("").is_err());
        assert!(parse_point("ActionDispatch").is_err());
        assert!(parse_point("").is_err());
    }

    #[test]
    fn minimal_action_dispatch_targets_the_crash_lab_tests() {
        assert_eq!(
            CrashPoint::ActionDispatch.runs_for(CrashWorkflow::Minimal),
            &[
                ("vb_runtime", "journal_before_dispatch"),
                ("vb_runtime", "crash_recovery"),
                ("vb_storage", "snapshot_tail_matches_full_journal"),
            ]
        );
        assert_eq!(CrashWorkflow::Minimal.name(), "minimal");
        assert_eq!(CrashPoint::ActionDispatch.name(), "action_dispatch");
        assert_eq!(available_workflows(), &["minimal"]);
        assert_eq!(available_points(), &["action_dispatch"]);
        assert_eq!(
            all_points_for(CrashWorkflow::Minimal),
            &[CrashPoint::ActionDispatch]
        );
    }

    #[test]
    fn resolve_single_point() {
        assert_eq!(
            resolve_points("minimal", Some("action_dispatch"), false),
            Ok((CrashWorkflow::Minimal, vec![CrashPoint::ActionDispatch]))
        );
    }

    #[test]
    fn resolve_all_points() {
        assert_eq!(
            resolve_points("minimal", None, true),
            Ok((CrashWorkflow::Minimal, vec![CrashPoint::ActionDispatch]))
        );
    }

    #[test]
    fn resolve_both_flags_fails_closed() {
        assert_eq!(
            resolve_points("minimal", Some("action_dispatch"), true),
            Err(XtaskCommandError::InvalidInput {
                command: "crash-lab".to_string(),
                input: "crash-at".to_string(),
                reason: "--crash-at and --all-crash-points are mutually exclusive".to_string(),
            })
        );
    }

    #[test]
    fn resolve_neither_flag_fails_closed() {
        assert_eq!(
            resolve_points("minimal", None, false),
            Err(XtaskCommandError::MissingRequiredInput {
                command: "crash-lab".to_string(),
                input: "--crash-at or --all-crash-points".to_string(),
            })
        );
    }

    #[test]
    fn resolve_unknown_workflow_fails_before_flag_check() {
        assert!(resolve_points("nope", None, false).is_err());
    }

    #[test]
    fn crash_lab_routes_to_the_legacy_cli() {
        let parsed = parse_xtask_command([
            "xtask".into(),
            "crash-lab".into(),
            "--workflow".into(),
            "minimal".into(),
            "--crash-at".into(),
            "action_dispatch".into(),
        ]);
        assert_eq!(parsed, Ok(XtaskCommand::Legacy("crash-lab")));
    }
}
