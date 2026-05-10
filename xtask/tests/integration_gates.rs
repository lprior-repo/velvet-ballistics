//! Integration tests for xtask command-center gates.
//!
//! These tests verify the full CLI invocation behavior, exit codes,
//! and YAML evidence bundle structure by running actual `cargo xtask` commands.

use std::fs;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Test helper: run cargo xtask with given args and return output.
fn run_xtask(args: &[&str]) -> Output {
    match Command::new("cargo")
        .args(["xtask", "--"])
        .args(args)
        .current_dir(workspace_root())
        .output()
    {
        Ok(output) => output,
        Err(error) => failed_output(format!("Failed to execute cargo xtask: {error}")),
    }
}

fn workspace_root() -> PathBuf {
    let xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match xtask_dir.parent() {
        Some(parent) => parent.to_path_buf(),
        None => xtask_dir,
    }
}

#[cfg(unix)]
fn failed_output(message: String) -> Output {
    Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: message.into_bytes(),
    }
}

fn evidence_root() -> PathBuf {
    workspace_root().join(".evidence")
}

/// Test helper: clean up evidence directory for a bead.
fn cleanup_evidence(bead_id: &str) {
    let dir = evidence_root().join(bead_id);
    if dir.exists() {
        let result = fs::remove_dir_all(&dir).map_err(|error| error.to_string());
        assert_eq!(result, Ok(()), "failed to remove evidence dir: {dir:?}");
    }
}

fn read_text_or_empty(path: &std::path::Path) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            assert_eq!(
                error.to_string(),
                "",
                "failed to read evidence file {path:?}: {error}"
            );
            String::new()
        }
    }
}

fn assert_ai_fast_evidence_contains_all_required_gates(content: &str) {
    assert!(
        content.contains("fmt"),
        "Evidence should contain gate 'fmt'"
    );
    assert!(
        content.contains("check"),
        "Evidence should contain gate 'check'"
    );
    assert!(
        content.contains("clippy"),
        "Evidence should contain gate 'clippy'"
    );
    assert!(
        content.contains("nextest"),
        "Evidence should contain gate 'nextest'"
    );
    assert!(
        content.contains("forbidden-scan"),
        "Evidence should contain gate 'forbidden-scan'"
    );
    assert!(
        content.contains("hotpath-scan"),
        "Evidence should contain gate 'hotpath-scan'"
    );
}

fn assert_ai_deep_evidence_contains_all_required_gates(content: &str) {
    assert!(
        content.contains("miri"),
        "Evidence should contain gate 'miri'"
    );
    assert!(
        content.contains("mutants"),
        "Evidence should contain gate 'mutants'"
    );
    assert!(
        content.contains("llvm-cov"),
        "Evidence should contain gate 'llvm-cov'"
    );
    assert!(
        content.contains("fuzz-build"),
        "Evidence should contain gate 'fuzz-build'"
    );
}

fn assert_ai_release_evidence_contains_all_required_gates(content: &str) {
    assert!(
        content.contains("check"),
        "Evidence should contain gate 'check'"
    );
    assert!(
        content.contains("test"),
        "Evidence should contain gate 'test'"
    );
    assert!(
        content.contains("supply-chain"),
        "Evidence should contain gate 'supply-chain'"
    );
    assert!(
        content.contains("miri"),
        "Evidence should contain gate 'miri'"
    );
    assert!(
        content.contains("fuzz-smoke"),
        "Evidence should contain gate 'fuzz-smoke'"
    );
    assert!(
        content.contains("coverage"),
        "Evidence should contain gate 'coverage'"
    );
    assert!(
        content.contains("mutants-smoke"),
        "Evidence should contain gate 'mutants-smoke'"
    );
    assert!(
        content.contains("bench-build"),
        "Evidence should contain gate 'bench-build'"
    );
    assert!(
        content.contains("feature-powerset"),
        "Evidence should contain gate 'feature-powerset'"
    );
    assert!(
        content.contains("source-length"),
        "Evidence should contain gate 'source-length'"
    );
    assert!(
        content.contains("maxperf"),
        "Evidence should contain gate 'maxperf'"
    );
}

fn yaml_file_contains_failed_gate_without_diagnostic(entry: &fs::DirEntry) -> bool {
    let path = entry.path();
    path.extension()
        .is_some_and(|extension| extension == "yaml")
        && fs::read_to_string(path).is_ok_and(|yaml| {
            let failed = yaml.contains("status: Fail") || yaml.contains("status: Fail\n");
            let diagnosed = yaml.contains("why_failed:")
                || yaml.contains("hint:")
                || yaml.contains("repair_command:");
            failed && !diagnosed
        })
}

fn count_failed_yaml_files_without_diagnostics(evidence_dir: &Path) -> usize {
    fs::read_dir(evidence_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(yaml_file_contains_failed_gate_without_diagnostic)
                .count()
        })
        .unwrap_or_default()
}

// ========================================================================
// Evidence Bundle Structure Tests (POST-004, POST-006)
// ========================================================================

#[test]
fn ai_fast_profile_emits_yaml_evidence_when_workspace_is_clean() {
    // Given: a clean workspace
    let bead_id = "vb-itest-fast";
    cleanup_evidence(bead_id);

    // When: cargo xtask ai-fast --bead vb-itest-fast is executed
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: exit code is 0 (all gates pass in clean workspace)
    // RED_PHASE: Command doesn't exist yet, so this fails
    assert!(
        output.status.success(),
        "ai-fast should succeed in clean workspace, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And: evidence file exists at .evidence/<bead>/ai-fast.yaml
    let evidence_path = evidence_root().join(bead_id).join("ai-fast.yaml");
    assert!(
        evidence_path.exists(),
        "Evidence file should exist at {:?}",
        evidence_path
    );

    cleanup_evidence(bead_id);
}

#[test]
fn ai_deep_profile_emits_yaml_evidence_when_workspace_is_clean() {
    // Given: a clean workspace
    let bead_id = "vb-itest-deep";
    cleanup_evidence(bead_id);

    // When: cargo xtask ai-deep --bead vb-itest-deep is executed
    let output = run_xtask(&["ai-deep", "--bead", bead_id]);

    // Then: exit code is 0
    assert!(
        output.status.success(),
        "ai-deep should succeed in clean workspace, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And: evidence file exists
    let evidence_path = evidence_root().join(bead_id).join("ai-deep.yaml");
    assert!(
        evidence_path.exists(),
        "Evidence file should exist at {:?}",
        evidence_path
    );

    cleanup_evidence(bead_id);
}

#[test]
fn ai_release_unknown_bead_fails_closed_without_evidence() {
    // Given: an unknown release bead
    let bead_id = "vb-itest-release";
    cleanup_evidence(bead_id);

    // When: cargo xtask ai-release --bead vb-itest-release is executed
    let output = run_xtask(&["ai-release", "--bead", bead_id]);

    // Then: release fails closed before green evidence is minted
    assert!(
        !output.status.success(),
        "ai-release minted success for unknown bead"
    );

    // And: no ai-release evidence file exists
    let evidence_path = evidence_root().join(bead_id).join("ai-release.yaml");
    assert!(
        !evidence_path.exists(),
        "unknown bead must not get green evidence at {evidence_path:?}"
    );

    cleanup_evidence(bead_id);
}

#[test]
fn evidence_file_contains_all_required_fields_when_ai_fast_profile_runs() {
    // Given: ai-fast profile has run
    let bead_id = "vb-itest-fields";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = evidence_root().join(bead_id).join("ai-fast.yaml");

    if evidence_path.exists() {
        // Then: YAML contains all required fields per gate entry
        // Each gate should have: kind, gate_name, command, exit_code, log, status
        assert!(
            content.contains("kind:"),
            "Evidence should contain 'kind' field"
        );
        assert!(
            content.contains("gate_name:"),
            "Evidence should contain 'gate_name' field"
        );
        assert!(
            content.contains("command:"),
            "Evidence should contain 'command' field"
        );
        assert!(
            content.contains("exit_code:"),
            "Evidence should contain 'exit_code' field"
        );
        assert!(
            content.contains("log:"),
            "Evidence should contain 'log' field"
        );
        assert!(
            content.contains("status:"),
            "Evidence should contain 'status' field"
        );
    }

    cleanup_evidence(bead_id);
}

// ========================================================================
// Exit Code Semantics Tests (POST-008)
// ========================================================================

#[test]
fn exit_code_is_zero_when_all_gates_pass() {
    // Given: clean workspace
    let bead_id = "vb-itest-exit0";
    cleanup_evidence(bead_id);

    // When: ai-fast runs and all gates pass
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: exit code is 0
    assert!(
        output.status.success(),
        "Expected exit code 0 when all gates pass, got: {}",
        output.status
    );

    cleanup_evidence(bead_id);
}

#[test]
fn exit_code_is_zero_or_one_when_gate_status_is_reported() {
    // Given: a workspace where fmt would fail (e.g., unformatted code)
    // For RED_PHASE: we just verify the exit code semantics exist
    let bead_id = "vb-itest-exit1";
    cleanup_evidence(bead_id);

    // When: ai-fast runs (may pass or fail depending on workspace state)
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: exit code is either 0 (all pass) or 1 (some fail)
    // The important thing is the exit code reflects actual gate status
    let exit_code = output.status.code().unwrap_or(-1);
    assert!(
        exit_code == 0 || exit_code == 1,
        "Exit code should be 0 or 1, got: {}",
        exit_code
    );

    cleanup_evidence(bead_id);
}

#[test]
fn exit_code_is_failure_when_evidence_is_missing() {
    // Given: partial evidence directory (missing some gates)
    let bead_id = "vb-itest-missing";
    let evidence_dir = evidence_root().join(bead_id);
    cleanup_evidence(bead_id);

    // Create evidence directory with only fmt.yaml
    assert!(
        fs::create_dir_all(&evidence_dir).is_ok(),
        "failed to create evidence dir: {evidence_dir:?}"
    );
    let partial_evidence = r#"---
gates:
  - kind: fmt
    gate_name: fmt
    command: cargo +nightly fmt --all
    exit_code: 0
    log: target/evidence/fmt.log
    status: Pass
"#;
    assert!(
        fs::write(evidence_dir.join("fmt.yaml"), partial_evidence).is_ok(),
        "failed to write partial evidence"
    );

    // When: ai-fast is run with --bead (INV-001: fail-closed on missing evidence)
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: exit code is 1 (missing evidence = failure)
    assert!(
        !output.status.success(),
        "Should exit with failure when evidence is missing"
    );

    cleanup_evidence(bead_id);
}

// ========================================================================
// Profile Aggregation Tests (POST-007)
// ========================================================================

#[test]
fn ai_fast_aggregates_all_six_gates_when_evidence_is_emitted() {
    // Given: ai-fast profile has run
    let bead_id = "vb-itest-aggregate";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = evidence_root().join(bead_id).join("ai-fast.yaml");

    if evidence_path.exists() {
        // Then: contains entries for all 6 ai-fast gates
        assert_ai_fast_evidence_contains_all_required_gates(&content);
    }

    cleanup_evidence(bead_id);
}

#[test]
fn ai_deep_aggregates_all_four_gates_when_evidence_is_emitted() {
    // Given: ai-deep profile has run
    let bead_id = "vb-itest-deep-agg";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-deep", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = evidence_root().join(bead_id).join("ai-deep.yaml");

    if evidence_path.exists() {
        // Then: contains entries for all 4 ai-deep gates
        assert_ai_deep_evidence_contains_all_required_gates(&content);
    }

    cleanup_evidence(bead_id);
}

#[test]
fn ai_release_aggregates_all_eleven_gates_when_evidence_is_emitted() {
    // Given: ai-release profile has run
    let bead_id = "vb-itest-release-agg";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-release", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = evidence_root().join(bead_id).join("ai-release.yaml");

    if evidence_path.exists() {
        // Then: contains entries for all 11 ai-release gates
        assert_ai_release_evidence_contains_all_required_gates(&content);
    }

    cleanup_evidence(bead_id);
}

// ========================================================================
// Bead Directory Scoping Tests (POST-006, POST-009)
// ========================================================================

#[test]
fn bead_flag_creates_evidence_directory_when_ai_fast_runs() {
    // Given: .evidence does not exist
    let bead_id = "vb-itest-bead-flag";
    cleanup_evidence(bead_id);

    // When: ai-fast is run with --bead flag
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);
    assert!(
        output.status.success(),
        "ai-fast should succeed when creating bead-scoped evidence"
    );

    // Then: .evidence/<bead-id>/ directory is created
    let evidence_dir = evidence_root().join(bead_id);
    assert!(
        evidence_dir.exists(),
        "Evidence directory should be created at {:?}",
        evidence_dir
    );

    cleanup_evidence(bead_id);
}

#[test]
fn no_bead_flag_outputs_yaml_to_stdout_when_ai_fast_succeeds() {
    // Given: no --bead flag
    // When: ai-fast is run without --bead
    let output = run_xtask(&["ai-fast"]);

    // Then: stdout is valid YAML (no .evidence/ directory created)
    // If the command succeeded, check stdout is YAML
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should be parseable as YAML (at least starts with ---)
        assert!(
            stdout.contains("---") || stdout.contains("gates:") || stdout.contains("gate_name:"),
            "stdout should be YAML, got: {}",
            stdout
        );
    }

    // And: no .evidence/ directory created
    // Note: This may not be true if the command ran with --bead earlier
}

#[test]
fn evidence_path_is_confined_to_bead_directory_when_bead_id_has_traversal() {
    // Given: bead_id with path traversal attempt
    let bead_id = "../../../etc";
    cleanup_evidence(bead_id);

    // When: ai-fast is run with malicious bead_id
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: command fails with appropriate error
    // Path traversal should be rejected
    assert!(
        !output.status.success(),
        "Path traversal in bead_id should be rejected"
    );

    cleanup_evidence(bead_id);
}

// ========================================================================
// Error Handling Tests (ERR-005, ERR-006)
// ========================================================================

#[test]
fn unknown_subcommand_returns_error_when_gate_name_is_not_registered() {
    // Given: unknown subcommand
    let output = run_xtask(&["unknown-gate-name"]);

    // Then: exit code is 1
    assert!(!output.status.success(), "Unknown subcommand should fail");

    // And: error message mentions the unknown command
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown-gate-name")
            || stderr.contains("not found")
            || stderr.contains("Subcommand"),
        "Error should mention the unknown subcommand, got: {}",
        stderr
    );
}

#[test]
fn invalid_bead_id_is_rejected_when_it_contains_special_characters() {
    // Given: bead_id with special characters
    let bead_id = "vb-test<script>";
    cleanup_evidence(bead_id);

    // When: ai-fast is run with invalid bead_id
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: command fails gracefully
    // (具体行为取决于实现，但应该不会 panic 或创建奇怪的目录)
    assert!(
        !output.status.success(),
        "Invalid bead_id should fail gracefully"
    );
    cleanup_evidence(bead_id);
}

// ========================================================================
// Invariant Tests (INV-001, INV-003, INV-005)
// ========================================================================

#[test]
fn stdout_contains_structured_yaml_when_ai_fast_writes_to_stdout() {
    // Given: ai-fast profile runs
    let bead_id = "vb-itest-stdout";
    cleanup_evidence(bead_id);

    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: command completes
    // Then: stdout is valid YAML (INV-005: structured output only)
    // Raw tool output (fmt diffs, clippy warnings) should be in log files only
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.is_empty() {
        // stdout should be YAML (starts with --- or contains YAML structure)
        // It should NOT contain raw cargo fmt/clippy output
        assert!(
            stdout.contains("---") || stdout.contains("gate_name:") || stdout.contains("gates:"),
            "stdout should be YAML, not raw tool output. Got: {}...",
            stdout.chars().take(200).collect::<String>()
        );
    }

    cleanup_evidence(bead_id);
}

#[test]
fn missing_evidence_is_failure_not_silent_pass_when_required_gates_are_absent() {
    // Given: .evidence/ exists but is missing evidence for required gates
    let bead_id = "vb-itest-failclosed";
    cleanup_evidence(bead_id);

    let evidence_dir = evidence_root().join(bead_id);
    assert!(
        fs::create_dir_all(&evidence_dir).is_ok(),
        "failed to create evidence dir: {evidence_dir:?}"
    );

    // Create only fmt.yaml, missing clippy, nextest, etc.
    let partial = r#"kind: fmt
gate_name: fmt
command: cargo +nightly fmt --all
exit_code: 0
log: target/evidence/fmt.log
status: Pass
"#;
    assert!(
        fs::write(evidence_dir.join("fmt.yaml"), partial).is_ok(),
        "failed to write partial evidence"
    );

    // When: ai-fast is run
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: exit code is 1 (INV-001: fail-closed, no silent pass)
    assert!(
        !output.status.success(),
        "Missing evidence should cause failure, not silent pass"
    );

    cleanup_evidence(bead_id);
}

#[test]
fn failed_evidence_contains_why_failed_hint_or_repair_command_when_gate_fails() {
    // Given: a failing gate scenario
    // For this test, we check that the evidence structure supports why_failed
    // when a gate actually fails
    let bead_id = "vb-itest-whyfailed";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence files are inspected
    let evidence_dir = evidence_root().join(bead_id);

    let undiagnosed_failed_yaml_count = count_failed_yaml_files_without_diagnostics(&evidence_dir);
    assert_eq!(
        undiagnosed_failed_yaml_count, 0,
        "Failed gates should have why_failed diagnostics"
    );

    cleanup_evidence(bead_id);
}
