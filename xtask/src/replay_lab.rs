#![allow(unreachable_pub)]
//! Replay lab (spec section 76.1, VB-REPLAY-001 / VB-REPLAY-007).
//!
//! `cargo xtask replay-lab` runs the differential replay tests: the legs
//! that compare two replay paths or pin replay-after-crash behavior
//! (snapshot + tail matches full journal, journal committed before
//! dispatch, recovery reads partial events written before a crash).
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
    ]
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
            ]
        );
    }

    #[test]
    fn replay_lab_routes_to_the_legacy_cli() {
        let parsed = parse_xtask_command(["xtask".into(), "replay-lab".into()]);
        assert_eq!(parsed, Ok(XtaskCommand::Legacy("replay-lab")));
    }
}
