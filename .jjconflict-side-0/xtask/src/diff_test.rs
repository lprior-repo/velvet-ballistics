#![allow(unreachable_pub)]
//! Differential test suites (spec section 77.11).
//!
//! `cargo xtask diff-test --suite <name>` runs one differential suite: two
//! implementations of the same behavior compared against each other. The
//! suite name is parsed at the boundary into `DiffSuite`; the package and
//! test filter it runs come from a fixed table, never from the raw string.

#![allow(clippy::print_stderr)]

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use xtask::XtaskCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSuite {
    Replay,
}

impl DiffSuite {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Replay => "replay",
        }
    }

    #[must_use]
    pub const fn target(self) -> (&'static str, &'static str) {
        match self {
            Self::Replay => ("vb_storage", "snapshot_tail_matches_full_journal"),
        }
    }
}

#[must_use]
pub fn available_suites() -> &'static [&'static str] {
    &["replay"]
}

pub fn parse_suite(name: &str) -> Result<DiffSuite, XtaskCommandError> {
    match name {
        "replay" => Ok(DiffSuite::Replay),
        _ => Err(XtaskCommandError::InvalidInput {
            command: "diff-test".to_string(),
            input: "suite".to_string(),
            reason: format!(
                "unknown diff-test suite: {name}; available suites: {}",
                available_suites().join(", ")
            ),
        }),
    }
}

pub fn cmd_diff_test(suite_name: &str, workspace_root: &Path) -> anyhow::Result<()> {
    let suite = parse_suite(suite_name).map_err(anyhow::Error::msg)?;
    let (package, filter) = suite.target();
    eprintln!(
        "Running diff-test suite {} (cargo nextest run -p {} {})",
        suite.name(),
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
        .context("Failed to run diff-test suite (is cargo-nextest installed?)")?;
    if status.success() {
        eprintln!(
            "PASS: diff-test suite '{}' completed successfully",
            suite.name()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "diff-test suite '{}' failed with exit code {:?}",
            suite.name(),
            status.code()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xtask::{XtaskCommand, parse_xtask_command};

    #[test]
    fn replay_suite_parses() {
        assert_eq!(parse_suite("replay"), Ok(DiffSuite::Replay));
    }

    #[test]
    fn unknown_suite_fails_closed_with_available_list() {
        assert_eq!(
            parse_suite("nope"),
            Err(XtaskCommandError::InvalidInput {
                command: "diff-test".to_string(),
                input: "suite".to_string(),
                reason: "unknown diff-test suite: nope; available suites: replay".to_string(),
            })
        );
    }

    #[test]
    fn suite_names_are_case_sensitive() {
        assert!(parse_suite("REPLAY").is_err());
        assert!(parse_suite("").is_err());
    }

    #[test]
    fn replay_suite_targets_the_four_snapshot_tail_tests() {
        assert_eq!(
            DiffSuite::Replay.target(),
            ("vb_storage", "snapshot_tail_matches_full_journal")
        );
        assert_eq!(DiffSuite::Replay.name(), "replay");
        assert_eq!(available_suites(), &["replay"]);
    }

    #[test]
    fn diff_test_routes_to_the_legacy_cli() {
        let parsed = parse_xtask_command([
            "xtask".into(),
            "diff-test".into(),
            "--suite".into(),
            "replay".into(),
        ]);
        assert_eq!(parsed, Ok(XtaskCommand::Legacy("diff-test")));
    }
}
