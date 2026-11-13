//! Black-box CLI tests for vb-eirmk.74.3.1.1 closure evidence field type validation.
//!
//! Validates that non-string required string fields produce field-specific type
//! errors, fixing the false-pass bug where raw_log_path=0 returned PASS.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process;

const TEST_ROOT: &str = ".evidence/field-types-r1/scratch/cli-tests";

fn setup_test(name: &str) -> TestFixture {
    let pid = std::process::id();
    let rel_dir = format!("{}/{}/{}", TEST_ROOT, pid, name);
    let abs_dir = fs::canonicalize(&rel_dir).unwrap_or_else(|_| {
        fs::create_dir_all(&rel_dir).expect("failed to create test dir");
        fs::canonicalize(&rel_dir).expect("failed to canonicalize test dir")
    });

    let bin_dir = abs_dir.join("bin");
    fs::create_dir_all(&bin_dir).expect("failed to create bin dir");
    let abs_bin_dir = fs::canonicalize(&bin_dir).expect("failed to canonicalize bin dir");

    let bd_path = abs_bin_dir.join("bd");
    let script = format!(
        "#!/bin/sh\n\
        echo \"$PWD\" > \"${{BD_CWD_CAPTURE}}\"\n\
        for arg in \"$@\"; do\n\
          printf '%s\\n' \"$arg\" >> \"${{BD_ARGS_CAPTURE}}\"\n\
        done\n\
        case \"$1\" in\n\
          list)\n\
            if [ \"${{BD_RESPONSE}}\" != \"\" ]; then\n\
              /usr/bin/cat \"${{BD_RESPONSE}}\"\n\
            else\n\
              echo '[]'\n\
            fi\n\
            ;;\n\
          *)\n\
            exit 1\n\
            ;;\n\
        esac\n"
    );
    fs::write(&bd_path, script).expect("failed to write bd script");
    fs::set_permissions(&bd_path, PermissionsExt::from_mode(0o755))
        .expect("failed to make bd executable");

    let beads_dir = abs_dir.join(".beads/vb-jpq7");
    fs::create_dir_all(&beads_dir).expect("failed to create beads dir");

    TestFixture {
        dir: abs_dir.to_string_lossy().to_string(),
        bin_dir: abs_bin_dir.to_string_lossy().to_string(),
    }
}

struct TestFixture {
    dir: String,
    bin_dir: String,
}

fn cleanup_test(fixture: TestFixture) {
    let _ = fs::remove_dir_all(&fixture.dir);
}

fn write_file(path: &str, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

fn write_manifest(dir: &str, row: serde_json::Value) -> String {
    let path = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", dir);
    let text = serde_json::to_string(&row).unwrap() + "\n";
    write_file(&path, &text);
    path
}

fn write_bd_response(dir: &str, bead_id: &str) -> String {
    let children = vec![serde_json::json!({
        "id": bead_id,
        "status": "closed",
        "close_reason": "done"
    })];
    let path = format!("{}/bd-response.json", dir);
    write_file(&path, &serde_json::to_string(&children).unwrap());
    path
}

/// Create a valid baseline row that passes all validation.
fn baseline_row(fixture: &TestFixture) -> serde_json::Value {
    let raw_log_path = format!("{}/raw.log", fixture.dir);
    let log_content = format!(
        "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: 1 passed\n",
        fixture.dir
    );
    write_file(&raw_log_path, &log_content);

    serde_json::json!({
        "bead_id": "vb-test.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef123456",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    })
}

fn run_checker(fixture: &TestFixture, bd_response: &str, manifest: &str) -> (i32, String, String) {
    let cwd_capture = format!("{}/bd-cwd.txt", fixture.dir);
    let args_capture = format!("{}/bd-args.txt", fixture.dir);
    write_file(&args_capture, "");

    let output = process::Command::new(env!("CARGO_BIN_EXE_closure-evidence"))
        .arg("--parent")
        .arg("vb-jpq7")
        .arg("--manifest")
        .arg(manifest)
        .arg("--bd-workdir")
        .arg(&fixture.dir)
        .env("PATH", &fixture.bin_dir)
        .env("BD_RESPONSE", bd_response)
        .env("BD_CWD_CAPTURE", &cwd_capture)
        .env("BD_ARGS_CAPTURE", &args_capture)
        .output()
        .expect("failed to run checker");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (exit_code, stdout, stderr)
}

/// Substitute a non-string value into a required string field and run the checker.
fn run_with_type_error(
    fixture: &TestFixture,
    field: &str,
    bad_value: serde_json::Value,
) -> (i32, String, String) {
    let mut row = baseline_row(fixture);
    row[field] = bad_value;

    let manifest = write_manifest(&fixture.dir, row);
    let bd_response = write_bd_response(&fixture.dir, "vb-test.1");
    run_checker(&fixture, &bd_response, &manifest)
}

#[test]
fn baseline_valid_row_passes() {
    let fixture = setup_test("baseline");
    let row = baseline_row(&fixture);
    let manifest = write_manifest(&fixture.dir, row);
    let bd_response = write_bd_response(&fixture.dir, "vb-test.1");
    let (code, stdout, stderr) = run_checker(&fixture, &bd_response, &manifest);

    assert_eq!(
        code, 0,
        "baseline should pass. exit: {}, stderr: {}",
        code, stderr
    );
    assert!(stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"));
    cleanup_test(fixture);
}

#[test]
fn raw_log_path_number_zero_fails() {
    let fixture = setup_test("raw-log-path-zero");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "raw_log_path", serde_json::json!(0));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(
        code, 1,
        "expected exit 1, got {}. combined: {}",
        code, combined
    );
    assert!(
        combined.contains("raw_log_path") && combined.contains("must be string"),
        "should report field-specific type error for raw_log_path"
    );
    assert!(!stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"));
    cleanup_test(fixture);
}

#[test]
fn raw_log_path_boolean_false_fails() {
    let fixture = setup_test("raw-log-path-bool");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "raw_log_path", serde_json::json!(false));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("raw_log_path") && combined.contains("must be string"),
        "should report field-specific type error for raw_log_path"
    );
    cleanup_test(fixture);
}

#[test]
fn raw_log_path_empty_array_fails() {
    let fixture = setup_test("raw-log-path-array");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "raw_log_path", serde_json::json!([]));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("raw_log_path") && combined.contains("must be string"),
        "should report field-specific type error for raw_log_path"
    );
    cleanup_test(fixture);
}

#[test]
fn raw_log_path_empty_object_fails() {
    let fixture = setup_test("raw-log-path-obj");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "raw_log_path", serde_json::json!({}));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("raw_log_path") && combined.contains("must be string"),
        "should report field-specific type error for raw_log_path"
    );
    cleanup_test(fixture);
}

#[test]
fn bead_id_number_fails() {
    let fixture = setup_test("bead-id-number");
    let (code, stdout, stderr) = run_with_type_error(&fixture, "bead_id", serde_json::json!(123));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("bead_id") && combined.contains("must be string"),
        "should report field-specific type error for bead_id"
    );
    cleanup_test(fixture);
}

#[test]
fn command_boolean_fails() {
    let fixture = setup_test("command-bool");
    let (code, stdout, stderr) = run_with_type_error(&fixture, "command", serde_json::json!(true));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("command") && combined.contains("must be string"),
        "should report field-specific type error for command"
    );
    cleanup_test(fixture);
}

#[test]
fn cwd_array_fails() {
    let fixture = setup_test("cwd-array");
    let (code, stdout, stderr) = run_with_type_error(&fixture, "cwd", serde_json::json!(["."]));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("cwd") && combined.contains("must be string"),
        "should report field-specific type error for cwd"
    );
    cleanup_test(fixture);
}

#[test]
fn commit_sha_object_fails() {
    let fixture = setup_test("commit-sha-obj");
    let (code, stdout, stderr) = run_with_type_error(
        &fixture,
        "commit_sha",
        serde_json::json!({"short": "abc123"}),
    );
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("commit_sha") && combined.contains("must be string"),
        "should report field-specific type error for commit_sha"
    );
    cleanup_test(fixture);
}

#[test]
fn tool_version_null_is_missing() {
    let fixture = setup_test("tool-version-null");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "tool_version", serde_json::json!(null));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("tool_version") && combined.contains("missing required field"),
        "null should be reported as missing"
    );
    cleanup_test(fixture);
}

#[test]
fn timestamp_whitespace_is_missing() {
    let fixture = setup_test("timestamp-blank");
    let (code, stdout, stderr) =
        run_with_type_error(&fixture, "timestamp", serde_json::json!("   "));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("timestamp") && combined.contains("missing required field"),
        "whitespace should be reported as missing"
    );
    cleanup_test(fixture);
}

#[test]
fn exit_code_string_still_rejected_by_integer_check() {
    let fixture = setup_test("exit-code-string");
    let (code, stdout, stderr) = run_with_type_error(&fixture, "exit_code", serde_json::json!("0"));
    let combined = format!("{}{}", stdout, stderr);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    assert!(
        combined.contains("exit_code must be integer"),
        "exit_code string should fail dedicated integer check"
    );
    cleanup_test(fixture);
}
