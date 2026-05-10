//! Deterministic integration tests for xtask gate orchestration.

use std::path::Path;
use std::process::Command;

use xtask::evidence::{
    Error, GateEvidence, GateProfile, GateStatus, Result, command_for_gate,
    run_profile_with_runner, validate_bead_id, validate_evidence_dir, write_evidence,
};

fn evidence_for(
    gate: &str,
    command: &[String],
    evidence_path: &Path,
    status: GateStatus,
) -> GateEvidence {
    let exit_code = if status == GateStatus::Pass { 0 } else { 1 };
    GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: command.join(" "),
        exit_code,
        log: evidence_path.with_extension("log"),
        status,
        why_failed: None,
    }
}

fn passing_runner(gate: &str, command: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    let evidence = evidence_for(gate, command, evidence_path, GateStatus::Pass);
    write_evidence(&evidence, evidence_path)?;
    Ok(evidence)
}

fn failing_nextest_runner(
    gate: &str,
    command: &[String],
    evidence_path: &Path,
) -> Result<GateEvidence> {
    let status = if gate == "nextest" {
        GateStatus::Fail
    } else {
        GateStatus::Pass
    };
    let mut evidence = evidence_for(gate, command, evidence_path, status);
    if gate == "nextest" {
        evidence.why_failed = Some(xtask::evidence::WhyFailed {
            gate_name: gate.to_string(),
            hint: "deterministic fake failure".to_string(),
            repair_command: "moon run :test".to_string(),
        });
    }
    write_evidence(&evidence, evidence_path)?;
    Ok(evidence)
}

fn forbidden_runner(
    _gate: &str,
    _command: &[String],
    _evidence_path: &Path,
) -> Result<GateEvidence> {
    Err(Error::SubcommandNotFound {
        name: "runner should not be called".to_string(),
    })
}

#[test]
fn ai_fast_profile_emits_yaml_without_nested_cargo() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let profile = run_profile_with_runner(
        GateProfile::Fast,
        Some("vb-itest-fast"),
        temp.path(),
        passing_runner,
    )
    .expect("profile should run with fake runner");

    assert_eq!(profile.exit_code, 0);
    assert_eq!(profile.gates.len(), GateProfile::Fast.gates().len());
    assert!(temp.path().join("ai-fast.yaml").exists());
    assert!(
        validate_evidence_dir(temp.path(), GateProfile::Fast.gates())
            .expect("evidence validation should run")
            .is_empty()
    );
}

#[test]
fn profile_aggregation_fails_closed_on_failed_gate() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let profile = run_profile_with_runner(
        GateProfile::Fast,
        Some("vb-itest-fail"),
        temp.path(),
        failing_nextest_runner,
    )
    .expect("profile should collect fake failure evidence");

    assert_eq!(profile.exit_code, 1);
    assert!(profile.gates.iter().any(|gate| {
        gate.gate_name == "nextest" && gate.status == GateStatus::Fail && gate.why_failed.is_some()
    }));
}

#[test]
fn release_profile_gates_have_fail_closed_commands() {
    for gate in GateProfile::Release.gates() {
        let command = command_for_gate(gate).expect("release gate command should exist");
        assert!(!command.is_empty(), "{gate} command must not be empty");
    }

    assert!(matches!(
        command_for_gate("unknown-release-gate"),
        Err(Error::SubcommandNotFound { .. })
    ));
}

#[test]
fn invalid_bead_id_is_rejected_before_runner_execution() {
    let temp = tempfile::tempdir().expect("tempdir should be created");

    let result = run_profile_with_runner(
        GateProfile::Fast,
        Some("../../escape"),
        temp.path(),
        forbidden_runner,
    );

    assert!(matches!(result, Err(Error::InvalidBeadId { .. })));
}

#[test]
fn bead_id_validation_rejects_path_traversal_and_special_chars() {
    assert!(validate_bead_id("vb-safe_1.2").is_ok());
    assert!(matches!(
        validate_bead_id("../bad"),
        Err(Error::InvalidBeadId { .. })
    ));
    assert!(matches!(
        validate_bead_id("vb-test<script>"),
        Err(Error::InvalidBeadId { .. })
    ));
}

#[test]
fn validate_evidence_dir_reports_missing_files() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let evidence = evidence_for(
        "fmt",
        &["moon".to_string(), "run".to_string(), ":fmt".to_string()],
        &temp.path().join("fmt.yaml"),
        GateStatus::Pass,
    );
    write_evidence(&evidence, &temp.path().join("fmt.yaml")).expect("fmt evidence should write");

    let errors = validate_evidence_dir(temp.path(), &["fmt", "clippy", "nextest"])
        .expect("validation should succeed");

    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|error| matches!(error, Error::MissingEvidence { .. }))
    );
}

#[test]
fn binary_unknown_subcommand_fails_without_cargo_wrapper() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("unknown-gate-name")
        .output()
        .expect("xtask binary should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown-gate-name") || stderr.contains("unrecognized"));
}

#[test]
fn binary_invalid_bead_id_fails_before_profile_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["ai-fast", "--bead", "../../escape"])
        .output()
        .expect("xtask binary should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid bead id"));
}
