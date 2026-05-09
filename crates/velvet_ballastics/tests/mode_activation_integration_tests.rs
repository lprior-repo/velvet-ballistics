//! Integration tests for mode activation boundaries (POST-002, POST-003, POST-004, POST-005).
//!
//! These tests run the actual velvet-ballastics binary and verify:
//! - Pure commands succeed without storage present (POST-002)
//! - Storage commands fail fast with StorageError on invalid paths (POST-004)
//! - Exit codes are stable regardless of inactive subsystems (POST-005)
//! - FjallJournal::open is called only from Storage/Runtime mode commands (INV-001)
//!
//! RED PHASE: These tests fail because ModeError/command_mode are not yet implemented.
//! The binary will still run, but the mode activation boundary enforcement is not yet
//! in place.

#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The velvet-ballastics binary path.
fn velvet_bin() -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("target")
        });
    target_dir.join("debug").join("velvet-ballastics")
}

/// Run the velvet-ballastics binary with the given args.
fn run_bin<I>(args: I) -> Output
where
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    Command::new(velvet_bin())
        .args(args)
        .output()
        .expect("velvet-ballastics binary must be built")
}

/// Create a minimal valid workflow YAML in a temp directory.
fn temp_workflow(contents: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir available");
    let workflow_path = dir.path().join("workflow.yaml");
    fs::write(&workflow_path, contents).expect("workflow writable");
    dir
}

/// Create an empty input bin file in a temp directory.
fn temp_input_bin() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir available");
    let input_path = dir.path().join("input.bin");
    fs::write(&input_path, b"").expect("input bin writable");
    dir
}

// =============================================================================
// SECTION 1: Pure Mode — validate (POST-002)
// =============================================================================

#[test]
fn validate_succeeds_on_valid_workflow_without_storage() {
    // POST-002: Pure commands run without storage runtime or UI side effects
    // Given: a valid workflow.yaml
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    // When: user runs validate
    let output = run_bin(["velvet-ballastics", "validate", &workflow.to_string_lossy()]);

    // Then: exit code is 0
    assert_eq!(
        output.status.code(),
        Some(0),
        "validate must exit 0 on valid workflow, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_fails_on_invalid_workflow_without_storage() {
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    # missing action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin(["velvet-ballastics", "validate", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "validate on invalid workflow must exit 1, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_succeeds_when_no_storage_path_exists() {
    // POST-005: Running validate with no storage path present succeeds with code 0
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    // No --db argument, no storage path
    let output = run_bin(["velvet-ballastics", "validate", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "validate must succeed (exit 0) even without storage, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// SECTION 2: Pure Mode — verify (POST-002)
// =============================================================================

#[test]
fn verify_succeeds_on_passing_workflow() {
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin([
        "velvet-ballastics",
        "verify",
        &workflow.to_string_lossy(),
        "--profile",
        "quick",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "verify must exit 0 on passing workflow, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().ends_with("verified"),
        "verify output must end with 'verified', got: {}",
        stdout
    );
}

#[test]
fn verify_succeeds_with_json_output() {
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin([
        "velvet-ballastics",
        "verify",
        &workflow.to_string_lossy(),
        "--profile",
        "quick",
        "--json",
    ]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"success\":true"),
        "JSON must have success:true: {stdout}"
    );
    assert!(
        stdout.contains("\"profile\":\"quick\""),
        "JSON must have profile:quick: {stdout}"
    );
}

#[test]
fn verify_fails_with_exit_2_on_failing_workflow() {
    // BDD: verify fails with exit 2 (VerificationFailed) on failing workflow
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    # missing action — fails verification
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin(["velvet-ballastics", "verify", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "verify on failing workflow must exit 2 (VerificationFailed)"
    );
}

#[test]
fn verify_does_not_require_db() {
    // POST-002: verify does NOT require --db
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin(["velvet-ballastics", "verify", &workflow.to_string_lossy()]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("missing argument: --db"),
        "verify must NOT require --db, stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 3: Pure Mode — compile (POST-002)
// =============================================================================

#[test]
fn compile_produces_ir_without_storage() {
    // POST-002: compile does NOT open FjallJournal
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");
    let output_path = dir.path().join("out.vbir");

    let output = run_bin([
        "velvet-ballastics",
        "compile",
        &workflow.to_string_lossy(),
        "--emit",
        "ir",
        "--out",
        &output_path.to_string_lossy(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "compile must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "compile must produce output file");
    let meta = fs::metadata(&output_path).expect("output metadata");
    assert!(meta.len() > 0, "compile output must be non-empty");
}

// =============================================================================
// SECTION 4: Pure Mode — bench-run (POST-002)
// =============================================================================

#[test]
fn bench_run_executes_in_memory_without_storage() {
    // POST-002: bench-run does NOT call FjallJournal::open
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin([
        "velvet-ballastics",
        "bench-run",
        &workflow.to_string_lossy(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "bench-run must exit 0 on valid workflow, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compile:"),
        "output must have compile: timing"
    );
    assert!(
        stdout.contains("execute:"),
        "output must have execute: timing"
    );
}

#[test]
fn bench_run_does_not_fail_with_storage_error() {
    // INV-001: bench-run must NOT produce StorageError (5) even with --db
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin([
        "velvet-ballastics",
        "bench-run",
        &workflow.to_string_lossy(),
        "--db",
        "/tmp/nonexistent_journal_path",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("StorageError") || output.status.code() == Some(0),
        "bench-run must NOT fail with StorageError (5), stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 5: Pure Mode — agent-context (POST-002)
// =============================================================================

#[test]
fn agent_context_emits_cli_schema_without_storage() {
    // POST-002: agent-context is Pure (static JSON build)
    let output = run_bin(["velvet-ballastics", "agent-context"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "agent-context must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("schema_version"),
        "output must have schema_version"
    );
    assert!(
        stdout.contains("AgentContext"),
        "output must have AgentContext"
    );
}

// =============================================================================
// SECTION 6: Pure Mode — status (POST-002)
// =============================================================================

#[test]
fn status_reports_in_memory_without_storage() {
    // POST-002: status is Pure (transient Shard::new)
    let output = run_bin(["velvet-ballastics", "status"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "status must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("running:") || stdout.contains("shutting_down:"),
        "status output must have running/shutting_down, got: {stdout}"
    );
}

// =============================================================================
// SECTION 7: Pure Mode — graph (POST-002)
// =============================================================================

#[test]
fn graph_outputs_dot_without_storage() {
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin(["velvet-ballastics", "graph", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "graph must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph") || stdout.contains("graph"),
        "graph output must contain DOT digraph, got: {stdout}"
    );
}

// =============================================================================
// SECTION 8: Pure Mode — simulate (POST-002)
// =============================================================================

#[test]
fn simulate_dry_runs_without_storage() {
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let output = run_bin(["velvet-ballastics", "simulate", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "simulate must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// SECTION 9: Pure Mode — action list/inspect (POST-002)
// =============================================================================

#[test]
fn action_list_succeeds_without_storage() {
    let output = run_bin(["velvet-ballastics", "action", "list"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "action list must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn action_inspect_succeeds_without_storage() {
    let output = run_bin(["velvet-ballastics", "action", "inspect", "1"]);

    // Either success or "action not found" is acceptable — but not StorageError
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("StorageError"),
        "action inspect must NOT fail with StorageError (5), stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 10: Storage Mode — run with durability=none (POST-003)
// =============================================================================

#[test]
fn run_durability_none_skips_storage() {
    // POST-003: run with durability=none skips FjallJournal::open
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");
    let input_dir = temp_input_bin();
    let input = input_dir.path().join("input.bin");

    let output = run_bin([
        "velvet-ballastics",
        "run",
        &workflow.to_string_lossy(),
        "--input-bin",
        &input.to_string_lossy(),
        "--durability",
        "none",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error opening journal"),
        "run durability=none must NOT attempt journal open, stderr: {stderr}"
    );
    assert!(
        !stderr.contains("StorageError"),
        "run durability=none must NOT produce StorageError, stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 11: Storage Mode — inspect (POST-004, ERR-STORAGE-INIT)
// =============================================================================

#[test]
fn inspect_fails_fast_with_storage_error_on_invalid_path() {
    // POST-004: Mode activation is fail-fast before any subsystem init
    // ERR-STORAGE-INIT: Invalid --db path produces CliExitCode::StorageError (5)
    let output = run_bin([
        "velvet-ballastics",
        "inspect",
        "1",
        "--db",
        "/tmp/this/path/does/not/exist/by_definition_abc123",
    ]);

    assert_eq!(
        output.status.code(),
        Some(5),
        "inspect on invalid path must exit 5 (StorageError), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("journal") || stderr.contains("error opening"),
        "error must mention journal/path, stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 12: Storage Mode — doctor (POST-004)
// =============================================================================

#[test]
fn doctor_fails_fast_on_invalid_path() {
    let output = run_bin([
        "velvet-ballastics",
        "doctor",
        "--db",
        "/tmp/nonexistent_journal_path_xyz789",
    ]);

    assert_eq!(
        output.status.code(),
        Some(5),
        "doctor on invalid path must exit 5 (StorageError), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// SECTION 13: Storage Mode — submit (POST-003)
// =============================================================================

#[test]
fn submit_opens_fjall_journal() {
    // POST-003: submit opens FjallJournal regardless of durability
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");
    let input_dir = temp_input_bin();
    let input = input_dir.path().join("input.bin");
    let journal_dir = tempfile::tempdir().expect("journal dir");
    let journal = journal_dir.path();

    let output = run_bin([
        "velvet-ballastics",
        "submit",
        &workflow.to_string_lossy(),
        "--input-bin",
        &input.to_string_lossy(),
        "--db",
        &journal.to_string_lossy(),
        "--durability",
        "journaled",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "submit must exit 0 when journal opens, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// SECTION 14: Argument Parsing Errors (ERR-Taxonomy)
// =============================================================================

#[test]
fn unknown_command_exits_with_code_1_and_lists_valid_commands() {
    // ERR-Taxonomy: ParseError::UnknownCommand → exit 1 + valid command list
    let output = run_bin(["velvet-ballastics", "foobar"]);

    assert_eq!(output.status.code(), Some(1), "unknown command must exit 1");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("foobar") || stderr.contains("unknown command"),
        "error must mention unknown command, stderr: {stderr}"
    );
    assert!(
        stderr.contains("expected one of") || stderr.contains("valid commands"),
        "error must enumerate valid commands, stderr: {stderr}"
    );
}

#[test]
fn inspect_without_db_fails() {
    // parse_args requires --db for storage commands
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    // Running inspect without --db should fail
    let output = run_bin(["velvet-ballastics", "inspect", "1"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "inspect without --db must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--db") || stderr.contains("missing"),
        "error must mention missing --db, stderr: {stderr}"
    );
}

// =============================================================================
// SECTION 15: Exit Code Stability (POST-005)
// =============================================================================

#[test]
fn pure_commands_exit_independent_of_storage_availability() {
    // POST-005: Pure command exit code depends only on workflow validity,
    // not on storage/runtime availability
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    // validate should succeed even if /tmp/nonexistent/storage/path does not exist
    let output = run_bin(["velvet-ballastics", "validate", &workflow.to_string_lossy()]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "validate must exit 0 for valid workflow regardless of storage existence"
    );
}

// =============================================================================
// SECTION 16: Meta — All Pure Commands Classified Correctly (INV-001)
// =============================================================================

#[test]
fn all_pure_commands_do_not_produce_storage_error() {
    // INV-001: Pure commands must NOT produce StorageError (5)
    // This is a meta-test that verifies the mode classification is correct.
    let dir = temp_workflow(
        r#"
name: test
steps:
  - id: step1
    action: test-action
"#,
    );
    let workflow = dir.path().join("workflow.yaml");

    let workflow_path = workflow.to_string_lossy().into_owned();
    let pure_commands: Vec<(&str, Vec<&str>)> = vec![
        ("validate", vec!["validate", &workflow_path]),
        (
            "verify",
            vec!["verify", &workflow_path, "--profile", "quick"],
        ),
        ("explain", vec!["explain", &workflow_path]),
        (
            "compile",
            vec![
                "compile",
                &workflow_path,
                "--emit",
                "ir",
                "--out",
                "/tmp/out.vbir",
            ],
        ),
        ("graph", vec!["graph", &workflow_path]),
        ("simulate", vec!["simulate", &workflow_path]),
        ("bench-run", vec!["bench-run", &workflow_path]),
        ("agent-context", vec!["agent-context"]),
        ("status", vec!["status"]),
    ];

    for (name, mut args) in pure_commands {
        args.insert(0, "velvet-ballastics");
        let output = run_bin(args.into_iter().map(OsString::from).collect::<Vec<_>>());

        assert_ne!(
            output.status.code(),
            Some(5),
            "Pure command '{name}' must NOT exit with StorageError (5), stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
