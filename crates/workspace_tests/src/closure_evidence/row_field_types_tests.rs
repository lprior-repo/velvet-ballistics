//! Unit tests for closure evidence row field type validation.
//!
//! Validates the fix for vb-eirmk.74.3.1.1: non-string required fields
//! must produce field-specific type errors instead of passing silently.
//!
//! Uses a genuine persistent fixture with a real raw log containing the
//! required provenance bindings (command, cwd, timestamp, exit_code).
//! Fixture path includes process ID to avoid races between test runs.
//! Uses OnceLock to ensure the write completes before any test reads it.

use crate::closure_evidence::row_validation::validate_row_shape;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Workspace root for validation calls.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .to_path_buf()
}

/// Genuine persistent fixture path including process ID to avoid
/// races between concurrent test runs and interference from prior runs.
fn fixture_path() -> PathBuf {
    let pid = std::process::id();
    fixture_root().join(format!(".evidence/field-types-r1/fixtures/raw-{}.log", pid))
}

/// Ensures the genuine persistent fixture exists with correct provenance
/// bindings. Uses OnceLock to write exactly once per test binary process,
/// and waits for the write to complete before any test proceeds.
static FIXTURE_READY: OnceLock<()> = OnceLock::new();

fn ensure_fixture() {
    FIXTURE_READY.get_or_init(|| {
        let path = fixture_path();
        let dir = path.parent().expect("parent directory");
        fs::create_dir_all(dir).expect("create fixture directory");
        fs::write(
            &path,
            "command: cargo test\n\
             cwd: .\n\
             timestamp: 2026-01-01T00:00:00Z\n\
             exit_code: 0\n\
             stdout: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; finished in 0.01s\n",
        )
        .expect("write fixture raw log");
        // OnceLock guarantees this completes before returning to caller.
    });
}

fn fixture_raw_log_string() -> String {
    fixture_path().to_string_lossy().to_string()
}

fn make_row(fields: &[(&str, serde_json::Value)]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (key, value) in fields {
        obj.insert(key.to_string(), value.clone());
    }
    serde_json::Value::Object(obj)
}

fn baseline_row_fields() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("bead_id", serde_json::json!("vb-test-1")),
        ("command", serde_json::json!("cargo test")),
        ("cwd", serde_json::json!(".")),
        ("commit_sha", serde_json::json!("abc123def456")),
        ("tool_version", serde_json::json!("cargo 1.80")),
        ("timestamp", serde_json::json!("2026-01-01T00:00:00Z")),
        ("raw_log_path", serde_json::json!(fixture_raw_log_string())),
        ("stdout_summary", serde_json::json!("ok")),
        ("stderr_summary", serde_json::json!("empty")),
        ("exit_code", serde_json::json!(0)),
        ("status", serde_json::json!("PASS")),
        ("evidence_kind", serde_json::json!("raw-command")),
    ]
}

fn set_field(
    fields: &mut Vec<(&'static str, serde_json::Value)>,
    name: &'static str,
    value: serde_json::Value,
) {
    let entry = fields
        .iter_mut()
        .find(|(k, _)| *k == name)
        .expect("field not found");
    entry.1 = value;
}

#[test]
fn baseline_row_shape_is_valid() {
    ensure_fixture();
    let fields = baseline_row_fields();
    let row = make_row(&fields);
    let obj = row.as_object().expect("object");
    let failures = validate_row_shape(&fixture_root(), 1, obj);
    assert!(
        failures.is_empty(),
        "baseline row must have zero failures: {:?}",
        failures
    );
}

// All eleven required STRING fields from the schema (exit_code is integer, not string).
const REQUIRED_STRING_FIELDS: &[&str] = &[
    "bead_id",
    "command",
    "cwd",
    "commit_sha",
    "tool_version",
    "timestamp",
    "raw_log_path",
    "stdout_summary",
    "stderr_summary",
    "status",
    "evidence_kind",
];

#[test]
fn all_required_string_fields_reject_number_zero() {
    ensure_fixture();
    for field in REQUIRED_STRING_FIELDS {
        let mut fields = baseline_row_fields();
        set_field(&mut fields, field, serde_json::json!(0));
        let row = make_row(&fields);
        let obj = row.as_object().expect("object");
        let failures = validate_row_shape(&fixture_root(), 1, obj);
        assert!(
            failures
                .iter()
                .any(|f| f.contains(field) && f.contains("must be string")),
            "field {} with number 0 should produce field-specific type error, got: {:?}",
            field,
            failures
        );
    }
}

#[test]
fn all_required_string_fields_reject_boolean_false() {
    ensure_fixture();
    for field in REQUIRED_STRING_FIELDS {
        let mut fields = baseline_row_fields();
        set_field(&mut fields, field, serde_json::json!(false));
        let row = make_row(&fields);
        let obj = row.as_object().expect("object");
        let failures = validate_row_shape(&fixture_root(), 1, obj);
        assert!(
            failures
                .iter()
                .any(|f| f.contains(field) && f.contains("must be string")),
            "field {} with boolean false should produce field-specific type error, got: {:?}",
            field,
            failures
        );
    }
}

#[test]
fn all_required_string_fields_reject_empty_array() {
    ensure_fixture();
    for field in REQUIRED_STRING_FIELDS {
        let mut fields = baseline_row_fields();
        set_field(&mut fields, field, serde_json::json!([]));
        let row = make_row(&fields);
        let obj = row.as_object().expect("object");
        let failures = validate_row_shape(&fixture_root(), 1, obj);
        assert!(
            failures
                .iter()
                .any(|f| f.contains(field) && f.contains("must be string")),
            "field {} with empty array should produce field-specific type error, got: {:?}",
            field,
            failures
        );
    }
}

#[test]
fn all_required_string_fields_reject_empty_object() {
    ensure_fixture();
    for field in REQUIRED_STRING_FIELDS {
        let mut fields = baseline_row_fields();
        set_field(&mut fields, field, serde_json::json!({}));
        let row = make_row(&fields);
        let obj = row.as_object().expect("object");
        let failures = validate_row_shape(&fixture_root(), 1, obj);
        assert!(
            failures
                .iter()
                .any(|f| f.contains(field) && f.contains("must be string")),
            "field {} with empty object should produce field-specific type error, got: {:?}",
            field,
            failures
        );
    }
}

#[test]
fn missing_required_field_is_still_detected() {
    ensure_fixture();
    let fields = baseline_row_fields();
    let mut obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for (k, v) in fields {
        if k != "command" {
            obj.insert(k.to_string(), v.clone());
        }
    }
    let failures = validate_row_shape(&fixture_root(), 1, &obj);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("command") && f.contains("missing required field")),
        "missing command should still be detected: {:?}",
        failures
    );
}

#[test]
fn null_required_field_is_missing() {
    ensure_fixture();
    let mut fields = baseline_row_fields();
    set_field(&mut fields, "tool_version", serde_json::json!(null));
    let row = make_row(&fields);
    let obj = row.as_object().expect("object");
    let failures = validate_row_shape(&fixture_root(), 1, obj);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("tool_version") && f.contains("missing required field")),
        "null tool_version should be reported as missing: {:?}",
        failures
    );
}

#[test]
fn whitespace_only_required_field_is_missing() {
    ensure_fixture();
    let mut fields = baseline_row_fields();
    set_field(&mut fields, "timestamp", serde_json::json!("   "));
    let row = make_row(&fields);
    let obj = row.as_object().expect("object");
    let failures = validate_row_shape(&fixture_root(), 1, obj);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("timestamp") && f.contains("missing required field")),
        "whitespace timestamp should be reported as missing: {:?}",
        failures
    );
}

#[test]
fn exit_code_non_integer_handled_by_dedicated_check() {
    ensure_fixture();
    let mut fields = baseline_row_fields();
    set_field(&mut fields, "exit_code", serde_json::json!("0"));
    let row = make_row(&fields);
    let obj = row.as_object().expect("object");
    let failures = validate_row_shape(&fixture_root(), 1, obj);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("exit_code must be integer")),
        "string exit_code should fail dedicated integer check: {:?}",
        failures
    );
}

#[test]
fn exit_code_missing_gets_both_diagnostics() {
    ensure_fixture();
    let fields = baseline_row_fields();
    let mut obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for (k, v) in fields {
        if k != "exit_code" {
            obj.insert(k.to_string(), v.clone());
        }
    }
    let failures = validate_row_shape(&fixture_root(), 1, &obj);
    let has_missing = failures
        .iter()
        .any(|f| f.contains("exit_code") && f.contains("missing required field"));
    let has_integer = failures
        .iter()
        .any(|f| f.contains("exit_code must be integer"));
    assert!(
        has_missing,
        "missing exit_code should be detected: {:?}",
        failures
    );
    assert!(
        has_integer,
        "missing exit_code should also fail integer check: {:?}",
        failures
    );
}

#[test]
fn type_error_reports_json_type_name() {
    ensure_fixture();
    let mut fields = baseline_row_fields();
    set_field(&mut fields, "bead_id", serde_json::json!(42));
    let row = make_row(&fields);
    let obj = row.as_object().expect("object");
    let failures = validate_row_shape(&fixture_root(), 1, obj);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("bead_id") && f.contains("number")),
        "type error should report actual JSON type name: {:?}",
        failures
    );
}
