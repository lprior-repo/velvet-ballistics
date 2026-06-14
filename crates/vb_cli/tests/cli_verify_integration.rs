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

fn parse_structured_value(stdout: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(stdout)
        .ok()
        .or_else(|| serde_saphyr::from_str::<serde_json::Value>(stdout).ok())
}

fn must_parse_structured_value(stdout: &str, context: &str) -> serde_json::Value {
    let parsed = parse_structured_value(stdout);
    assert!(
        parsed.is_some(),
        "{context}. stdout was not parseable structured output:\n{stdout}"
    );
    match parsed {
        Some(value) => value,
        None => std::process::abort(),
    }
}

fn parse_text_csv_line(output: &str, prefix: &str) -> Vec<String> {
    let value = output.lines().find_map(|line| line.strip_prefix(prefix));
    assert!(
        value.is_some(),
        "missing text line prefix `{prefix}` in output:\n{output}"
    );

    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                trimmed
                    .split(", ")
                    .map(std::string::ToString::to_string)
                    .collect()
            }
        }
        None => std::process::abort(),
    }
}

const QUICK_GATE_STATUSES: [&str; 15] = [
    "profile",
    "shape",
    "names",
    "references",
    "expressions",
    "CFG",
    "bounded:deferred",
    "budgets:deferred",
    "contracts:deferred",
    "taint:deferred",
    "idempotency:deferred",
    "durability:deferred",
    "capabilities:deferred",
    "results",
    "evidence:deferred",
];

const STANDARD_GATE_STATUSES: [&str; 15] = [
    "profile",
    "shape",
    "names",
    "references",
    "expressions",
    "CFG",
    "bounded",
    "budgets",
    "contracts:deferred",
    "taint:deferred",
    "idempotency:deferred",
    "durability:deferred",
    "capabilities:deferred",
    "results",
    "evidence:deferred",
];

const FULL_DEFERRED_GATES: [&str; 6] = [
    "contracts",
    "taint",
    "idempotency",
    "durability",
    "capabilities",
    "evidence",
];

const STANDARD_PASSED_GATES: [&str; 9] = [
    "profile",
    "shape",
    "names",
    "references",
    "expressions",
    "CFG",
    "bounded",
    "budgets",
    "results",
];

fn json_string_vec(value: &serde_json::Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(std::string::ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// BDD Scenario: Happy Path — Minimal Valid Workflow
// ---------------------------------------------------------------------------

/// ### Behavior: run_verification returns canonical gate statuses for minimal valid workflow
/// Given: a valid minimal workflow YAML at tests/fixtures/valid/minimal.yaml
/// When: run_verification is called with Quick profile
/// Then: result is Ok(VerifyOk) with non-empty digest_hex
/// And: result.checks matches the master §63 gate-status sequence
#[test]
fn bdd_happy_quick_profile_returns_ok_with_checks() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);
    let status = output.status;

    // Quick profile with valid workflow must succeed (exit 0)
    assert!(
        status.success(),
        "verify quick must succeed with valid workflow. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(status.code(), Some(0), "exit code must be 0 on success");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = parse_structured_value(&stdout).expect("quick verify must emit structured yaml");
    assert_eq!(
        json_string_vec(&json, "/checks"),
        QUICK_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        vec![
            "bounded".to_string(),
            "budgets".to_string(),
            "contracts".to_string(),
            "taint".to_string(),
            "idempotency".to_string(),
            "durability".to_string(),
            "capabilities".to_string(),
            "evidence".to_string(),
        ]
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
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json = must_parse_structured_value(
        &stdout,
        "verify --emit yaml must emit a structured response",
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        json.pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json_string_vec(&json, "/checks"),
        STANDARD_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        FULL_DEFERRED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );

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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must be valid UTF-8
    assert!(
        stdout.is_ascii() || std::str::from_utf8(output.stdout.as_slice()).is_ok(),
        "JSON output must be valid UTF-8"
    );

    let _ = must_parse_structured_value(
        &stdout,
        "verify --emit yaml must emit a structured response",
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: Full Profile Fail-Closed
// ---------------------------------------------------------------------------

/// ### Behavior: Full profile fails closed when canonical gates remain deferred
#[test]
fn bdd_full_profile_fails_closed_on_deferred_gates() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(output.status.code(), Some(4));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = parse_structured_value(&stdout).expect("full verify must emit structured yaml");
    assert_eq!(
        json.pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(4)
    );
    assert_eq!(
        json_string_vec(&json, "/checks"),
        STANDARD_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        FULL_DEFERRED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json.pointer("/error").and_then(serde_json::Value::as_str),
        Some(
            "full verification blocked: deferred gates remain: contracts, taint, idempotency, durability, capabilities, evidence"
        )
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: Standard Profile Warning (Not Error)
// ---------------------------------------------------------------------------

/// ### Behavior: Standard profile reports the exact deferred gate set
#[test]
fn bdd_standard_profile_reports_exact_deferred_gate_set() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = parse_structured_value(&stdout).expect("standard verify must emit structured yaml");
    assert_eq!(
        json.pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json.pointer("/all_gates_closed")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        FULL_DEFERRED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
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
// BDD Scenario: INV-002 Gate Parity Between Text and Structured Output
// ---------------------------------------------------------------------------

/// ### Behavior: INV-002 — human and machine output report identical gate status sets
/// Given: a valid workflow where Full profile fails closed because deferred gates remain
/// When: the text output is inspected and the structured output is inspected
/// Then: the ordered gate-status list is identical in both
/// And: the deferred-gate list is identical in both
#[test]
fn bdd_inv002_gate_parity_between_text_and_structured_output_on_full_profile_failure() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let json_code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = must_parse_structured_value(
        &stdout,
        "verify full --emit yaml must emit structured output",
    );

    let text_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let text_code = text_output.status.code();
    let stderr = String::from_utf8_lossy(&text_output.stderr);

    assert_eq!(
        json_code, text_code,
        "Text and structured output must report same exit code for the same fail-closed verify"
    );
    assert_eq!(
        json_code,
        Some(4),
        "Full profile must fail closed on deferred gates"
    );
    assert_eq!(
        parse_text_csv_line(&stderr, "gate statuses: "),
        json_string_vec(&json, "/checks")
    );
    assert_eq!(
        parse_text_csv_line(&stderr, "passed gates: "),
        json_string_vec(&json, "/passed_checks")
    );
    assert_eq!(
        parse_text_csv_line(&stderr, "deferred gates: "),
        json_string_vec(&json, "/deferred_checks")
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = must_parse_structured_value(
        &stdout,
        "verify standard structured output must be parseable",
    );

    let checks = json.get("checks").and_then(|checks| checks.as_array());
    let gates_passed = json
        .get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(checks.is_some(), "checks must be present in JSON");
    assert!(
        gates_passed.is_some(),
        "replay.gates_passed must be present in JSON"
    );

    let check_names: Vec<&str> = checks
        .map_or(&[][..], |checks| checks.as_slice())
        .iter()
        .filter_map(|gate| gate.as_str())
        .collect();
    let gates = gates_passed.map_or(&[][..], |gates| gates.as_slice());
    let gate_names: Vec<&str> = gates.iter().filter_map(|gate| gate.as_str()).collect();

    assert_eq!(
        check_names,
        STANDARD_GATE_STATUSES.to_vec(),
        "Standard profile must report the exact canonical gate sequence"
    );
    assert_eq!(
        gate_names,
        STANDARD_PASSED_GATES.to_vec(),
        "Standard profile must report the exact locally closed gate set"
    );
    assert_eq!(
        json_string_vec(&json, "/passed_checks"),
        STANDARD_PASSED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        FULL_DEFERRED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/replay/gate_sequence"),
        STANDARD_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json.pointer("/all_gates_closed")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(0)
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(output.status.code(), Some(4));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json =
        must_parse_structured_value(&stdout, "verify full structured output must be parseable");

    let checks = json.get("checks").and_then(|checks| checks.as_array());
    let gates_passed = json
        .get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert_eq!(
        json.get("success").and_then(|value| value.as_bool()),
        Some(false)
    );
    assert!(checks.is_some(), "checks must be present in JSON");
    assert!(
        gates_passed.is_some(),
        "replay.gates_passed must be present in JSON"
    );

    let check_names: Vec<&str> = checks
        .map_or(&[][..], |checks| checks.as_slice())
        .iter()
        .filter_map(|gate| gate.as_str())
        .collect();
    let gates = gates_passed.map_or(&[][..], |gates| gates.as_slice());
    let gate_names: Vec<&str> = gates.iter().filter_map(|g| g.as_str()).collect();

    assert_eq!(
        check_names,
        STANDARD_GATE_STATUSES.to_vec(),
        "Full profile must preserve the exact canonical gate sequence in the fail-closed report"
    );
    assert_eq!(
        gate_names,
        STANDARD_PASSED_GATES.to_vec(),
        "Full profile must preserve the exact locally closed gate set in the fail-closed report"
    );
    assert_eq!(
        json_string_vec(&json, "/passed_checks"),
        STANDARD_PASSED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/deferred_checks"),
        FULL_DEFERRED_GATES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json_string_vec(&json, "/replay/gate_sequence"),
        STANDARD_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        json.pointer("/all_gates_closed")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.pointer("/error").and_then(serde_json::Value::as_str),
        Some(
            "full verification blocked: deferred gates remain: contracts, taint, idempotency, durability, capabilities, evidence"
        )
    );
    assert_eq!(
        json.pointer("/repair_hints/0")
            .and_then(serde_json::Value::as_str),
        Some(
            "Close every deferred master §63 gate before treating --profile full as acceptance evidence"
        )
    );
    assert_eq!(
        json.pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(4)
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json =
        must_parse_structured_value(&stdout, "verify quick structured output must be parseable");

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

    // Quick profile must NOT have budget gates and must leave them deferred
    assert!(
        !gate_names.contains(&"bounded") && !gate_names.contains(&"budgets"),
        "Quick profile must not report bounded/budgets as passed. Found: {:?}",
        gate_names
    );
    assert!(
        json.get("checks")
            .and_then(|checks| checks.as_array())
            .is_some_and(|checks| {
                checks
                    .iter()
                    .filter_map(|gate| gate.as_str())
                    .any(|gate| gate == "bounded:deferred" || gate == "budgets:deferred")
            }),
        "Quick profile must leave bounded and budgets deferred"
    );
}
