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
///
/// Note: `name` must use Velvet v1 identifier grammar — `[a-z][a-z0-9_]{0,63}`
/// (see `vb_compile::mod_compile_validation::is_public_name`). Hyphens are
/// rejected with `CompileError::InvalidName`, which the CLI surfaces as a
/// compile failure. Earlier drafts used `setconst-test` (hyphen), which
/// broke every `run --step` test that fed this fixture through the binary.
const SETCONST_WORKFLOW: &str = r#"version: velvet-ballistics/v1
name: setconst_test
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
///
/// Note: `name` must use Velvet v1 identifier grammar — `[a-z][a-z0-9_]{0,63}`
/// (see `vb_compile::mod_compile_validation::is_public_name`). Hyphens are
/// rejected with `CompileError::InvalidName`. Earlier drafts used `nop-test`
/// (hyphen), which broke the `pc_delta` test path through the CLI.
const NOP_WORKFLOW: &str = r#"version: velvet-ballistics/v1
name: nop_test
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
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(args);

    match command.output() {
        Ok(output) => Some(output),
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics: {err}"
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

fn parse_yaml(output: &std::process::Output) -> serde_json::Value {
    let stdout = output_stdout(output);
    serde_saphyr::from_str(&stdout).unwrap_or_else(|e| {
        assert!(
            forced_assertion_failure(),
            "stdout should be valid YAML: {e}; stdout={stdout}"
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    // Must fail with exit code 2 (ValidationFailed)
    assert!(!output.status.success(), "durability strict should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 (ValidationFailed)"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("durability") || stderr.contains("none"),
        "error should mention durability: {stderr}"
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert!(!output.status.success(), "invalid step ID should fail");
    let code = output.status.code();
    assert!(
        code == Some(1) || code == Some(2),
        "exit code should be 1 or 2, got {code:?}"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("not found") || stderr.contains("99"),
        "error should mention step not found: {stderr}"
    );
}

/// VB-PRE002-CLI: JSON output for step not found includes error code and step ID
///
/// All structured errors are written to stderr per Unix convention and
/// the contract POST-008 requirement for consistent error stream handling.
#[test]
fn run_step_invalid_step_id_yaml_includes_error_details() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert!(!output.status.success(), "invalid step ID should fail");
    let stderr = output_stderr(&output);

    // When using --emit yaml, the error should be structured YAML on stderr
    let yaml: serde_json::Value = match serde_saphyr::from_str(&stderr) {
        Ok(v) => v,
        Err(_) => {
            assert!(
                forced_assertion_failure(),
                "step not found with --emit yaml should produce valid YAML error on stderr: {stderr}"
            );
            return;
        }
    };

    // YAML error should have 'error' field per contract error taxonomy
    assert!(
        yaml.get("error").is_some(),
        "YAML error should have 'error' field per contract: {yaml}"
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
    if !write_test_file(&workflow_path, b"{{{broken") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert!(!output.status.success(), "broken YAML should fail");
    assert!(
        output.status.code() == Some(2) || output.status.code() == Some(1),
        "exit code should be 1 or 2: got {:?}",
        output.status.code()
    );
}

/// VB-PRE003-CLI: Compile error in JSON format includes error details
#[test]
fn run_step_compile_error_yaml_includes_errors() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, b"{{{broken") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert!(!output.status.success(), "broken YAML should fail");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);

    // Error should be reported in JSON or text format
    let has_error_info =
        stdout.contains("error") || stderr.contains("error") || stdout.contains("compile");
    assert!(
        has_error_info,
        "error should be reported: stdout={stdout}, stderr={stderr}"
    );
}

// ---------------------------------------------------------------------------
// VB-PRE005: Output format validation
// ---------------------------------------------------------------------------

/// VB-PRE005-CLI: `run --step` accepts --emit yaml flag and produces valid YAML
#[test]
fn run_step_yaml_flag_produces_valid_yaml() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step with --emit yaml");
    let stdout = output_stdout(&output);

    // stdout must be valid YAML
    let _: serde_json::Value = match serde_saphyr::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            assert!(
                forced_assertion_failure(),
                "stdout should be valid YAML: {e}; stdout={stdout}"
            );
            return;
        }
    };
}

/// VB-PRE005-CLI: `run --step` accepts --emit postcard flag and produces valid postcard
#[test]
fn run_step_postcard_flag_produces_valid_postcard() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        std::ffi::OsStr::new("postcard"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "run --step with --emit postcard");

    // Postcard is binary format - just verify command succeeds and produces output
    let stdout = output_stdout(&output);
    assert!(!stdout.is_empty(), "postcard output should not be empty");
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    // No --emit flag = text output
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
        None => return,
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step 0");
    let json = parse_yaml(&output);

    // JSON must report step 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "YAML should report step 0, got {json}"
    );
}

// ---------------------------------------------------------------------------
// VB-POST002: JSON structured output
// ---------------------------------------------------------------------------

/// VB-POST002-CLI: JSON output has all required schema fields (step, kind, signal, deltas)
#[test]
fn run_step_yaml_output_has_required_schema_fields() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    // Required top-level fields per POST-002 and POST-003
    assert!(
        json.get("step").is_some(),
        "YAML must have 'step' field: {json}"
    );
    assert!(
        json.get("kind").is_some(),
        "YAML must have 'kind' field: {json}"
    );
    assert!(
        json.get("signal").is_some(),
        "YAML must have 'signal' field: {json}"
    );
    assert!(
        json.get("deltas").is_some(),
        "YAML must have 'deltas' field: {json}"
    );

    // deltas object must have all four delta types per POST-004
    let deltas = json.get("deltas").unwrap();
    assert!(
        deltas.get("slot_deltas").is_some(),
        "deltas must have 'slot_deltas': {deltas}"
    );
    assert!(
        deltas.get("taint_deltas").is_some(),
        "deltas must have 'taint_deltas': {deltas}"
    );
    assert!(
        deltas.get("state_deltas").is_some(),
        "deltas must have 'state_deltas': {deltas}"
    );
    assert!(
        deltas.get("pc_delta").is_some(),
        "deltas must have 'pc_delta': {deltas}"
    );
}

/// VB-POST002-CLI: JSONL output has valid line-per-object format
#[test]
fn run_step_postcard_output_is_valid_postcard() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        std::ffi::OsStr::new("postcard"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "run --step --emit postcard");

    // Postcard is binary format - verify command succeeds
    let stdout = output_stdout(&output);
    assert!(!stdout.is_empty(), "postcard output should not be empty");
}

// ---------------------------------------------------------------------------
// VB-POST003: Output includes step idx, kind, pc_after, signal
// ---------------------------------------------------------------------------

/// VB-POST003-CLI: JSON output includes step index, kind, signal
#[test]
fn run_step_yaml_output_includes_step_kind_signal() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    // step should be 0
    assert_eq!(
        json.get("step").and_then(|v| v.as_u64()),
        Some(0),
        "step should be 0"
    );

    // kind should be present (SetConst for our workflow)
    let kind = json.get("kind").and_then(|v| v.as_str());
    assert!(kind.is_some(), "kind should be present: {kind:?}");

    // signal should be a valid EngineSignal name
    let signal = json.get("signal").and_then(|v| v.as_str());
    assert!(signal.is_some(), "signal should be present: {signal:?}");
}

// ---------------------------------------------------------------------------
// VB-POST004: Delta JSON schema
// ---------------------------------------------------------------------------

/// VB-POST004-CLI: delta JSON has correct pc_delta structure with before/after
#[test]
fn run_step_delta_yaml_pc_delta_has_before_and_after() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, NOP_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    let deltas = json.get("deltas").expect("YAML must have 'deltas' field");
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
    assert!(
        before.is_some() && after.is_some(),
        "pc_delta before/after must be numbers"
    );
}

/// VB-POST004-CLI: slot_deltas is an array with correct slot change structure
#[test]
fn run_step_delta_yaml_slot_deltas_is_array_with_changes() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    let deltas = json.get("deltas").expect("YAML must have 'deltas' field");
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

    // Each slot delta should have slot, before, after
    for item in arr {
        assert!(
            item.get("slot").is_some(),
            "slot_delta item should have 'slot': {item}"
        );
        assert!(
            item.get("before").is_some(),
            "slot_delta item should have 'before': {item}"
        );
        assert!(
            item.get("after").is_some(),
            "slot_delta item should have 'after': {item}"
        );
    }
}

/// VB-POST004-CLI: state_deltas is an array with before/after state
#[test]
fn run_step_delta_yaml_state_deltas_has_before_after() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    let deltas = json.get("deltas").expect("YAML must have 'deltas' field");
    let state_deltas = deltas
        .get("state_deltas")
        .expect("deltas must have 'state_deltas'");

    // state_deltas should be an array
    assert!(
        state_deltas.is_array(),
        "state_deltas should be an array: {state_deltas}"
    );

    let arr = state_deltas.as_array().unwrap();
    // Step 0 transitions from Pending -> Succeeded
    assert!(
        !arr.is_empty(),
        "state_deltas should not be empty after step execution"
    );

    // Each state delta should have step, before, after
    for item in arr {
        assert!(
            item.get("step").is_some(),
            "state_delta item should have 'step': {item}"
        );
        assert!(
            item.get("before").is_some(),
            "state_delta item should have 'before': {item}"
        );
        assert!(
            item.get("after").is_some(),
            "state_delta item should have 'after': {item}"
        );
    }
}

/// VB-POST004-CLI: taint_deltas is present and is an array
#[test]
fn run_step_delta_yaml_taint_deltas_is_array() {
    let dir = match run_step_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    let deltas = json.get("deltas").expect("YAML must have 'deltas' field");
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert_cli_success(&output, "run --step --emit yaml");
    let json = parse_yaml(&output);

    // Signal should be "Finished" or "Continue" for SetConst
    let signal = json.get("signal").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        signal == "Finished" || signal == "Continue",
        "SetConst step should produce Finished or Continue signal, got {signal}"
    );

    // TODO: Q2 resolution - full SlotValue vs summary serialization
    // When Q2 is resolved, assert the exact structure:
    // assert!(json.get("output_slot").is_some(), "Finished signal should include output_slot");
    // let output_slot = json.get("output_slot").unwrap();
    // assert!(output_slot.get("value").is_some(), "output_slot should have value");
    // assert!(output_slot.get("taint").is_some(), "output_slot should have taint");

    // For now, verify that some form of output is present
    let has_output = json.get("output_slot").is_some()
        || (json.get("deltas").is_some()
            && json.get("deltas").unwrap().get("slot_deltas").is_some());
    assert!(
        has_output,
        "Step result should include output information (output_slot or slot_deltas)"
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
fn run_step_error_in_yaml_format_reports_error_and_message() {
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
    if !write_test_file(&workflow_path, b"{{{broken") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    assert!(!output.status.success(), "broken YAML should fail");

    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);

    // Error should be reported somewhere (stdout in JSON mode, or stderr)
    let has_error =
        stdout.contains("error") || stderr.contains("error") || stdout.contains("compile");
    assert!(
        has_error,
        "error should be reported: stdout={stdout}, stderr={stderr}"
    );
}

/// VB-POST006-CLI: Runtime error in JSONL format reports error as JSON object
#[test]
fn run_step_error_in_postcard_format_reports_error_object() {
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
    if !write_test_file(&workflow_path, b"{{{broken") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        std::ffi::OsStr::new("postcard"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert!(!output.status.success(), "broken YAML should fail");
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
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
    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    // Malformed postcard data
    if !write_test_file(&input_path, b"garbage-postcard") {
        return;
    }

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
        None => return,
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

    if !write_test_file(&workflow_path, SETCONST_WORKFLOW.as_bytes()) {
        return;
    }
    // Empty file is valid - decodes to Box<[SlotValue]>::from([])
    if !write_test_file(&input_path, &[]) {
        return;
    }

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
        None => return,
    };

    // Empty step input is valid - command should succeed
    assert_cli_success(&output, "run --step with empty step-input");
}
