#![forbid(unsafe_code)]
//! Integration tests for the `incident` command — vb-qi37.17.1.
//!
//! These tests create a temporary journal, populate it with events, and
//! invoke the velvet-ballastics CLI binary to verify end-to-end behavior.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use vb_core::RunId;
use vb_storage::EventSeq;
use vb_storage::FjallJournal;
use vb_storage::events::JournalEvent;

/// A guard that holds a temp directory and the path to a journal inside it.
struct JournalGuard {
    _temp_dir: tempfile::TempDir,
    db_path: PathBuf,
}

impl JournalGuard {
    fn path(&self) -> &PathBuf {
        &self.db_path
    }
}

/// Helper: run the velvet-ballastics binary with the given arguments.
fn run_cli(args: Vec<OsString>) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_velvet-ballastics");
    let output = Command::new(exe).args(args).output().expect("cli must run");
    output
}

/// Helper to build OsString args from str parts.
fn make_args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|s| OsString::from(s)).collect()
}

/// Create a temporary FjallJournal and append events to it.
fn setup_test_journal(events: &[JournalEvent]) -> JournalGuard {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("test_db");

    let journal = FjallJournal::open(&db_path, None).expect("open journal");
    journal
        .append_strict_batch(events)
        .expect("append strict batch");

    JournalGuard {
        _temp_dir: temp_dir,
        db_path,
    }
}

/// Build a minimal set of events for a failed run.
fn failed_run_events() -> Vec<JournalEvent> {
    vec![
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            step: vb_core::ids::StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(1),
            step: vb_core::ids::StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run: RunId::new(42),
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ]
}

/// Build events for a successful run.
fn successful_run_events() -> Vec<JournalEvent> {
    vec![
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            step: vb_core::ids::StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: RunId::new(42),
            seq: EventSeq::new(1),
            result: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
    ]
}

// ---------------------------------------------------------------------------
// T-014: Failed run → JSON output
// ---------------------------------------------------------------------------

#[test]
fn t_014_failed_run_json_output() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "incident should succeed: status={:?} stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    // Verify JSON structure
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["run_id"], "42");
    assert_eq!(json["failure_code"], "RunFailed");
}

// ---------------------------------------------------------------------------
// T-015: Non-existent run → structured error on stderr
// ---------------------------------------------------------------------------

#[test]
fn t_015_nonexistent_run_structured_error() {
    let guard = setup_test_journal(&successful_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "99999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    // Error JSON is written to stderr by json_error() as a DiagnosticReport.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code().map(|c| c != 0).unwrap_or(true),
        "non-existent run should return non-zero exit"
    );
    let json: serde_json::Value = serde_json::from_str(&stderr).expect("valid JSON on stderr");
    assert_eq!(json["code"], "ValidationFailed");
    assert_eq!(json["kind"], "DiagnosticReport");
    assert!(
        json["message"].as_str().unwrap_or("").contains("no events"),
        "message should mention no events"
    );
    // POST-003 / INV-002: no stack traces in error output
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr_str.to_lowercase().contains("backtrace"),
        "error output must not contain stack traces"
    );
    assert!(
        !stderr_str.contains("at crates/"),
        "error output must not contain source location traces"
    );
}

// ---------------------------------------------------------------------------
// T-016: Successful run → no failure fields populated
// ---------------------------------------------------------------------------

#[test]
fn t_016_successful_run_not_incident() {
    let guard = setup_test_journal(&successful_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    // The JSON report does not include a top-level "failure_found" field;
    // absence of failure indicators is the signal.
    assert_eq!(json["failure_code"].as_str(), Some(""));
    assert_eq!(json["failed_at_step"], serde_json::Value::Null);
    // POST-004: non-failed run should return StorageError (exit code 5)
    assert_eq!(
        output.status.code(),
        Some(5),
        "non-failed run should return StorageError"
    );
}

// ---------------------------------------------------------------------------
// T-017: Text output format
// ---------------------------------------------------------------------------

#[test]
fn t_017_text_output_format() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--text",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("incident report for run"));
    assert!(stdout.contains("RunFailed"));
}

// ---------------------------------------------------------------------------
// T-018: JSONL output format
// ---------------------------------------------------------------------------

#[test]
fn t_018_jsonl_output_format() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--jsonl",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout.trim()).expect("valid JSON");
    assert_eq!(json["failure_code"], "RunFailed");
}
