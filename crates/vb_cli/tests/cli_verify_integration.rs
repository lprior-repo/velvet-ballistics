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

fn write_test_file(path: &std::path::Path, contents: &[u8]) {
    if let Err(error) = std::fs::write(path, contents) {
        panic!("failed to write {}: {error}", path.display());
    }
}

fn must_create_temp_yaml_file(prefix: &str) -> tempfile::NamedTempFile {
    match tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".yaml")
        .tempfile()
    {
        Ok(file) => file,
        Err(error) => panic!("failed to create temp file with prefix {prefix}: {error}"),
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

fn parse_yaml_value(stdout: &str) -> Result<serde_json::Value, String> {
    serde_saphyr::from_str::<serde_json::Value>(stdout).map_err(|error| error.to_string())
}

fn must_parse_yaml_value(stdout: &str, context: &str) -> serde_json::Value {
    let parsed = parse_yaml_value(stdout);
    assert!(
        parsed.is_ok(),
        "{context}. stdout was not parseable YAML-compatible structured output:\n{stdout}"
    );
    match parsed {
        Ok(value) => value,
        Err(_) => std::process::abort(),
    }
}

fn must_parse_json_value(bytes: &[u8], context: &str) -> serde_json::Value {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) => value,
        Err(error) => panic!(
            "{context}: {error}; bytes={}",
            String::from_utf8_lossy(bytes)
        ),
    }
}

fn must_parse_jsonl_value(bytes: &[u8], context: &str) -> serde_json::Value {
    let text = String::from_utf8_lossy(bytes);
    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    assert!(
        bytes.last().copied() == Some(b'\n'),
        "{context}: JSONL record must end with a single newline, got {text:?}"
    );
    assert_eq!(
        newline_count, 1,
        "{context}: expected exactly one newline-delimited JSON record, got {text:?}"
    );
    let trimmed = match text.strip_suffix('\n') {
        Some(value) => value,
        None => panic!("{context}: JSONL record lost its trailing newline: {text:?}"),
    };
    assert!(
        !trimmed.is_empty(),
        "{context}: JSONL payload must contain one JSON value, got {text:?}"
    );
    assert!(
        !trimmed.contains('\n'),
        "{context}: expected exactly one JSON line, got {text:?}"
    );
    assert!(
        !trimmed.contains('\r'),
        "{context}: JSONL record must use LF framing only, got {text:?}"
    );
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}; line={trimmed}"),
    }
}

fn assert_empty_stream(bytes: &[u8], context: &str) {
    assert!(
        bytes.is_empty(),
        "{context}: expected empty stream, got {:?}",
        String::from_utf8_lossy(bytes)
    );
}

fn json_string(value: &serde_json::Value, pointer: &str, context: &str) -> String {
    match value.pointer(pointer).and_then(serde_json::Value::as_str) {
        Some(text) => text.to_string(),
        None => panic!("{context}: expected string at {pointer}, got {value}"),
    }
}

fn assert_machine_diagnostic(
    value: &serde_json::Value,
    expected_code: &str,
    expected_exit_code: u64,
    message_prefix: &str,
    context: &str,
) -> String {
    assert_eq!(
        value
            .pointer("/schema_version")
            .and_then(serde_json::Value::as_str),
        Some("velvet-ballistics/cli-output/v1"),
        "{context}: schema_version"
    );
    assert_eq!(
        value.pointer("/kind").and_then(serde_json::Value::as_str),
        Some("DiagnosticReport"),
        "{context}: kind"
    );
    assert_eq!(
        value.pointer("/code").and_then(serde_json::Value::as_str),
        Some(expected_code),
        "{context}: code"
    );
    assert_eq!(
        value
            .pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(expected_exit_code),
        "{context}: exit_code"
    );
    assert!(
        value.get("success").is_none(),
        "{context}: diagnostic payload must not expose success field"
    );
    assert!(
        value.get("profile").is_none(),
        "{context}: diagnostic payload must not expose profile field"
    );
    assert!(
        value.get("error").is_none(),
        "{context}: diagnostic payload must not expose legacy error field"
    );

    let message = json_string(value, "/message", context);
    assert!(
        message.starts_with(message_prefix),
        "{context}: message `{message}` did not start with `{message_prefix}`"
    );
    message
}

fn assert_lower_hex_string(value: &str, expected_len: usize, context: &str) {
    assert_eq!(
        value.len(),
        expected_len,
        "{context}: expected {expected_len} hex characters, got {value}"
    );
    assert!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{context}: expected lowercase hex, got {value}"
    );
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

const QUICK_DEFERRED_GATES: [&str; 8] = [
    "bounded",
    "budgets",
    "contracts",
    "taint",
    "idempotency",
    "durability",
    "capabilities",
    "evidence",
];

const QUICK_PASSED_GATES: [&str; 7] = [
    "profile",
    "shape",
    "names",
    "references",
    "expressions",
    "CFG",
    "results",
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

const TAINT_WARNING: &str = "taint warning: compiled-form WorkflowParts taint validation is not implemented; AST validation alone does not close this gate";

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

fn csv_line(values: &[&str]) -> String {
    values.join(", ")
}

fn expected_default_text_success(
    profile: &str,
    statuses: &[&str],
    passed: &[&str],
    deferred: &[&str],
    warnings: &[&str],
) -> String {
    let mut lines = vec![
        format!("verified (2 nodes, profile={profile})"),
        format!("gate statuses: {}", csv_line(statuses)),
        format!("passed gates: {}", csv_line(passed)),
        format!("deferred gates: {}", csv_line(deferred)),
    ];
    if !warnings.is_empty() {
        lines.push(format!("warnings: {}", warnings.join(" | ")));
    }
    lines.push(format!(
        "Deferred gates remain: {}. This report does not close all master §63 gates.",
        csv_line(deferred)
    ));
    lines.join("\n") + "\n"
}

fn expected_default_text_failure(
    statuses: &[&str],
    passed: &[&str],
    deferred: &[&str],
    warnings: &[&str],
) -> String {
    let mut lines = vec![
        format!(
            "full verification blocked: deferred gates remain: {}",
            csv_line(deferred)
        ),
        format!("gate statuses: {}", csv_line(statuses)),
        format!("passed gates: {}", csv_line(passed)),
        format!("deferred gates: {}", csv_line(deferred)),
    ];
    if !warnings.is_empty() {
        lines.push(format!("warnings: {}", warnings.join(" | ")));
    }
    lines.join("\n") + "\n"
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
    let json = must_parse_yaml_value(
        &stdout,
        "quick verify must emit YAML-compatible structured output",
    );
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
    let yaml_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

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
        yaml_output.status.code(),
        Some(0),
        "YAML report must succeed"
    );
    assert_eq!(
        text_code, json_code,
        "Text and Json formats must produce same exit code"
    );
    assert_eq!(
        text_code, jsonl_code,
        "Text and Jsonl formats must produce same exit code"
    );
    assert_eq!(text_code, Some(0), "Valid workflow must exit with code 0");

    assert_empty_stream(&yaml_output.stderr, "verify --emit yaml success stderr");
    assert_empty_stream(&json_output.stderr, "verify --json success stderr");
    assert_empty_stream(&jsonl_output.stderr, "verify --jsonl success stderr");

    let yaml_stdout = String::from_utf8_lossy(&yaml_output.stdout);
    let canonical_report = must_parse_yaml_value(
        &yaml_stdout,
        "verify --emit yaml must emit canonical structured report",
    );

    let json_value = must_parse_json_value(
        &json_output.stdout,
        "verify --json must emit parseable machine JSON",
    );
    let jsonl_value = must_parse_jsonl_value(
        &jsonl_output.stdout,
        "verify --jsonl must emit exactly one machine JSON line",
    );
    assert_eq!(json_value, canonical_report);
    assert_eq!(jsonl_value, canonical_report);
    assert_eq!(
        json_value
            .pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
}

#[test]
fn integration_verify_emit_text_success_matches_human_contract() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert_empty_stream(&output.stderr, "verify --emit text quick success stderr");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected_default_text_success(
            "quick",
            &QUICK_GATE_STATUSES,
            &QUICK_PASSED_GATES,
            &QUICK_DEFERRED_GATES,
            &[],
        )
    );
}

#[test]
fn integration_verify_legacy_json_flags_match_canonical_fail_closed_report() {
    let yaml_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);
    let json_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);
    let jsonl_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--jsonl"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(yaml_output.status.code(), Some(4));
    assert_eq!(json_output.status.code(), Some(4));
    assert_eq!(jsonl_output.status.code(), Some(4));
    assert_empty_stream(&yaml_output.stderr, "verify --emit yaml fail-closed stderr");
    assert_empty_stream(&json_output.stderr, "verify --json fail-closed stderr");
    assert_empty_stream(&jsonl_output.stderr, "verify --jsonl fail-closed stderr");

    let yaml_stdout = String::from_utf8_lossy(&yaml_output.stdout);
    let canonical_report = must_parse_yaml_value(
        &yaml_stdout,
        "verify --emit yaml full-profile fail-closed output must be parseable",
    );
    let json_report = must_parse_json_value(
        &json_output.stdout,
        "verify --json full-profile fail-closed output must be parseable",
    );
    let jsonl_report = must_parse_jsonl_value(
        &jsonl_output.stdout,
        "verify --jsonl full-profile fail-closed output must be exactly one JSON line",
    );

    assert_eq!(json_report, canonical_report);
    assert_eq!(jsonl_report, canonical_report);
    assert_eq!(
        json_report
            .pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json_report
            .pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(4)
    );
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
    let temp_file = must_create_temp_yaml_file("vb-test-malformed-");
    write_test_file(temp_file.path(), MALFORMED_YAML.as_bytes());

    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        temp_file.path().as_os_str(),
    ]);

    let status = output.status;
    // YAML parse error must result in exit code 2 (ValidationFailed) per contract POST-008
    assert_eq!(
        status.code(),
        Some(2),
        "YAML parse error must exit with code 2 (ValidationFailed)"
    );

    assert_empty_stream(&output.stdout, "verify malformed default-text stdout");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim_end().starts_with("YAML parse error: "),
        "stderr must carry the exact YAML parse classification, got: {stderr}"
    );
}

/// ### Behavior: exit_code_for_error returns ValidationFailed for YamlParse
#[test]
fn bdd_yaml_parse_exit_code_is_validation_failed() {
    // This is tested via CLI - YAML parse error → exit code 2 (ValidationFailed) per contract
    let temp_file = must_create_temp_yaml_file("vb-test-malformed2-");

    write_test_file(temp_file.path(), b"invalid: yaml: content: here:");

    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        temp_file.path().as_os_str(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "YamlParse must return exit code 2 (ValidationFailed)"
    );
}

#[test]
fn integration_verify_emit_text_parse_error_uses_text_diagnostic_contract() {
    let temp_file = must_create_temp_yaml_file("vb-test-emit-text-parse-");
    write_test_file(temp_file.path(), MALFORMED_YAML.as_bytes());

    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        temp_file.path().as_os_str(),
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert_empty_stream(&output.stdout, "verify --emit text malformed-yaml stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim_end().starts_with("YAML parse error: "),
        "verify --emit text malformed-yaml stderr must carry the YAML parse classification, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// BDD Scenario: Structured Report Completeness — Standard JSON Report
// ---------------------------------------------------------------------------

/// ### Behavior: standard-profile JSON output carries the full verification report schema
/// Given: a valid workflow producing a standard-profile verification report
/// When: cmd_verify is called with Json format
/// Then: the structured output contains the expected certificate and replay fields
#[test]
fn bdd_json_output_contains_all_certificate_fields() {
    let output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert_empty_stream(&output.stderr, "verify --json standard stderr");

    let json = must_parse_json_value(
        &output.stdout,
        "verify --json standard must emit a structured response",
    );

    assert_eq!(
        json.pointer("/schema_version")
            .and_then(serde_json::Value::as_str),
        Some("velvet-ballistics/cli-output/v1")
    );
    assert_eq!(
        json.pointer("/kind").and_then(serde_json::Value::as_str),
        Some("verify_report")
    );
    assert_eq!(
        json.pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json.pointer("/profile").and_then(serde_json::Value::as_str),
        Some("standard")
    );
    assert_eq!(
        json.pointer("/node_count")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert_eq!(
        json_string_vec(&json, "/checks"),
        STANDARD_GATE_STATUSES
            .iter()
            .map(|gate| gate.to_string())
            .collect::<Vec<String>>()
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

    let digest = json_string(&json, "/digest", "verify report digest");
    assert_lower_hex_string(&digest, 64, "verify report digest");
    assert_eq!(
        json.pointer("/all_gates_closed")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json_string_vec(&json, "/warnings"),
        vec![TAINT_WARNING.to_string()]
    );
    assert_eq!(
        json.pointer("/repair_hints")
            .and_then(serde_json::Value::as_array)
            .map(std::vec::Vec::len),
        Some(0)
    );
    assert_eq!(
        json.pointer("/exit_code")
            .and_then(serde_json::Value::as_u64),
        Some(0)
    );
    assert!(
        json.get("error").is_none(),
        "successful verify report must not contain an error field"
    );

    let artifact_source_digest = json_string(
        &json,
        "/artifact/source_digest_hex",
        "verify report artifact.source_digest_hex",
    );
    assert_eq!(artifact_source_digest, digest);

    let artifact_ir_digest = json_string(
        &json,
        "/artifact/ir_digest_hex",
        "verify report artifact.ir_digest_hex",
    );
    assert_lower_hex_string(
        &artifact_ir_digest,
        64,
        "verify report artifact.ir_digest_hex",
    );
    assert_eq!(
        json.pointer("/artifact/node_count")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );

    assert_eq!(
        json_string_vec(&json, "/replay/gates_passed"),
        STANDARD_PASSED_GATES
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
        json.pointer("/replay/replay_safe")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );

    assert_eq!(
        json.pointer("/durability/profile")
            .and_then(serde_json::Value::as_str),
        Some("none")
    );
    assert_eq!(
        json.pointer("/durability/journal_written")
            .and_then(serde_json::Value::as_bool),
        Some(false)
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

    let _ = must_parse_yaml_value(
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
    let json = must_parse_yaml_value(
        &stdout,
        "full verify must emit YAML-compatible structured output",
    );
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
    let json = must_parse_yaml_value(
        &stdout,
        "standard verify must emit YAML-compatible structured output",
    );
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
    let temp_file = must_create_temp_yaml_file("vb-test-format-parity-");
    write_test_file(temp_file.path(), MALFORMED_YAML.as_bytes());

    let text_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("text"),
        temp_file.path().as_os_str(),
    ]);

    let json_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--json"),
        temp_file.path().as_os_str(),
    ]);

    let jsonl_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--jsonl"),
        temp_file.path().as_os_str(),
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
    assert_empty_stream(&json_output.stdout, "verify --json malformed-yaml stdout");
    assert_empty_stream(&jsonl_output.stdout, "verify --jsonl malformed-yaml stdout");

    let json_error = must_parse_json_value(
        &json_output.stderr,
        "verify --json error path must emit parseable JSON on stderr",
    );
    let jsonl_error = must_parse_jsonl_value(
        &jsonl_output.stderr,
        "verify --jsonl error path must emit exactly one JSON line on stderr",
    );
    assert_eq!(json_error, jsonl_error);
    let message = assert_machine_diagnostic(
        &json_error,
        "ValidationFailed",
        2,
        "YAML parse error: ",
        "verify malformed-yaml legacy JSON diagnostic",
    );
    assert_eq!(
        json_error,
        serde_json::json!({
            "schema_version": "velvet-ballistics/cli-output/v1",
            "kind": "DiagnosticReport",
            "code": "ValidationFailed",
            "exit_code": 2,
            "message": message,
        })
    );
}

#[test]
fn integration_verify_legacy_json_flags_emit_machine_diagnostics_on_unknown_profile() {
    let json_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("thorough"),
        std::ffi::OsStr::new("--json"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);
    let jsonl_output = must_run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("thorough"),
        std::ffi::OsStr::new("--jsonl"),
        &fixture_os("tests/fixtures/valid/minimal.yaml"),
    ]);

    assert_eq!(json_output.status.code(), Some(2));
    assert_eq!(jsonl_output.status.code(), Some(2));
    assert_empty_stream(&json_output.stdout, "verify --json parse-diagnostic stdout");
    assert_empty_stream(
        &jsonl_output.stdout,
        "verify --jsonl parse-diagnostic stdout",
    );

    let json_error = must_parse_json_value(
        &json_output.stderr,
        "verify --json parse error must emit parseable JSON on stderr",
    );
    let jsonl_error = must_parse_jsonl_value(
        &jsonl_output.stderr,
        "verify --jsonl parse error must emit exactly one JSON line on stderr",
    );
    let message = assert_machine_diagnostic(
        &json_error,
        "ValidationFailed",
        2,
        "unknown verify profile: thorough (expected: quick, standard, full)",
        "verify unknown-profile legacy JSON diagnostic",
    );
    let expected = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "DiagnosticReport",
        "code": "ValidationFailed",
        "exit_code": 2,
        "message": message,
    });
    assert_eq!(json_error, expected);
    assert_eq!(jsonl_error, expected);
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
    let json = must_parse_yaml_value(
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
fn integration_verify_all_profiles_match_default_text_contract() {
    for (profile, expected_exit_code) in
        [("quick", Some(0)), ("standard", Some(0)), ("full", Some(4))]
    {
        let output = must_run_cli(&[
            std::ffi::OsStr::new("verify"),
            std::ffi::OsStr::new("--profile"),
            std::ffi::OsStr::new(profile),
            &fixture_os("tests/fixtures/valid/minimal.yaml"),
        ]);

        assert_eq!(
            output.status.code(),
            expected_exit_code,
            "verify {profile} default text mode must use the expected exit code"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if expected_exit_code == Some(0) {
            assert_empty_stream(
                &output.stderr,
                &format!("verify {profile} default text success stderr"),
            );
            let expected_stdout = match profile {
                "quick" => expected_default_text_success(
                    profile,
                    &QUICK_GATE_STATUSES,
                    &QUICK_PASSED_GATES,
                    &QUICK_DEFERRED_GATES,
                    &[],
                ),
                "standard" => expected_default_text_success(
                    profile,
                    &STANDARD_GATE_STATUSES,
                    &STANDARD_PASSED_GATES,
                    &FULL_DEFERRED_GATES,
                    &[TAINT_WARNING],
                ),
                _ => String::new(),
            };
            assert_eq!(
                stdout, expected_stdout,
                "verify {profile} default text success output must match the exact summary contract"
            );
        } else {
            assert_empty_stream(
                &output.stdout,
                &format!("verify {profile} default text deferred stdout"),
            );
            assert_eq!(
                stderr,
                expected_default_text_failure(
                    &STANDARD_GATE_STATUSES,
                    &STANDARD_PASSED_GATES,
                    &FULL_DEFERRED_GATES,
                    &[TAINT_WARNING],
                ),
                "verify {profile} default text deferred output must match the exact blocked-gate contract"
            );
        }
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
    let json = must_parse_yaml_value(
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
    let json = must_parse_yaml_value(&stdout, "verify full structured output must be parseable");

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
    let json = must_parse_yaml_value(&stdout, "verify quick structured output must be parseable");

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
