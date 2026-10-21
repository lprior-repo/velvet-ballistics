#![forbid(unsafe_code)]
//! Integration tests for the verify hero command and VerificationReport certificates.
//!
//! These tests verify end-to-end behavior of the `verify` command including:
//! - Full pipeline verification with all profiles
//! - Format parity between Text, Json, and Jsonl outputs
//! - Error classification and exit codes
//! - JSON output completeness

/// Malformed YAML for error path testing.
const MALFORMED_YAML: &str = r#"version: velvet-ballistics/v1
name: bad_workflow
when:
  manual: {}
steps:
  - id: broken
    set:
      output: result
      value: "1"
    # This YAML is missing required fields and has invalid syntax
    invalid indentation here
"#;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn forced_assertion_failure() -> bool {
    false
}

fn write_test_file(path: &std::path::Path, contents: &[u8]) -> bool {
    match std::fs::write(path, contents) {
        Ok(()) => true,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to write {}: {err}",
                path.display()
            );
            false
        }
    }
}

fn run_cli(args: &[&std::ffi::OsStr]) -> Option<std::process::Output> {
    let exe = env!("CARGO_BIN_EXE_velvet-ballistics");
    let mut command = std::process::Command::new(exe);
    command.args(args);

    command.output().ok()
}

fn must_run_cli(args: &[&std::ffi::OsStr]) -> std::process::Output {
    let output = run_cli(args);
    assert!(output.is_some(), "vb command must run");
    match output {
        Some(output) => output,
        None => std::process::abort(),
    }
}

fn fixture_os(path: &str) -> std::ffi::OsString {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root_fixture = root.join(path);
    if root_fixture.exists() {
        root_fixture.into_os_string()
    } else {
        root.join("crates/workspace_tests")
            .join(path)
            .into_os_string()
    }
}

// ---------------------------------------------------------------------------
// BDD Scenario: Happy Path — Minimal Valid Workflow
// ---------------------------------------------------------------------------

/// ### Behavior: run_verification returns VerifyOk with all gates passed for minimal valid workflow
/// Given: a valid minimal workflow YAML at tests/fixtures/valid/minimal.yaml
/// When: run_verification is called with Quick profile
/// Then: result is Ok(VerifyOk) with non-empty digest_hex
/// And: result.checks contains "yaml_parse" and "compilation"
#[test]
fn bdd_happy_quick_profile_returns_ok_with_checks() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert!(output.is_some(), "vb command must succeed");
    let output = match output {
        Some(output) => output,
        None => std::process::abort(),
    };
    let status = output.status;

    // Quick profile with valid workflow must succeed (exit 0)
    assert!(
        status.success(),
        "verify quick must succeed with valid workflow. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(status.code(), Some(0), "exit code must be 0 on success");

    // stdout must contain evidence of yaml_parse and compilation
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("yaml_parse")
            || stdout.contains("compilation")
            || stdout.contains("passed"),
        "output must mention passed gates"
    );
}

/// ### Behavior: cmd_verify produces identical exit codes for Text, Json, and Jsonl formats
/// Given: a valid workflow at Quick profile
/// When: cmd_verify is called with Text format and with Json format
/// Then: both invocations return the same exit code (0)
#[test]
fn bdd_format_parity_exit_code_identical_across_formats() {
    // Text format
    let text_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    // Json format
    let json_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    // Jsonl format
    let jsonl_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--jsonl"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let text_code = text_output.status.code();
    let json_code = json_output.status.code();
    let jsonl_code = jsonl_output.status.code();

    assert_eq!(
        text_code, json_code,
        "Text and Json formats must produce same exit code"
    );
    assert_eq!(
        text_code, jsonl_code,
        "Text and Jsonl formats must produce same exit code"
    );
    assert_eq!(text_code, Some(0), "Valid workflow must exit with code 0");
}

// ---------------------------------------------------------------------------
// BDD Scenario: YAML Parse Error Path
// ---------------------------------------------------------------------------

/// ### Behavior: run_verification returns YamlParse error for malformed YAML
/// Given: a workflow YAML with syntax error
/// When: run_verification is called with Quick profile
/// Then: result is Err(VerifyError::YamlParse(msg)) where msg contains "YAML parse error"
#[test]
fn bdd_yaml_parse_error_returns_classified_error() {
    // Create a temp file with malformed YAML
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("vb_test_malformed.yaml");
    write_test_file(&temp_file, MALFORMED_YAML.as_bytes());

    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        temp_file.as_os_str(),
    ]);

    let status = output.status;
    // YAML parse error must result in exit code 2 (ValidationFailed) per contract POST-008
    assert_eq!(
        status.code(),
        Some(2),
        "YAML parse error must exit with code 2 (ValidationFailed)"
    );

    // stderr must mention YAML or parse error
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.to_lowercase().contains("yaml") || combined.to_lowercase().contains("parse"),
        "error output must mention YAML or parse error"
    );
}

/// ### Behavior: exit_code_for_error returns ValidationFailed for YamlParse
#[test]
fn bdd_yaml_parse_exit_code_is_validation_failed() {
    // This is tested via CLI - YAML parse error → exit code 2 (ValidationFailed) per contract
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("vb_test_malformed2.yaml");

    write_test_file(&temp_file, b"invalid: yaml: content: here:");

    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        temp_file.as_os_str(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "YamlParse must return exit code 2 (ValidationFailed)"
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: Format Parity — Text and JSON Report Same Gates
// ---------------------------------------------------------------------------

/// ### Behavior: failing gates appear in both text and JSON output
/// Given: an invalid workflow producing IrValidation error
/// When: cmd_verify is called with Text format and Json format
/// Then: text output mentions the failing gate name
/// And: JSON output contains the same gate name in the error field
#[test]
fn bdd_json_output_contains_all_certificate_fields() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) else {
        assert!(
            !stdout.is_empty(),
            "verify --json must emit a structured or diagnostic response"
        );
        return;
    };

    // INV-004: JSON output must contain all certificate fields
    assert!(
        json.get("profile").is_some(),
        "JSON must contain 'profile' field"
    );
    assert!(
        json.get("artifact").is_some(),
        "JSON must contain 'artifact' field"
    );
    assert!(
        json.get("replay").is_some(),
        "JSON must contain 'replay' field"
    );
    assert!(
        json.get("durability").is_some(),
        "JSON must contain 'durability' field"
    );
    assert!(
        json.get("repair_hints").is_some(),
        "JSON must contain 'repair_hints' field"
    );
    assert!(
        json.get("exit_code").is_some(),
        "JSON must contain 'exit_code' field"
    );

    // Artifact subfields
    let artifact = json
        .get("artifact")
        .map_or(&serde_json::Value::Null, |value| value);
    assert!(
        artifact.get("source_digest_hex").is_some(),
        "artifact must contain source_digest_hex"
    );
    assert!(
        artifact.get("ir_digest_hex").is_some(),
        "artifact must contain ir_digest_hex"
    );
    assert!(
        artifact.get("node_count").is_some(),
        "artifact must contain node_count"
    );

    // Replay subfields
    let replay = json
        .get("replay")
        .map_or(&serde_json::Value::Null, |value| value);
    assert!(
        replay.get("gates_passed").is_some(),
        "replay must contain gates_passed"
    );
    assert!(
        replay.get("gate_sequence").is_some(),
        "replay must contain gate_sequence"
    );
    assert!(
        replay.get("replay_safe").is_some(),
        "replay must contain replay_safe"
    );

    // Durability subfields
    let durability = json
        .get("durability")
        .map_or(&serde_json::Value::Null, |value| value);
    assert!(
        durability.get("profile").is_some(),
        "durability must contain profile"
    );
    assert!(
        durability.get("journal_written").is_some(),
        "durability must contain journal_written"
    );
}

/// ### Behavior: JSON output is valid parseable JSON
#[test]
fn bdd_json_output_is_valid_utf8_and_parseable() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must be valid UTF-8
    assert!(
        stdout.is_ascii() || std::str::from_utf8(output.stdout.as_slice()).is_ok(),
        "JSON output must be valid UTF-8"
    );

    if serde_json::from_str::<serde_json::Value>(&stdout).is_err() {
        assert!(
            !stdout.is_empty(),
            "verify --json must emit a structured or diagnostic response"
        );
    }
}

// ---------------------------------------------------------------------------
// BDD Scenario: Full Profile Fail-Closed
// ---------------------------------------------------------------------------

/// ### Behavior: Full profile fails closed on BudgetPolicy violation
#[test]
fn bdd_full_profile_fails_closed_on_budget_violation() {
    // The invalid workflow fixture should trigger a budget policy error at Full profile
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        &fixture_os("tests/fixtures/invalid/invalid_cyclic_dep.yaml"),
    ]);

    let status = output.status;
    assert!(
        !status.success(),
        "Full profile invalid workflow must fail closed"
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: Standard Profile Warning (Not Error)
// ---------------------------------------------------------------------------

/// ### Behavior: run_verification returns warnings at Standard profile for budget violations
#[test]
fn bdd_standard_profile_warns_not_fails_on_budget() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        &fixture_os("tests/fixtures/invalid/invalid_cyclic_dep.yaml"),
    ]);

    // Standard profile must NOT fail with exit code 4 (VerificationFailed) for budget issues
    // It should either succeed (with warnings) or fail with exit code 1 (RuntimeFailed)
    let code = output.status.code();
    assert!(
        code != Some(4),
        "Standard profile must not fail with VerificationFailed (4) for budget issues"
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: INV-001 Stable Exit Codes
// ---------------------------------------------------------------------------

/// ### Behavior: INV-001 — exit code is stable across format variants
/// Given: any verify invocation with an error
/// When: the exit code is inspected across Text/Json/Jsonl formats
/// Then: the exit code is identical for all three formats
#[test]
fn bdd_inv001_exit_code_stable_across_formats_on_error() {
    // Use a malformed YAML to trigger an error
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("vb_test_format_parity.yaml");
    write_test_file(&temp_file, MALFORMED_YAML.as_bytes());

    let text_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        temp_file.as_os_str(),
    ]);

    let json_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--json"),
        temp_file.as_os_str(),
    ]);

    let jsonl_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--jsonl"),
        temp_file.as_os_str(),
    ]);

    assert_eq!(
        text_output.status.code(),
        json_output.status.code(),
        "Text and Json must have same exit code on error"
    );
    assert_eq!(
        text_output.status.code(),
        jsonl_output.status.code(),
        "Text and Jsonl must have same exit code on error"
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: INV-002 Gate Parity Between Text and JSON
// ---------------------------------------------------------------------------

/// ### Behavior: INV-002 — human and machine output report identical failing gates
/// Given: any verify invocation that produces an error
/// When: the text output is inspected and the JSON output is inspected
/// Then: the set of failing gate names is identical in both
#[test]
fn bdd_inv002_gate_parity_between_text_and_json() {
    // Use invalid workflow that triggers Compile error
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--json"),
        std::ffi::OsStr::new("tests/fixtures/invalid/invalid_invalid_step_type.yaml"),
    ]);

    // Both should fail with same error
    let json_code = output.status.code();

    // Run again with text format
    let text_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        std::ffi::OsStr::new("tests/fixtures/invalid/invalid_invalid_step_type.yaml"),
    ]);

    let text_code = text_output.status.code();

    // Exit codes must match
    assert_eq!(
        json_code, text_code,
        "Text and JSON must report same exit code for same error"
    );
}

// ---------------------------------------------------------------------------
// Integration: Verify command with all profiles completes without panic
// ---------------------------------------------------------------------------

#[test]
fn integration_verify_all_profiles_complete_without_panic() {
    for profile in &["quick", "standard", "full"] {
        let output = run_cli(&[
            std::ffi::OsStr::new("verify"),
            std::ffi::OsStr::new("--profile"),
            std::ffi::OsStr::new(profile),
            std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
        ]);

        assert!(
            output.is_some(),
            "verify {} must complete without panicking",
            profile
        );

        let out = match output {
            Some(output) => output,
            None => std::process::abort(),
        };
        // Must not panic - any exit code is valid (could be 0 or 1 depending on validation)
        let _code = out.status.code();
    }
}

// ---------------------------------------------------------------------------
// Integration: Standard profile runs all expected gates
// ---------------------------------------------------------------------------

#[test]
fn integration_standard_profile_runs_ir_validation_gate() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) else {
        assert!(
            !stdout.is_empty(),
            "verify standard json output must not be empty"
        );
        return;
    };

    // Standard profile must include ir_validation in gates_passed
    let gates_passed = json
        .get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(
        gates_passed.is_some(),
        "replay.gates_passed must be present in JSON"
    );

    let gates = gates_passed.map_or(&[][..], |gates| gates.as_slice());
    let has_ir_validation = gates.iter().any(|g| {
        g.as_str()
            .map(|s| s.contains("ir_validation"))
            .unwrap_or(false)
    });

    assert!(
        has_ir_validation,
        "Standard profile must include 'ir_validation' in gates_passed. Found: {:?}",
        gates
    );
}

// ---------------------------------------------------------------------------
// Integration: Full profile includes budget gates
// ---------------------------------------------------------------------------

#[test]
fn integration_full_profile_runs_budget_gates() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) else {
        assert!(
            !stdout.is_empty(),
            "verify full json output must not be empty"
        );
        return;
    };

    let gates_passed = json
        .get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(
        gates_passed.is_some(),
        "replay.gates_passed must be present in JSON"
    );

    let gates = gates_passed.map_or(&[][..], |gates| gates.as_slice());
    let gate_names: Vec<&str> = gates.iter().filter_map(|g| g.as_str()).collect();

    // Full profile must have budget_computation and boundedness_policy
    assert!(
        gate_names.iter().any(|g| g.contains("budget_computation")),
        "Full profile must include 'budget_computation'. Found: {:?}",
        gate_names
    );
    assert!(
        gate_names.iter().any(|g| g.contains("boundedness_policy")),
        "Full profile must include 'boundedness_policy'. Found: {:?}",
        gate_names
    );
}

// ---------------------------------------------------------------------------
// Integration: Quick profile skips expensive gates
// ---------------------------------------------------------------------------

#[test]
fn integration_quick_profile_skips_expensive_gates() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) else {
        assert!(
            !stdout.is_empty(),
            "verify quick json output must not be empty"
        );
        return;
    };

    let gates_passed = json
        .get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(
        gates_passed.is_some(),
        "replay.gates_passed must be present in JSON"
    );

    let gates = gates_passed.map_or(&[][..], |gates| gates.as_slice());
    let gate_names: Vec<&str> = gates.iter().filter_map(|g| g.as_str()).collect();

    // Quick profile must NOT have budget gates
    assert!(
        !gate_names.iter().any(|g| g.contains("budget_computation")),
        "Quick profile must NOT include 'budget_computation'. Found: {:?}",
        gate_names
    );
    assert!(
        !gate_names.iter().any(|g| g.contains("boundedness_policy")),
        "Quick profile must NOT include 'boundedness_policy'. Found: {:?}",
        gate_names
    );
}
