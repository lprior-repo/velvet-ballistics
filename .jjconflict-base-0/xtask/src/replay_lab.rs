#![allow(unreachable_pub)]
//! Replay lab (spec section 76.1, VB-REPLAY-001 / VB-REPLAY-007).
//!
//! `cargo xtask replay-lab` runs the differential replay tests: the legs
//! that compare two replay paths or pin replay-after-crash behavior
//! (snapshot + tail matches full journal, journal committed before
//! dispatch, recovery reads partial events written before a crash,
//! production engine and taint lattice match the reference models).
//! The package and test filter each leg runs come from a fixed table,
//! never from user input; the command takes no arguments.

#![allow(clippy::print_stderr)]

use std::path::Path;
use std::process::Command;

use anyhow::Context;

#[must_use]
pub fn replay_legs() -> &'static [(&'static str, &'static str)] {
    &[
        ("vb_runtime", "journal_before_dispatch"),
        ("vb_runtime", "crash_recovery"),
        ("vb_storage", "snapshot_tail_matches_full_journal"),
        ("velvet-ballistics-workspace-tests", "engine_vs_reference"),
        ("velvet-ballistics-workspace-tests", "taint_vs_reference"),
    ]
}

fn nextest_args<'a>(package: &'a str, filter: &'a str) -> Vec<&'a str> {
    let mut args = vec!["nextest", "run", "-p", package];
    if package == "velvet-ballistics-workspace-tests" {
        args.extend(["--test", "reference_model_regressions"]);
    }
    args.push(filter);
    args
}

fn run_nextest(package: &str, filter: &str, workspace_root: &Path) -> anyhow::Result<()> {
    let args = nextest_args(package, filter);
    eprintln!("Running cargo {args:?}");
    let status = Command::new("cargo")
        .args(&args)
        .current_dir(workspace_root)
        .status()
        .context("Failed to run replay-lab tests (is cargo-nextest installed?)")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "replay-lab tests failed: cargo nextest run -p {package} {filter} exited with {:?}",
            status.code()
        )
    }
}

pub fn cmd_replay_lab(workspace_root: &Path) -> anyhow::Result<()> {
    replay_legs()
        .iter()
        .try_for_each(|(package, filter)| run_nextest(package, filter, workspace_root))?;
    eprintln!("PASS: replay-lab completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xtask::{XtaskCommand, parse_xtask_command};

    #[test]
    fn replay_legs_are_the_replay_differential_legs() {
        assert_eq!(
            replay_legs(),
            &[
                ("vb_runtime", "journal_before_dispatch"),
                ("vb_runtime", "crash_recovery"),
                ("vb_storage", "snapshot_tail_matches_full_journal"),
                ("velvet-ballistics-workspace-tests", "engine_vs_reference"),
                ("velvet-ballistics-workspace-tests", "taint_vs_reference"),
            ]
        );
    }

    #[test]
    fn replay_lab_routes_to_the_legacy_cli() {
        let parsed = parse_xtask_command(["xtask".into(), "replay-lab".into()]);
        assert_eq!(parsed, Ok(XtaskCommand::Legacy("replay-lab")));
    }

    #[test]
    fn reference_legs_select_the_registered_integration_target() {
        assert_eq!(
            nextest_args("velvet-ballistics-workspace-tests", "engine_vs_reference"),
            vec![
                "nextest",
                "run",
                "-p",
                "velvet-ballistics-workspace-tests",
                "--test",
                "reference_model_regressions",
                "engine_vs_reference"
            ]
        );
        assert_eq!(
            nextest_args("vb_runtime", "journal_before_dispatch"),
            vec![
                "nextest",
                "run",
                "-p",
                "vb_runtime",
                "journal_before_dispatch"
            ]
        );
    }
}
