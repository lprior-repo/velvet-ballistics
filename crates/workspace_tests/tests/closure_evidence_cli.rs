//! Black-box CLI tests for vb-jpq7 closure evidence checker.
//!
//! Each test supplies a private fake `bd` executable via PATH that returns
//! a controlled JSON response. The production code must call the real `bd`
//! binary with the correct arguments and cwd.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process;

const TEST_ROOT: &str = ".evidence/vb-jpq7.48/scratch/closure-evidence-tests";

/// Create a unique test directory and private fake bd fixture.
fn setup_test(name: &str) -> TestFixture {
    let pid = std::process::id();
    let rel_dir = format!("{}/{}/{}", TEST_ROOT, pid, name);
    let abs_dir = fs::canonicalize(&rel_dir).unwrap_or_else(|_| {
        fs::create_dir_all(&rel_dir).expect("failed to create test dir");
        fs::canonicalize(&rel_dir).expect("failed to canonicalize test dir")
    });

    // Create private bin directory with fake bd
    let bin_dir = abs_dir.join("bin");
    fs::create_dir_all(&bin_dir).expect("failed to create bin dir");
    let abs_bin_dir = fs::canonicalize(&bin_dir).expect("failed to canonicalize bin dir");

    // Write fake bd script that captures argv and cwd
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

    // Create beads directory for manifest
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
    fs::remove_dir_all(&fixture.dir).expect("failed to remove test dir");
}

fn write_file(path: &str, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

fn write_manifest(dir: &str, rows: &[serde_json::Value]) -> String {
    let path = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", dir);
    let mut text = String::new();
    for row in rows {
        text.push_str(&serde_json::to_string(row).unwrap());
        text.push('\n');
    }
    write_file(&path, &text);
    path
}

/// Write a bd response file and return its path.
fn write_bd_response(dir: &str, json: &str) -> String {
    let path = format!("{}/bd-response.json", dir);
    write_file(&path, json);
    path
}

/// Run the closure-evidence checker with a private fake bd.
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

    // Verify bd was called with correct args and cwd
    check_bd_args(&cwd_capture, &args_capture, &fixture.dir);

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (exit_code, stdout, stderr)
}

fn check_bd_args(cwd_capture: &str, args_capture: &str, expected_cwd: &str) {
    let actual_cwd = fs::read_to_string(cwd_capture)
        .expect("failed to read bd cwd capture")
        .trim()
        .to_string();
    assert_eq!(
        actual_cwd, expected_cwd,
        "bd was not called with expected cwd"
    );

    let args_text = fs::read_to_string(args_capture)
        .expect("failed to read bd args capture");
    let actual_args: Vec<&str> = args_text
        .lines()
        .filter(|line| !line.is_empty())
        .collect();

    let expected_args = [
        "list",
        "--parent", "vb-jpq7",
        "--status", "all",
        "--json",
        "-n", "0",
    ];

    assert_eq!(
        actual_args, expected_args,
        "bd was not called with expected args"
    );
}

#[test]
fn valid_closure_evidence_passes() {
    let fixture = setup_test("valid");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
        serde_json::json!({"id": "vb-jpq7.2", "status": "open"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 0, "expected exit 0, got {}. stderr: {}", code, stderr);
    assert!(stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"));
    cleanup_test(fixture);
}

#[test]
fn missing_closure_evidence_fails() {
    let fixture = setup_test("missing");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    write_manifest(&fixture.dir, &[]);
    let manifest = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", fixture.dir);
    let (code, _, _) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    cleanup_test(fixture);
}

#[test]
fn self_test_mode_passes() {
    let output = process::Command::new(env!("CARGO_BIN_EXE_closure-evidence"))
        .arg("--self-test")
        .output()
        .expect("failed to run self-test");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(exit_code, 0, "self-test exit code: {}", exit_code);
    assert!(stdout.contains("SELF_TEST_PASS"), "stdout: {}", stdout);
}

#[test]
fn bad_marker_in_evidence_is_rejected() {
    let fixture = setup_test("bad-marker");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "subagent-only result",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("rejected"), "should report rejected marker");
    cleanup_test(fixture);
}

#[test]
fn missing_required_field_is_rejected() {
    let fixture = setup_test("missing-field");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("missing required field"), "should report missing field");
    cleanup_test(fixture);
}

#[test]
fn invalid_commit_sha_is_rejected() {
    let fixture = setup_test("bad-sha");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "not-a-sha",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("invalid commit_sha"), "should report invalid sha");
    cleanup_test(fixture);
}

#[test]
fn non_integer_exit_code_is_rejected() {
    let fixture = setup_test("non-integer-exit");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": "0",
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("exit_code must be integer"), "should report non-integer exit");
    cleanup_test(fixture);
}

#[test]
fn invalid_timestamp_is_rejected() {
    let fixture = setup_test("bad-timestamp");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "yesterday",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("timestamp is not"), "should report invalid timestamp");
    cleanup_test(fixture);
}

#[test]
fn split_followup_nonzero_row_passes_with_resolution() {
    let fixture = setup_test("split-followup");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/split.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command",
        "resolution_kind": "split_followup",
        "resolution_rationale": "will be fixed in followup bead",
        "split_bead_id": "vb-rud5"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 0, "split-followup should pass. exit: {} stderr: {}", code, stderr);
    assert!(stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"));
    cleanup_test(fixture);
}

#[test]
fn nonzero_row_without_resolution_fails() {
    let fixture = setup_test("no-resolution");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "nonzero without resolution should fail. exit: {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("resolution_kind"), "should report missing resolution");
    cleanup_test(fixture);
}

#[test]
fn empty_bd_list_returns_exit2() {
    // Regression: empty child list should be operational error (exit 2), not pass
    let fixture = setup_test("empty-list");
    write_bd_response(&fixture.dir, "[]");

    write_manifest(&fixture.dir, &[]);
    let manifest = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", fixture.dir);
    let (code, stdout, _) = run_checker(&fixture, &format!("{}/bd-response.json", fixture.dir), &manifest);

    assert_eq!(code, 2, "expected exit 2 for empty list, got {}", code);
    assert!(!stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"), "should not report pass");
    cleanup_test(fixture);
}

#[test]
fn child_after_position50_is_validated() {
    // Regression: >50 children should all be validated, not just first 50
    let fixture = setup_test("position50");

    let mut children = Vec::new();
    for i in 1..=55 {
        children.push(serde_json::json!({
            "id": format!("vb-jpq7.{}", i),
            "status": "closed",
            "close_reason": "done"
        }));
    }
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    // Provide evidence for children 1-54, but NOT for child 55
    let mut rows = Vec::new();
    for i in 1..=54 {
        rows.push(serde_json::json!({
            "bead_id": format!("vb-jpq7.{}", i),
            "command": "cargo test",
            "cwd": fixture.dir,
            "commit_sha": "abcdef1",
            "tool_version": "cargo 1.91.0-nightly",
            "timestamp": "2026-01-01T00:00:00Z",
            "raw_log_path": format!("{}/raw{}.log", fixture.dir, i),
            "stdout_summary": "1 passed",
            "stderr_summary": "empty",
            "exit_code": 0,
            "status": "PASS",
            "evidence_kind": "raw-command"
        }));
    }
    write_manifest(&fixture.dir, &rows);
    let manifest = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", fixture.dir);

    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1 for missing evidence at position 55, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("vb-jpq7.55"), "should report missing evidence for child 55");
    cleanup_test(fixture);
}

#[test]
fn missing_manifest_is_validation_failure() {
    // Regression: missing manifest after successful bd query is exit 1 with FAIL marker,
    // not exit 2 operational error.
    let fixture = setup_test("missing-manifest");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    // Don't create the manifest file
    let manifest = format!("{}/.beads/vb-jpq7/closure-evidence-manifest.jsonl", fixture.dir);
    let (code, stdout, _) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1 for missing manifest, got {}", code);
    assert!(stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_FAIL"), "should have FAIL marker");
    assert!(stdout.contains("cannot read manifest"), "should report manifest read error");
    cleanup_test(fixture);
}

#[test]
fn row_exit_code_must_match_log_exit_code() {
    // Regression: raw log provenance must bind numeric exit_code exactly
    let fixture = setup_test("exit-code-mismatch");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1 for exit_code mismatch, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("exit_code: 0"), "should report provenance binding for exit_code");
    cleanup_test(fixture);
}

#[test]
fn whitespace_only_resolution_rationale_fails() {
    // Regression: whitespace-only rationale must be rejected
    let fixture = setup_test("whitespace-rationale");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command",
        "resolution_kind": "split_followup",
        "resolution_rationale": "  ",
        "split_bead_id": "vb-rud5"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1 for whitespace-only rationale, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("resolution_rationale"), "should report missing rationale");
    cleanup_test(fixture);
}

#[test]
fn duplicate_split_bead_id_fails() {
    // Regression: duplicate split_bead_id with same rationale must be rejected
    let fixture = setup_test("duplicate-split");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
        serde_json::json!({"id": "vb-jpq7.2", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command",
        "resolution_kind": "split_followup",
        "resolution_rationale": "fix in followup",
        "split_bead_id": "vb-rud5"
    });

    let row2 = serde_json::json!({
        "bead_id": "vb-jpq7.2",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command",
        "resolution_kind": "split_followup",
        "resolution_rationale": "fix in followup",
        "split_bead_id": "vb-rud5"
    });

    let manifest = write_manifest(&fixture.dir, &[row, row2]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 1, "expected exit 1 for duplicate split, got {}", code);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("duplicate split_followup"), "should report duplicate split");
    cleanup_test(fixture);
}

#[test]
fn approved_waiver_passes() {
    // Regression: approved_waiver with valid waiver_id must pass
    let fixture = setup_test("approved-waiver");
    let children = vec![
        serde_json::json!({"id": "vb-jpq7.1", "status": "closed", "close_reason": "done"}),
    ];
    let response_path = write_bd_response(&fixture.dir, &serde_json::to_string(&children).unwrap());

    let raw_log_path = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw_log_path,
        &format!("command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\nstdout: fail\n", fixture.dir),
    );

    let row = serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": fixture.dir,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "failures",
        "stderr_summary": "empty",
        "exit_code": 1,
        "status": "FAIL",
        "evidence_kind": "raw-command",
        "resolution_kind": "approved_waiver",
        "resolution_rationale": "known issue, approved by team lead",
        "waiver_id": "waiver-123"
    });

    let manifest = write_manifest(&fixture.dir, &[row]);
    let (code, stdout, stderr) = run_checker(&fixture, &response_path, &manifest);

    assert_eq!(code, 0, "approved_waiver should pass. exit: {} stderr: {}", code, stderr);
    assert!(stdout.contains("VB_JPQ7_CLOSURE_EVIDENCE_PASS"));
    cleanup_test(fixture);
}
