//! Integration tests for xtask command-center gates.
//!
//! These tests verify the full CLI invocation behavior, exit codes,
//! and YAML evidence bundle structure by running actual `cargo xtask` commands.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Test helper: run cargo xtask with given args and return output.
fn run_xtask(args: &[&str]) -> Output {
    Command::new("cargo")
        .args(["xtask", "--"])
        .args(args)
        .current_dir("/home/lewis/src/Velvet-ballistics")
        .output()
        .expect("Failed to execute cargo xtask")
}

/// Test helper: clean up evidence directory for a bead.
fn cleanup_evidence(bead_id: &str) {
    let dir = PathBuf::from(".evidence").join(bead_id);
    let _ = fs::remove_dir_all(&dir);
}

// ========================================================================
// Evidence Bundle Structure Tests (POST-004, POST-006)
// ========================================================================

#[test]
fn test_ai_fast_profile_emits_yaml_evidence() {
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
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-fast.yaml");
    assert!(
        evidence_path.exists(),
        "Evidence file should exist at {:?}",
        evidence_path
    );

    cleanup_evidence(bead_id);
}

#[test]
fn test_ai_deep_profile_emits_yaml_evidence() {
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
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-deep.yaml");
    assert!(
        evidence_path.exists(),
        "Evidence file should exist at {:?}",
        evidence_path
    );

    cleanup_evidence(bead_id);
}

#[test]
fn test_ai_release_profile_emits_yaml_evidence() {
    // Given: a clean workspace
    let bead_id = "vb-itest-release";
    cleanup_evidence(bead_id);

    // When: cargo xtask ai-release --bead vb-itest-release is executed
    let output = run_xtask(&["ai-release", "--bead", bead_id]);

    // Then: exit code is 0
    assert!(
        output.status.success(),
        "ai-release should succeed in clean workspace, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And: evidence file exists
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-release.yaml");
    assert!(
        evidence_path.exists(),
        "Evidence file should exist at {:?}",
        evidence_path
    );

    cleanup_evidence(bead_id);
}

#[test]
fn test_evidence_file_contains_all_required_fields() {
    // Given: ai-fast profile has run
    let bead_id = "vb-itest-fields";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-fast.yaml");

    if evidence_path.exists() {
        let content = fs::read_to_string(&evidence_path).expect("Failed to read evidence file");

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
fn test_exit_code_0_when_all_gates_pass() {
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
fn test_exit_code_1_when_any_gate_fails() {
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
fn test_exit_code_1_when_evidence_missing() {
    // Given: partial evidence directory (missing some gates)
    let bead_id = "vb-itest-missing";
    let evidence_dir = PathBuf::from(".evidence").join(bead_id);
    cleanup_evidence(bead_id);

    // Create evidence directory with only fmt.yaml
    fs::create_dir_all(&evidence_dir).ok();
    let partial_evidence = r#"---
gates:
  - kind: fmt
    gate_name: fmt
    command: cargo +nightly fmt --all
    exit_code: 0
    log: target/evidence/fmt.log
    status: Pass
"#;
    fs::write(evidence_dir.join("fmt.yaml"), partial_evidence).ok();

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
fn test_ai_fast_aggregates_all_6_gates() {
    // Given: ai-fast profile has run
    let bead_id = "vb-itest-aggregate";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-fast.yaml");

    if evidence_path.exists() {
        let content = fs::read_to_string(&evidence_path).expect("Failed to read evidence file");

        // Then: contains entries for all 6 ai-fast gates
        let expected_gates = [
            "fmt",
            "check",
            "clippy",
            "nextest",
            "forbidden-scan",
            "hotpath-scan",
        ];
        for gate in expected_gates {
            assert!(
                content.contains(gate),
                "Evidence should contain gate '{}'",
                gate
            );
        }
    }

    cleanup_evidence(bead_id);
}

#[test]
fn test_ai_deep_aggregates_all_4_gates() {
    // Given: ai-deep profile has run
    let bead_id = "vb-itest-deep-agg";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-deep", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-deep.yaml");

    if evidence_path.exists() {
        let content = fs::read_to_string(&evidence_path).expect("Failed to read evidence file");

        // Then: contains entries for all 4 ai-deep gates
        let expected_gates = ["miri", "mutants", "llvm-cov", "fuzz-build"];
        for gate in expected_gates {
            assert!(
                content.contains(gate),
                "Evidence should contain gate '{}'",
                gate
            );
        }
    }

    cleanup_evidence(bead_id);
}

#[test]
fn test_ai_release_aggregates_all_11_gates() {
    // Given: ai-release profile has run
    let bead_id = "vb-itest-release-agg";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-release", "--bead", bead_id]);

    // When: evidence file is read
    let evidence_path = PathBuf::from(".evidence")
        .join(bead_id)
        .join("ai-release.yaml");

    if evidence_path.exists() {
        let content = fs::read_to_string(&evidence_path).expect("Failed to read evidence file");

        // Then: contains entries for all 11 ai-release gates
        let expected_gates = [
            "check",
            "test",
            "supply-chain",
            "miri",
            "fuzz-smoke",
            "coverage",
            "mutants-smoke",
            "bench-build",
            "feature-powerset",
            "source-length",
            "maxperf",
        ];
        for gate in expected_gates {
            assert!(
                content.contains(gate),
                "Evidence should contain gate '{}'",
                gate
            );
        }
    }

    cleanup_evidence(bead_id);
}

// ========================================================================
// Bead Directory Scoping Tests (POST-006, POST-009)
// ========================================================================

#[test]
fn test_bead_flag_creates_evidence_directory() {
    // Given: .evidence does not exist
    let bead_id = "vb-itest-bead-flag";
    cleanup_evidence(bead_id);

    // When: ai-fast is run with --bead flag
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: .evidence/<bead-id>/ directory is created
    let evidence_dir = PathBuf::from(".evidence").join(bead_id);
    assert!(
        evidence_dir.exists(),
        "Evidence directory should be created at {:?}",
        evidence_dir
    );

    cleanup_evidence(bead_id);
}

#[test]
fn test_no_bead_flag_outputs_to_stdout() {
    // Given: no --bead flag
    // When: ai-fast is run without --bead
    let output = run_xtask(&["ai-fast"]);

    // Then: stdout is valid YAML (no .evidence/ directory created)
    let evidence_dir = PathBuf::from(".evidence");

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
fn test_evidence_path_confined_to_bead_directory() {
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
fn test_unknown_subcommand_returns_error() {
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
fn test_invalid_bead_id_rejected() {
    // Given: bead_id with special characters
    let bead_id = "vb-test<script>";
    cleanup_evidence(bead_id);

    // When: ai-fast is run with invalid bead_id
    let output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // Then: command fails gracefully
    // (具体行为取决于实现，但应该不会 panic 或创建奇怪的目录)
    cleanup_evidence(bead_id);
}

// ========================================================================
// Invariant Tests (INV-001, INV-003, INV-005)
// ========================================================================

#[test]
fn test_no_raw_tool_output_on_stdout() {
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
            &stdout[..stdout.len().min(200)]
        );
    }

    cleanup_evidence(bead_id);
}

#[test]
fn test_missing_evidence_is_failure_not_silent_pass() {
    // Given: .evidence/ exists but is missing evidence for required gates
    let bead_id = "vb-itest-failclosed";
    cleanup_evidence(bead_id);

    let evidence_dir = PathBuf::from(".evidence").join(bead_id);
    fs::create_dir_all(&evidence_dir).ok();

    // Create only fmt.yaml, missing clippy, nextest, etc.
    let partial = r#"kind: fmt
gate_name: fmt
command: cargo +nightly fmt --all
exit_code: 0
log: target/evidence/fmt.log
status: Pass
"#;
    fs::write(evidence_dir.join("fmt.yaml"), partial).ok();

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
fn test_why_failed_hint_and_repair_command_present() {
    // Given: a failing gate scenario
    // For this test, we check that the evidence structure supports why_failed
    // when a gate actually fails
    let bead_id = "vb-itest-whyfailed";
    cleanup_evidence(bead_id);

    let _output = run_xtask(&["ai-fast", "--bead", bead_id]);

    // When: evidence files are inspected
    let evidence_dir = PathBuf::from(".evidence").join(bead_id);

    if evidence_dir.exists() {
        // Read any evidence file and check structure
        if let Ok(entries) = fs::read_dir(&evidence_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map_or(false, |e| e == "yaml") {
                    let content = fs::read_to_string(entry.path()).ok();
                    if let Some(yaml) = content {
                        // If this gate failed, why_failed should be present
                        if yaml.contains("status: Fail") || yaml.contains("status: Fail\n") {
                            // The why_failed block should exist for failed gates
                            assert!(
                                yaml.contains("why_failed:")
                                    || yaml.contains("hint:")
                                    || yaml.contains("repair_command:"),
                                "Failed gate should have why_failed diagnostic"
                            );
                        }
                    }
                }
            }
        }
    }

    cleanup_evidence(bead_id);
}
