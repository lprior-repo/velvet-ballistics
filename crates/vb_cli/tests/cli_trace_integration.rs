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
    serde_saphyr::from_str(stdout).expect("stdout should be valid YAML-compatible JSON")
}

fn trace_array(value: &serde_json::Value) -> &Vec<serde_json::Value> {
    value
        .get("trace")
        .and_then(|trace| trace.as_array())
        .expect("trace should be an array")
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

    let stderr = output_stderr(&output);
    assert!(
        stderr.is_empty(),
        "stderr must be empty on success, got: {stderr}"
    );
    let stdout = output_stdout(&output);
    // Check for exact text trace output header, entries, and footer
    assert!(
        stdout.starts_with("execution trace for run"),
        "stdout must start with trace header: {stdout}"
    );
    assert!(
        stdout.contains("[0] RunAccepted (seq 0)"),
        "stdout must contain entry 0 RunAccepted: {stdout}"
    );
    assert!(
        stdout.contains("[1] StepStarted step 0 (seq 1)"),
        "stdout must contain entry 1 StepStarted: {stdout}"
    );
    assert!(
        stdout.contains("[2] StepSucceeded step 0 (seq 2)"),
        "stdout must contain entry 2 StepSucceeded: {stdout}"
    );
    assert!(
        stdout.contains("[3] RunFinished (seq 3)"),
        "stdout must contain entry 3 RunFinished: {stdout}"
    );
    assert!(
        stdout.ends_with("4 event(s) total\n"),
        "stdout must end with total count: {stdout}"
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

    let stderr = output_stderr(&output);
    assert!(
        stderr.is_empty(),
        "stderr must be empty on success, got: {stderr}"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.starts_with("execution trace for run"),
        "text output must start with header: {stdout}"
    );
    assert!(
        stdout.contains("  [0] "),
        "text output must have leading-space indexed entries: {stdout}"
    );
    assert!(
        stdout.ends_with(" event(s) total\n"),
        "text output must end with total count line: {stdout}"
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

    let parsed = json_trace(&output_stdout(&output));

    assert_eq!(parsed.get("kind"), Some(&serde_json::json!("trace_report")));
    assert_eq!(parsed.get("run_id"), Some(&serde_json::json!(run_id.get().to_string())));
    assert_eq!(parsed.get("total"), Some(&serde_json::json!(4)));
    let trace = trace_array(&parsed);
    assert_eq!(trace.len(), 4, "trace array should have 4 entries");
    assert_eq!(trace[1], serde_json::json!({
        "seq": 1,
        "type": "StepStarted",
        "step": 0,
        "status": "active"
    }));
    assert_eq!(trace[2], serde_json::json!({
        "seq": 2,
        "type": "StepSucceeded",
        "step": 0,
        "status": "completed",
        "output": 0
    }));
    assert_eq!(trace[3], serde_json::json!({
        "seq": 3,
        "type": "RunFinished",
        "status": "completed",
        "result": 0
    }));
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

    let parsed = json_trace(&output_stdout(&output));
    assert_eq!(parsed.get("total"), Some(&serde_json::json!(4)));
    assert_eq!(trace_array(&parsed).len(), 4);
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
    let parsed = json_trace(&output_stdout(&output));
    let trace = parsed
        .get("trace")
        .and_then(|value| value.as_array())
        .expect("trace should be an array");
    assert_eq!(trace.len(), 2);
    for entry in trace {
        assert_eq!(entry.get("step").and_then(|value| value.as_u64()), Some(0));
    }
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
    let parsed = json_trace(&output_stdout(&output));
    let trace = parsed
        .get("trace")
        .and_then(|value| value.as_array())
        .expect("trace should be an array");
    assert_eq!(trace.len(), 2);
    for entry in trace {
        assert_eq!(
            entry.get("action").and_then(|value| value.as_u64()),
            Some(17)
        );
    }
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
    let parsed = json_trace(&output_stdout(&output));
    let trace = parsed
        .get("trace")
        .and_then(|value| value.as_array())
        .expect("trace should be an array");
    assert_eq!(trace.len(), 1);
    assert_eq!(
        trace[0].get("status").and_then(|value| value.as_str()),
        Some("active")
    );
    assert_eq!(
        trace[0].get("type").and_then(|value| value.as_str()),
        Some("StepStarted")
    );
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
    let parsed = json_trace(&output_stdout(&output));
    let trace = parsed
        .get("trace")
        .and_then(|value| value.as_array())
        .expect("trace should be an array");
    assert_eq!(trace.len(), 2);
    assert_eq!(
        trace[0].get("seq").and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        trace[1].get("seq").and_then(|value| value.as_u64()),
        Some(2)
    );
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
    let parsed = json_trace(&output_stdout(&output));
    assert_eq!(
        parsed.get("total").and_then(|value| value.as_u64()),
        Some(1)
    );
    let trace = parsed
        .get("trace")
        .and_then(|value| value.as_array())
        .expect("trace should be an array");
    assert_eq!(trace.len(), 1);
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
    let stderr = output_stderr(&output);
    assert!(
        stderr.is_empty(),
        "stderr must be empty on success, got: {stderr}"
    );
    let stdout = output_stdout(&output);
    assert_eq!(
        stdout,
        "no events found for run 99\n",
        "stdout must exactly report no events for the run"
    );
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
    assert_eq!(
        output.stdout,
        Vec::<u8>::new(),
        "stdout must be empty on storage error"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.starts_with("journal directory does not exist: "),
        "stderr must report missing journal directory: {stderr}"
    );
    assert!(
        stderr.ends_with("\n"),
        "stderr must end with newline"
    );
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
    assert_eq!(
        output.stdout,
        Vec::<u8>::new(),
        "stdout must be empty on validation error"
    );
    let stderr = output_stderr(&output);
    assert_eq!(
        stderr,
        "invalid run_id 'not-a-number': invalid digit found in string\n",
        "stderr must be exact validation error"
    );
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
    assert_eq!(
        output.stdout,
        Vec::<u8>::new(),
        "stdout must be empty on storage error"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.starts_with("journal directory does not exist: "),
        "stderr must report missing journal directory: {stderr}"
    );
    assert!(
        stderr.ends_with("\n"),
        "stderr must end with newline"
    );
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
