//! Deterministic integration tests for xtask gate orchestration.

use std::path::Path;
use std::process::Command;

use xtask::evidence::{
    Error, GateEvidence, GateProfile, GateStatus, Result, command_for_gate, explain_failure,
    run_profile_with_runner, validate_bead_id, validate_evidence_dir, write_evidence,
};

#[test]
fn ai_fast_profile_emits_yaml_evidence_without_nested_cargo() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let result = run_profile_with_runner(
        GateProfile::Fast,
        Some("vb-itest-fast"),
        temp.path(),
        passing_runner,
    );

    assert!(matches!(result, Ok(ref evidence) if evidence.exit_code == 0));
    assert!(temp.path().join("ai-fast.yaml").exists());
    assert!(temp.path().join("fmt.yaml").exists());
}

#[test]
fn profile_fails_closed_when_any_gate_fails() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let result = run_profile_with_runner(
        GateProfile::Fast,
        Some("vb-itest-fail"),
        temp.path(),
        failing_nextest_runner,
    );

    assert!(matches!(result, Ok(ref evidence) if evidence.exit_code == 1));
    let evidence = result.expect("profile evidence should be returned");
    assert!(evidence.gates.iter().any(|gate| {
        gate.gate_name == "nextest" && gate.status == GateStatus::Fail && gate.why_failed.is_some()
    }));
}

#[test]
fn release_profile_has_fail_closed_commands_for_all_gates() {
    assert!(
        GateProfile::Release
            .gates()
            .iter()
            .all(|gate| matches!(command_for_gate(gate), Ok(ref command) if !command.is_empty()))
    );
    assert!(matches!(
        command_for_gate("definitely-not-a-gate"),
        Err(Error::SubcommandNotFound { .. })
    ));
}

#[test]
fn bead_ids_reject_path_traversal() {
    assert!(validate_bead_id("vb-good.1_2-3").is_ok());
    assert!(matches!(
        validate_bead_id("../escape"),
        Err(Error::InvalidBeadId { .. })
    ));
}

#[test]
fn invalid_bead_id_is_rejected_before_runner_executes() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let result = run_profile_with_runner(
        GateProfile::Fast,
        Some("../escape"),
        temp.path(),
        passing_runner,
    );

    assert!(matches!(result, Err(Error::InvalidBeadId { .. })));
    assert!(!temp.path().join("fmt.yaml").exists());
}

#[test]
fn validate_evidence_dir_reports_missing_files() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(temp.path().join("fmt.yaml"), "kind: fmt").expect("fixture should be written");

    let result = validate_evidence_dir(temp.path(), &["fmt", "clippy", "nextest"]);

    assert!(matches!(result, Ok(ref errors) if errors.len() == 2));
}

#[test]
fn unknown_cli_subcommand_fails_at_binary_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("does-not-exist")
        .output()
        .expect("xtask binary should execute");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
}

#[test]
fn cli_rejects_invalid_bead_id_before_running_gates() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["ai-fast", "--bead", "../escape"])
        .output()
        .expect("xtask binary should execute");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid bead id"));
}

fn passing_runner(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    write_fake_evidence(gate, cmd, evidence_path, GateStatus::Pass, 0)
}

fn failing_nextest_runner(
    gate: &str,
    cmd: &[String],
    evidence_path: &Path,
) -> Result<GateEvidence> {
    if gate == "nextest" {
        write_fake_evidence(gate, cmd, evidence_path, GateStatus::Fail, 1)
    } else {
        write_fake_evidence(gate, cmd, evidence_path, GateStatus::Pass, 0)
    }
}

fn write_fake_evidence(
    gate: &str,
    cmd: &[String],
    evidence_path: &Path,
    status: GateStatus,
    exit_code: i32,
) -> Result<GateEvidence> {
    let mut evidence = GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: cmd.join(" "),
        exit_code,
        log: evidence_path.with_extension("log"),
        status,
        why_failed: None,
    };
    evidence.why_failed = explain_failure(&evidence);
    write_evidence(&evidence, evidence_path)?;
    Ok(evidence)
}
