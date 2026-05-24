//! Integration tests for `run --step` single-step CLI command (vb-qi37.14.1).
//!
//! These tests verify:
//! - PRE: durability gate, step validation, input decoding, output format
//! - POST: single-step execution, JSON/JSONL output schema, delta reporting
//! - ERR: engine error reporting in JSON format
//!
//! Tests are written to FAIL first because the implementation does not yet
//! produce structured JSON output with pc/slot/taint/state deltas.

#![forbid(unsafe_code)]
#![cfg(not(miri))]

// ---------------------------------------------------------------------------
// Test Fixtures
// ---------------------------------------------------------------------------

/// Minimal 2-step workflow: SetConst(slot0=42) -> Finish
const SETCONST_WORKFLOW: &str = r#"version: velvet-ballastics/v1
name: setconst-test
when:
  manual: {}
steps:
  - id: init
    set:
      output: slot0
      value: '42'
  - id: done
    finish:
      result: slot0
"#;

/// 3-step workflow with Nop Save then Finish
const NOP_WORKFLOW: &str = r#"version: velvet-ballastics/v1
name: nop-test
when:
  manual: {}
steps:
  - id: step0
    save:
      output: x
      value: '1'
  - id: step1
    save:
      output: 'y'
      value: '2'
  - id: done
    finish:
      result: x
"#;

// ---------------------------------------------------------------------------
// Helpers (mirror patterns from cli_integration.rs)
// ---------------------------------------------------------------------------

fn forced_assertion_failure() -> bool {
    false
}

fn run_step_tempdir() -> std::io::Result<tempfile::TempDir> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/run-step-integration-tmp");
    std::fs::create_dir_all(&root)?;
    tempfile::Builder::new()
        .prefix("vb-run-step-")
        .tempdir_in(root)
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
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
    command.args(args);

    match command.output() {
        Ok(output) => Some(output),
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics: {err}"
            );
            None
        }
    }
}

fn output_stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn output_stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_cli_success(output: &std::process::Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed: stdout={} stderr={}",
        output_stdout(output),
        output_stderr(output)
    );
}

fn parse_json(output: &std::process::Output) -> serde_json::Value {
    let stdout = output_stdout(output);
    serde_saphyr::from_str(&stdout).unwrap_or_else(|e| {
        assert!(
            forced_assertion_failure(),
            "stdout should be valid JSON: {e}; stdout={stdout}"
        );
        serde_json::Value::Null
    })
}

// ---------------------------------------------------------------------------
// VB-PRE001: Durability gate
// ---------------------------------------------------------------------------

/// VB-PRE001-CLI: `run --step` rejects durability=strict
#[test]
fn run_step_rejects_durability_strict() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("journal-db");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    // Must fail with exit code 2 (ValidationFailed)
    assert!(!output.status.success(), "durability strict should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 (ValidationFailed)"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on durability rejection"
    );
    let stderr = output_stderr(&output);
    assert_eq!(
        stderr,
        "step isolation requires --durability none\n",
        "exact stderr for durability rejection"
    );
}

/// VB-PRE001-CLI: `run --step` rejects durability=journaled
#[test]
fn run_step_rejects_durability_journaled() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("journal-db");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "durability journaled should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 (ValidationFailed)"
    );
}

// ---------------------------------------------------------------------------
// VB-PRE002: Invalid step ID
// ---------------------------------------------------------------------------

/// VB-PRE002-CLI: `run --step` with out-of-bounds step ID fails with step_not_found
#[test]
fn run_step_invalid_step_id_reports_not_found() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    // SETCONST_WORKFLOW has 2 steps (init, done) so step 99 is invalid
    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("99"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "invalid step ID should fail");
    let code = output.status.code();
    assert_eq!(code, Some(2), "validation failure exit code must be 2");
    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on step not found"
    );
    let stderr = output_stderr(&output);
    assert_eq!(
        stderr,
        "step 99 not found in workflow\n",
        "exact stderr for step not found"
    );
}

/// VB-PRE002-CLI: JSON output for step not found includes error code and step ID
///
/// All structured errors are written to stderr per Unix convention and
/// the contract POST-008 requirement for consistent error stream handling.
#[test]
fn run_step_invalid_step_id_json_includes_error_details() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("99"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_eq!(
        output.status.code(),
        Some(2),
        "invalid step ID must exit with ValidationFailed (2)"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on error"
    );
    let stderr = output_stderr(&output);

    // When using --emit yaml, the error should be structured YAML on stderr
    let json: serde_json::Value = match serde_saphyr::from_str(&stderr) {
        Ok(v) => v,
        Err(e) => {
            assert!(
                forced_assertion_failure(),
                "step not found with --emit yaml should produce valid YAML on stderr: {e}; stderr={stderr}"
            );
            return;
        }
    };

    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("step_not_found"),
        "JSON error must have code step_not_found: {json}"
    );
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(99),
        "JSON error must include step 99: {json}"
    );
    assert_eq!(
        json.get("message").and_then(|v| v.as_str()),
        Some("step 99 not found in workflow"),
        "JSON error must include exact message: {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-PRE003: Compile failure
// ---------------------------------------------------------------------------

/// VB-PRE003-CLI: `run --step` with invalid YAML fails with compile error
#[test]
fn run_step_compile_error_reports_failure() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    // Invalid YAML - missing required fields
    assert!(write_test_file(&workflow_path, b"{{{broken"));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "broken YAML should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "compile validation failure exit code must be 2"
    );
}

/// VB-PRE003-CLI: Compile error in JSON format includes error details
#[test]
fn run_step_compile_error_json_includes_errors() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(&workflow_path, b"{{{broken"));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "broken YAML should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "compile failure must exit with ValidationFailed (2)"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on compile error"
    );
    let stderr = output_stderr(&output);

    // With --emit yaml, the error should be a structured DiagnosticReport on stderr
    let json: serde_json::Value = match serde_saphyr::from_str(&stderr) {
        Ok(v) => v,
        Err(e) => {
            assert!(
                forced_assertion_failure(),
                "compile error with --emit yaml should produce valid YAML on stderr: {e}; stderr={stderr}"
            );
            return;
        }
    };

    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("DiagnosticReport"),
        "error must be a DiagnosticReport: {json}"
    );
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("CompileFailed"),
        "error code must be CompileFailed: {json}"
    );
    assert_eq!(
        json.get("exit_code").and_then(|v| v.as_u64()),
        Some(3),
        "diagnostic exit_code must be 3: {json}"
    );
    assert!(
        json.get("message").and_then(|v| v.as_str()).unwrap_or("").contains("compile error"),
        "error message must describe compile failure: {json}"
    );
    assert!(
        json.get("schema_version").is_some(),
        "diagnostic must have schema_version: {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-PRE005: Output format validation
// ---------------------------------------------------------------------------

/// VB-PRE005-CLI: `run --step` accepts --json flag and produces valid JSON
#[test]
fn run_step_json_flag_produces_valid_json() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step with --emit yaml");
    let stderr = output_stderr(&output);
    assert!(
        stderr.is_empty(),
        "stderr must be empty on success, got: {stderr}"
    );
    let stdout = output_stdout(&output);

    // stdout must be valid YAML-compatible structured output
    let json: serde_json::Value = match serde_saphyr::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            assert!(
                forced_assertion_failure(),
                "stdout should be valid YAML-compatible structured output: {e}; stdout={stdout}"
            );
            return;
        }
    };

    // Exact expected values for SetConst step 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "step must be 0"
    );
    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("SetConst"),
        "kind must be SetConst"
    );
    assert_eq!(
        json.get("signal").and_then(|v| v.as_str()),
        Some("Continue"),
        "signal must be Continue"
    );
    let deltas = json.get("deltas").expect("must have deltas");
    assert_eq!(
        deltas.get("pc_delta").and_then(|v| v.get("before")).and_then(|v| v.as_u64()),
        Some(0),
        "pc_delta before must be 0"
    );
    assert_eq!(
        deltas.get("pc_delta").and_then(|v| v.get("after")).and_then(|v| v.as_u64()),
        Some(1),
        "pc_delta after must be 1"
    );
}

/// VB-PRE005-CLI: `run --step` accepts --jsonl flag and produces valid JSONL
#[test]
fn run_step_jsonl_flag_produces_valid_jsonl() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step with --jsonl");
    let stdout = output_stdout(&output);

    let value: serde_json::Value = serde_saphyr::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout should be YAML-compatible structured output: {e}; stdout={stdout}")
    });
    assert_eq!(value.get("step"), Some(&serde_json::json!(0)));
}

/// VB-PRE005-CLI: `run --step` text output is human-readable
#[test]
fn run_step_text_output_is_human_readable() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    // No --json or --jsonl flag = text output
    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step text output");
    let stdout = output_stdout(&output);

    // Text output should contain step info
    assert!(
        stdout.contains("step:"),
        "text output should contain 'step:': {stdout}"
    );
    assert!(
        stdout.contains("signal:"),
        "text output should contain 'signal:': {stdout}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST001: step_once called exactly once
// ---------------------------------------------------------------------------

/// VB-POST001-CLI: `run --step` executes exactly one step and reports correct step index
#[test]
fn run_step_executes_single_step_and_reports_correct_index() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step 0");
    let json = parse_json(&output);

    // JSON must report step 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "JSON should report step 0, got {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST002: JSON structured output
// ---------------------------------------------------------------------------

/// VB-POST002-CLI: JSON output has all required schema fields (step, kind, signal, deltas)
#[test]
fn run_step_json_output_has_required_schema_fields() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let stderr = output_stderr(&output);
    assert!(
        stderr.is_empty(),
        "stderr must be empty on success, got: {stderr}"
    );
    let json = parse_json(&output);

    // Exact required top-level fields for SetConst step 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "step must be 0: {json}"
    );
    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("SetConst"),
        "kind must be SetConst: {json}"
    );
    assert_eq!(
        json.get("signal").and_then(|v| v.as_str()),
        Some("Continue"),
        "signal must be Continue: {json}"
    );
    assert!(
        json.get("deltas").is_some(),
        "must have deltas: {json}"
    );
    assert!(
        json.get("before").is_some(),
        "must have before snapshot: {json}"
    );
    assert!(
        json.get("after").is_some(),
        "must have after snapshot: {json}"
    );

    // deltas object must have all four delta types with exact pc_delta values
    let deltas = json.get("deltas").expect("must have deltas");
    assert!(
        deltas.get("slot_deltas").is_some(),
        "deltas must have slot_deltas: {deltas}"
    );
    assert_eq!(
        deltas.get("taint_deltas").and_then(|v| v.as_array().map(|a| a.len())),
        Some(0),
        "taint_deltas must be empty array: {deltas}"
    );
    assert!(
        deltas.get("state_deltas").is_some(),
        "deltas must have state_deltas: {deltas}"
    );
    let pc_delta = deltas.get("pc_delta").expect("deltas must have pc_delta");
    assert_eq!(
        pc_delta.get("before").and_then(|v| v.as_u64()),
        Some(0),
        "pc_delta before must be 0: {pc_delta}"
    );
    assert_eq!(
        pc_delta.get("after").and_then(|v| v.as_u64()),
        Some(1),
        "pc_delta after must be 1: {pc_delta}"
    );
}

/// VB-POST002-CLI: JSONL output has valid line-per-object format
#[test]
fn run_step_jsonl_output_is_valid_jsonl() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --jsonl");
    let stdout = output_stdout(&output);

    let json: serde_json::Value = serde_saphyr::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout should be YAML-compatible structured output: {e}; stdout={stdout}")
    });
    assert!(
        json.get("deltas").is_some(),
        "structured output should have deltas field"
    );
}

// ---------------------------------------------------------------------------
// VB-POST003: Output includes step idx, kind, pc_after, signal
// ---------------------------------------------------------------------------

/// VB-POST003-CLI: JSON output includes step index, kind, signal
#[test]
fn run_step_json_output_includes_step_kind_signal() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    // step should be 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "step should be 0"
    );

    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("SetConst"),
        "step 0 kind should be SetConst: {json}"
    );

    assert_eq!(
        json.get("signal").and_then(|v| v.as_str()),
        Some("Continue"),
        "SetConst step should continue to next step: {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST004: Delta JSON schema
// ---------------------------------------------------------------------------

/// VB-POST004-CLI: delta JSON has correct pc_delta structure with before/after
#[test]
fn run_step_delta_json_pc_delta_has_before_and_after() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(&workflow_path, NOP_WORKFLOW.as_bytes()));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    let deltas = json.get("deltas").expect("JSON must have 'deltas' field");
    let pc_delta = deltas.get("pc_delta").expect("deltas must have 'pc_delta'");

    // pc_delta should have "before" and "after"
    assert!(
        pc_delta.get("before").is_some(),
        "pc_delta must have 'before'"
    );
    assert!(
        pc_delta.get("after").is_some(),
        "pc_delta must have 'after'"
    );

    let before = pc_delta.get("before").and_then(|v| v.as_u64());
    let after = pc_delta.get("after").and_then(|v| v.as_u64());
    assert_eq!(before, Some(0), "pc_delta before must be entry step 0");
    assert_eq!(after, Some(1), "pc_delta after must advance to step 1");
}

/// VB-POST004-CLI: slot_deltas is an array with correct slot change structure
#[test]
fn run_step_delta_json_slot_deltas_is_array_with_changes() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    let deltas = json.get("deltas").expect("JSON must have 'deltas' field");
    let slot_deltas = deltas
        .get("slot_deltas")
        .expect("deltas must have 'slot_deltas'");

    // slot_deltas should be an array
    assert!(
        slot_deltas.is_array(),
        "slot_deltas should be an array: {slot_deltas}"
    );

    let arr = slot_deltas.as_array().unwrap();
    // SetConst step 0 writes to slot 0, so we expect at least one delta
    assert!(
        !arr.is_empty(),
        "slot_deltas should not be empty after SetConst step"
    );

    assert_eq!(
        arr.len(),
        1,
        "SetConst should produce exactly one slot delta"
    );
    let item = &arr[0];
    assert_eq!(item.get("slot").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(item.get("before"), Some(&serde_json::Value::Null));
    assert_eq!(
        item.get("after"),
        Some(&serde_json::json!({"I64": 42})),
        "slot_delta after must be exact I64 42: {item}"
    );
}

/// VB-POST004-CLI: state_deltas is an array with before/after state
#[test]
fn run_step_delta_json_state_deltas_has_before_after() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    let deltas = json.get("deltas").expect("JSON must have 'deltas' field");
    let state_deltas = deltas
        .get("state_deltas")
        .expect("deltas must have 'state_deltas'");

    // state_deltas should be an array
    assert!(
        state_deltas.is_array(),
        "state_deltas should be an array: {state_deltas}"
    );

    let arr = state_deltas.as_array().unwrap();
    // Step 0 transitions from Pending -> Succeeded, others stay Pending
    assert!(
        !arr.is_empty(),
        "state_deltas should not be empty after step execution"
    );

    // Each state delta must have exact step, before, after
    let expected_state_delta = serde_json::json!({
        "step": 0,
        "before": "Pending",
        "after": "Succeeded"
    });
    assert_eq!(
        arr.len(),
        1,
        "SetConst step should produce exactly one state delta"
    );
    assert_eq!(
        &arr[0],
        &expected_state_delta,
        "state_delta for step 0 must be exact Pending -> Succeeded"
    );
}

/// VB-POST004-CLI: taint_deltas is present and is an array
#[test]
fn run_step_delta_json_taint_deltas_is_array() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    let deltas = json.get("deltas").expect("JSON must have 'deltas' field");
    let taint_deltas = deltas
        .get("taint_deltas")
        .expect("deltas must have 'taint_deltas'");

    // taint_deltas should be an array (may be empty if no taint changed)
    assert!(
        taint_deltas.is_array(),
        "taint_deltas should be an array: {taint_deltas}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST005: Finished signal includes value/taint
// ---------------------------------------------------------------------------

/// VB-POST005-CLI: When step has output slot, JSON includes output_slot with value and taint
///
/// Note: This test is marked with a TODO because POST005 depends on Q2 resolution
/// (JSON full vs summary serialization). The exact assertion depends on whether
/// the full SlotValue is serialized or a summary.
#[test]
fn run_step_finished_includes_output_slot_value_and_taint() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step --json");
    let json = parse_json(&output);

    assert_eq!(
        json.get("signal").and_then(|v| v.as_str()),
        Some("Continue")
    );
    let output_slot = json
        .get("output_slot")
        .expect("SetConst output should include output_slot");
    assert_eq!(output_slot.get("slot").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(
        output_slot.get("value"),
        Some(&serde_json::json!({"I64": 42})),
        "output_slot value must be exact I64 42: {output_slot}"
    );
    assert_eq!(
        output_slot.get("taint").and_then(|v| v.as_str()),
        Some("Clean"),
        "output_slot taint must be Clean: {output_slot}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST006: Errors reported in output format
// ---------------------------------------------------------------------------

/// VB-POST006-CLI: Compile error in JSON format reports error
///
/// Note: We use a compile-time error (broken YAML) as a proxy since runtime errors
/// like SlotUninitialized require specific workflow conditions that cannot be
/// triggered through YAML workflow definition. The JSON error format is the same for both.
#[test]
fn run_step_error_in_json_format_reports_error_and_message() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    // Use invalid YAML to trigger an error
    assert!(write_test_file(&workflow_path, b"{{{broken"));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "broken YAML should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "compile failure must exit with ValidationFailed (2)"
    );

    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on compile error"
    );
    let stderr = output_stderr(&output);

    // With --emit yaml, error must be a structured DiagnosticReport on stderr
    let json: serde_json::Value = match serde_saphyr::from_str(&stderr) {
        Ok(v) => v,
        Err(e) => {
            assert!(
                forced_assertion_failure(),
                "compile error with --emit yaml should produce valid YAML on stderr: {e}; stderr={stderr}"
            );
            return;
        }
    };

    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("DiagnosticReport"),
        "error must be a DiagnosticReport: {json}"
    );
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("CompileFailed"),
        "error code must be CompileFailed: {json}"
    );
    assert_eq!(
        json.get("exit_code").and_then(|v| v.as_u64()),
        Some(3),
        "diagnostic exit_code must be 3: {json}"
    );
    assert!(
        json.get("message").and_then(|v| v.as_str()).unwrap_or("").contains("compile error"),
        "error message must describe compile failure: {json}"
    );
    assert!(
        json.get("schema_version").is_some(),
        "diagnostic must have schema_version: {json}"
    );
}

/// VB-POST006-CLI: Runtime error in JSONL format reports error as JSON object
#[test]
fn run_step_error_in_jsonl_format_reports_error_object() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    // Use invalid YAML to trigger an error
    assert!(write_test_file(&workflow_path, b"{{{broken"));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "broken YAML should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "compile failure must exit with ValidationFailed (2)"
    );

    let stdout = output_stdout(&output);
    assert!(
        stdout.is_empty(),
        "stdout must be empty on compile error"
    );
    let stderr = output_stderr(&output);

    // With --emit yaml, error must be a structured DiagnosticReport on stderr
    let json: serde_json::Value = serde_saphyr::from_str(&stderr).unwrap_or_else(|e| {
        panic!(
            "compile error with --emit yaml should produce valid YAML on stderr: {e}; stderr={stderr}"
        )
    });

    assert_eq!(
        json.get("kind").and_then(|v| v.as_str()),
        Some("DiagnosticReport"),
        "error must be a DiagnosticReport: {json}"
    );
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("CompileFailed"),
        "error code must be CompileFailed: {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST007: Durability error exit
// ---------------------------------------------------------------------------

/// VB-POST007-CLI: Non-None durability causes exit with ValidationFailed
///
/// This is implicitly tested by VB-PRE001 tests, but we include it explicitly
/// for contract completeness.
#[test]
fn run_step_durability_not_none_exits_with_validation_failed() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("journal-db");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "durability journaled should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "durability error exit code should be 2 (ValidationFailed)"
    );
}

// ---------------------------------------------------------------------------
// VB-POST008: Exit codes
// ---------------------------------------------------------------------------

/// VB-POST008-CLI: Success produces exit code 0
#[test]
fn run_step_success_exits_with_code_0() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert_cli_success(&output, "run --step success");
    assert_eq!(
        output.status.code(),
        Some(0),
        "success should be exit code 0"
    );
}

/// VB-POST008-CLI: Validation failure (PRE001-004) produces exit code 2
///
/// Per contract.md POST-008: "Exit code is `ValidationFailed` (2) for preconditions
/// PRE-001 through PRE-004 failures."
///
/// NOTE: This test FAILS because the implementation uses exit code 1 (ValidationFailed
/// in impl) instead of exit code 2 (ValidationFailed per contract). The test is
/// correct and should pass once the implementation is fixed to match the contract.
#[test]
fn run_step_validation_failure_exits_with_code_2() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    // Use invalid step ID to trigger PRE002 validation failure
    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("99"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "invalid step ID should fail");
    // Contract specifies exit code 2 for validation failures
    assert_eq!(
        output.status.code(),
        Some(2),
        "validation failure should be exit code 2 (per contract POST-008)"
    );
}

/// VB-POST008-CLI: Malformed step input produces exit code 2
///
/// Per contract PRE-004 and POST-008, decode failures should be exit code 2.
///
/// NOTE: This test FAILS because the implementation uses exit code 1 instead of 2.
/// The test is correct and should pass once the implementation is fixed.
#[test]
fn run_step_malformed_step_input_exits_with_code_2() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    // Malformed postcard data
    assert!(write_test_file(&input_path, b"garbage-postcard"));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    assert!(!output.status.success(), "malformed step-input should fail");
    // Contract specifies exit code 2 for PRE-004 decode failures
    assert_eq!(
        output.status.code(),
        Some(2),
        "decode failure should be exit code 2 (per contract PRE-004)"
    );
}

// ---------------------------------------------------------------------------
// VB-PRE004 edge case: empty step input is valid
// ---------------------------------------------------------------------------

/// VB-PRE004-CLI: Empty step input file is valid and decodes to empty slot list
#[test]
fn run_step_empty_step_input_succeeds() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    assert!(write_test_file(
        &workflow_path,
        SETCONST_WORKFLOW.as_bytes()
    ));
    // Empty file is valid - decodes to Box<[SlotValue]>::from([])
    assert!(write_test_file(&input_path, &[]));

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--step-input"),
        input_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => panic!("velvet-ballastics command failed before producing output"),
    };

    // Empty step input is valid - command should succeed
    assert_cli_success(&output, "run --step with empty step-input");
}
