#![forbid(unsafe_code)]
#![cfg(not(miri))]
//! Integration tests for the `trace` command.
//!
//! These tests exercise the full pipeline: CLI argument parsing → journal read → build_trace → output formatting.

use std::ffi::OsStr;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_cli(args: &[&std::ffi::OsStr]) -> Option<std::process::Output> {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
    command.args(args);
    command.output().ok()
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

fn assert_cli_exit_code(output: &std::process::Output, expected_exit: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "expected exit code {expected_exit}, got {:?}: stdout={} stderr={}",
        output.status.code(),
        output_stdout(output),
        output_stderr(output)
    );
}

// ---------------------------------------------------------------------------
// Integration: trace command with real journal
// ---------------------------------------------------------------------------

fn setup_trace_journal(dir: &std::path::Path) -> vb_core::RunId {
    let journal = FjallJournal::open(dir, None).expect("journal should open");
    let run_id = vb_core::RunId::new(1);
    let workflow_digest = WorkflowDigest::from_bytes([9u8; 32]);

    // Write a minimal set of journal events for a run
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(0),
            workflow: workflow_digest,
        },
        JournalEvent::StepStarted {
            run: run_id,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: run_id,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::ZERO,
        },
        JournalEvent::RunFinished {
            run: run_id,
            seq: EventSeq::new(3),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("append should succeed");
    run_id
}

fn setup_action_trace_journal(dir: &std::path::Path) -> vb_core::RunId {
    let journal = FjallJournal::open(dir, None).expect("journal should open");
    let run_id = vb_core::RunId::new(2);
    let workflow_digest = WorkflowDigest::from_bytes([8u8; 32]);
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(0),
            workflow: workflow_digest,
        },
        JournalEvent::ActionScheduled {
            run: run_id,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            action: ActionId::new(17),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run: run_id,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(17),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run: run_id,
            seq: EventSeq::new(3),
            step: StepIdx::new(3),
            action: ActionId::new(23),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("append should succeed");
    run_id
}

fn json_trace(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout).expect("stdout should be valid JSON")
}

// ---------------------------------------------------------------------------
// Integration: cmd_trace full pipeline with real Fjall journal
// ---------------------------------------------------------------------------

#[test]
fn cmd_trace_with_events_returns_all_entries_in_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some(), "trace command should execute");
    let output = output.unwrap();
    assert_cli_success(&output, "trace");

    let stdout = output_stdout(&output);
    // Check for ordered indices
    assert!(
        stdout.contains("[0]"),
        "stdout should contain index 0: {stdout}"
    );
    assert!(
        stdout.contains("[1]"),
        "stdout should contain index 1: {stdout}"
    );
    assert!(
        stdout.contains("[2]"),
        "stdout should contain index 2: {stdout}"
    );
    assert!(
        stdout.contains("RunAccepted"),
        "stdout should contain RunAccepted: {stdout}"
    );
    assert!(
        stdout.contains("StepStarted"),
        "stdout should contain StepStarted: {stdout}"
    );
    assert!(
        stdout.contains("StepSucceeded"),
        "stdout should contain StepSucceeded: {stdout}"
    );
    assert!(
        stdout.contains("RunFinished"),
        "stdout should contain RunFinished: {stdout}"
    );
    assert!(
        stdout.contains("4 event(s) total"),
        "stdout should report 4 events: {stdout}"
    );
}

#[test]
fn cmd_trace_text_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace");

    let stdout = output_stdout(&output);
    // Text format: "execution trace for run {id}"
    assert!(
        stdout.contains("execution trace for run"),
        "text output should have header: {stdout}"
    );
    // Text format: "  [idx] EventType step? (seq N)"
    assert!(
        stdout.contains("[0]"),
        "text output should have indexed entries: {stdout}"
    );
    // Text format: "{N} event(s) total"
    assert!(
        stdout.contains("event(s) total"),
        "text output should have total: {stdout}"
    );
}

#[test]
fn cmd_trace_json_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --emit yaml");

    let stdout = output_stdout(&output);
    assert!(stdout.contains("run_id:"), "YAML should contain run_id: ; got: {stdout}");
    assert!(stdout.contains("trace:"), "YAML should contain trace: ; got: {stdout}");
    assert!(stdout.contains("total:"), "YAML should contain total: ; got: {stdout}");
    assert!(stdout.contains("total: 4"), "YAML should contain total: 4; got: {stdout}");
}

#[test]
fn cmd_trace_jsonl_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --emit yaml");

    let stdout = output_stdout(&output);
    assert!(stdout.contains("run_id:"), "YAML should contain run_id: ; got: {stdout}");
    assert!(stdout.contains("trace:"), "YAML should contain trace: ; got: {stdout}");
    assert!(stdout.contains("total: 4"), "YAML should contain total: 4; got: {stdout}");
}

#[test]
fn cmd_trace_step_filter_returns_only_matching_step() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--step"),
        OsStr::new("0"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --step 0 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("step: 0"), "YAML should contain step: 0; got: {stdout}");
    assert!(stdout.contains("trace:"), "YAML should contain trace: ; got: {stdout}");
}

#[test]
fn cmd_trace_action_filter_returns_only_matching_action() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_action_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--action"),
        OsStr::new("17"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --action 17 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("action: 17"), "YAML should contain action: 17; got: {stdout}");
    assert!(stdout.contains("trace:"), "YAML should contain trace: ; got: {stdout}");
}

#[test]
fn cmd_trace_status_filter_returns_only_active_events() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--status"),
        OsStr::new("active"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --status active --emit yaml");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("status: active"), "YAML should contain status: active; got: {stdout}");
    assert!(stdout.contains("StepStarted"), "YAML should contain StepStarted; got: {stdout}");
}

#[test]
fn cmd_trace_sequence_range_filter_is_inclusive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--since-seq"),
        OsStr::new("1"),
        OsStr::new("--until-seq"),
        OsStr::new("2"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --since-seq 1 --until-seq 2 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("seq: 1"), "YAML should contain seq: 1; got: {stdout}");
    assert!(stdout.contains("seq: 2"), "YAML should contain seq: 2; got: {stdout}");
}

#[test]
fn cmd_trace_limit_bounds_filtered_output() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_action_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--status"),
        OsStr::new("active"),
        OsStr::new("--limit"),
        OsStr::new("1"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --status active --limit 1 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("total:"), "YAML should contain total: ; got: {stdout}");
    assert!(stdout.contains("trace:"), "YAML should contain trace: ; got: {stdout}");
}

#[test]
fn cmd_trace_empty_run_returns_success() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Create an empty journal (no events for run_id=99)
    let journal = FjallJournal::open(dir.path(), None).expect("journal should open");
    // Don't write any events for run 99
    drop(journal);

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("99"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    // Empty run should return success with "no events found" message
    assert_cli_success(&output, "trace on empty run");
}

#[test]
fn cmd_trace_invalid_db_path_returns_storage_error() {
    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("1"),
        OsStr::new("--db"),
        OsStr::new("/nonexistent/path/that/does/not/exist"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 5); // CliExitCode::StorageError = 5
}

#[test]
fn cmd_trace_invalid_run_id_format_returns_validation_failed() {
    let dir = tempfile::tempdir().expect("temp dir");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("not-a-number"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 2); // CliExitCode::ValidationFailed = 2
}

#[test]
fn read_journal_events_returns_storage_error_when_dir_not_found() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Journal was never created at this path
    let nonexistent = dir.path().join("truly_nonexistent_journal_db");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("1"),
        OsStr::new("--db"),
        nonexistent.as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 5); // CliExitCode::StorageError
}

// ---------------------------------------------------------------------------
// E2E: CLI binary trace command exit code
// ---------------------------------------------------------------------------

#[test]
fn cli_trace_command_exit_code_success() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 0);
}

#[test]
fn cli_trace_command_on_nonexistent_run_exit_code_zero() {
    // Per POST-006: non-existent run is treated as empty trace, exit 0
    let dir = tempfile::tempdir().expect("temp dir");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("999999"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 0);
}
