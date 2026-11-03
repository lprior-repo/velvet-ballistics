//! Regression tests for hands-on QA findings (vb-c5wp6).
//!
//! 1. `run` twice into the same durable database must succeed with distinct
//!    run ids (previously the second run failed at header persistence because
//!    every execution reused run 1).
//! 2. Read commands against a nonexistent `--db` must report the missing
//!    directory instead of "no events found".
//! 3. `<command> --help` must print scoped usage, not the full command list.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const MINIMAL_YAML: &str = "version: velvet-ballistics/v1\nname: minimal_workflow\nwhen:\n  manual: {}\nsteps:\n  - id: step_hello\n    set:\n      output: greeting\n      value: \"1\"\n  - id: done\n    finish:\n      result: greeting\n";

fn cli(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output()
        .expect("failed to spawn velvet-ballistics");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn first_stdout_line(stdout: &str) -> &str {
    stdout.lines().next().unwrap_or("")
}

#[test]
fn run_twice_into_same_db_succeeds_with_distinct_ids(
) -> Result<(), Box<dyn std::error::Error>> {
    let base = tempdir()?;
    let workflow = base.path().join("minimal.yaml");
    fs::write(&workflow, MINIMAL_YAML)?;
    let input = base.path().join("input.bin");
    fs::write(&input, [])?;
    let db = base.path().join("db");
    let workflow_str = workflow.to_str().ok_or("non-utf8 temp path")?;
    let input_str = input.to_str().ok_or("non-utf8 temp path")?;
    let db_str = db.to_str().ok_or("non-utf8 temp path")?;

    let run_once = || {
        cli(&[
            "run",
            workflow_str,
            "--input-bin",
            input_str,
            "--durability",
            "journaled",
            "--db",
            db_str,
        ])
    };

    let (first_ok, first_out, first_err) = run_once();
    assert!(first_ok, "first run failed: stdout={first_out} stderr={first_err}");
    let first_line = first_stdout_line(&first_out).to_string();
    assert!(
        first_line.starts_with("run 1: submitted=1 completed=1 failed=0 steps=2"),
        "first run into a fresh db keeps run 1, got: {first_line}"
    );

    let (second_ok, second_out, second_err) = run_once();
    assert!(
        second_ok,
        "second run into the same db failed: stdout={second_out} stderr={second_err}"
    );
    let second_line = first_stdout_line(&second_out);
    assert!(
        second_line.starts_with("run ") && !second_line.starts_with("run 1: "),
        "second run must use a fresh id, got: {second_line}"
    );
    assert!(
        second_out.contains("run completed"),
        "second run must complete: {second_out}"
    );
    Ok(())
}

#[test]
fn read_commands_reject_missing_db_directory() -> Result<(), Box<dyn std::error::Error>> {
    let base = tempdir()?;
    let missing = base.path().join("does-not-exist");
    let missing_str = missing.to_str().ok_or("non-utf8 temp path")?;
    assert!(!Path::new(missing_str).exists());

    for command in ["inspect", "events", "replay"] {
        let (ok, stdout, stderr) = cli(&[command, "1", "--db", missing_str]);
        assert!(!ok, "{command} unexpectedly succeeded against a missing db");
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("does not exist"),
            "{command} must report the missing directory, got: {combined}"
        );
    }
    Ok(())
}

#[test]
fn subcommand_help_is_scoped_to_command() -> Result<(), Box<dyn std::error::Error>> {
    let (ok, stdout, stderr) = cli(&["validate", "--help"]);
    assert!(ok, "validate --help failed: stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("validate   <workflow.yaml>"),
        "scoped help must show the validate usage line, got: {stdout}"
    );
    assert!(
        !stdout.contains("ipc-serve"),
        "scoped help must not dump the full command list, got: {stdout}"
    );

    let (full_ok, full_out, _) = cli(&["help"]);
    assert!(full_ok, "bare help failed");
    assert!(
        full_out.contains("ipc-serve") && full_out.contains("validate"),
        "bare help must keep the full command list, got: {full_out}"
    );
    Ok(())
}
