#![forbid(unsafe_code)]
//! BDD integration tests for vb-a2zn: absent-run exit code normalization.
//!
//! These tests create a temporary FjallJournal with events for one run,
//! then query for a different run that has zero events. They verify that
//! each CLI read command returns exit code 2 (VerificationFailed) for
//! absent runs, consistent across all 7 affected commands.

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
/// Returns a guard holding the temp dir and db path.
fn setup_journal_with_run_events(run_id: u64) -> JournalGuard {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("test_db");

    let journal = FjallJournal::open(&db_path, None).expect("open journal");
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(run_id),
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        },
        JournalEvent::StepStarted {
            run: RunId::new(run_id),
            seq: EventSeq::new(1),
            step: vb_core::StepIdx::new(0),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("append strict batch");

    JournalGuard {
        _temp_dir: temp_dir,
        db_path,
    }
}

// =========================================================================
// Phase 4.2.1: events command BDD tests
// =========================================================================

/// BDD-EVT-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_evt_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["events", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "events for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-EVT-2: Run 9999 has no events; --json → exit code 2 with error JSON.
#[test]
fn bdd_evt_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "events",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(exit_code, 2, "should return exit code 2 for absent run");

    // Error JSON is on stderr (as per existing patterns in the codebase)
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain either JSON error or at least a no-events message
    assert!(
        stderr.contains("no events") || stderr.contains("NoEvents"),
        "stderr should mention no events, got: {stderr}"
    );
}

/// BDD-EVT-3: Run 9999 with --jsonl → exit code 2.
#[test]
fn bdd_evt_3_absent_run_jsonl_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "events",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--jsonl",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "should return exit code 2 for absent run (jsonl)"
    );
}

/// BDD-EVT-6: Regression — run with actual events returns exit code 0.
#[test]
fn bdd_evt_6_present_run_regression() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["events", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 0,
        "events for present run should return exit code 0, got {}",
        exit_code
    );
}

// =========================================================================
// Phase 4.2.2: inspect command BDD tests
// =========================================================================

/// BDD-INS-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_ins_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["inspect", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "inspect for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-INS-2: Run 9999 has no events; --json → exit code 2.
#[test]
fn bdd_ins_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "inspect",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(exit_code, 2, "inspect absent run should return exit code 2");
}

/// BDD-INS-3: Regression — run with actual events returns exit code 0.
#[test]
fn bdd_ins_3_present_run_regression() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["inspect", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 0,
        "inspect for present run should return exit code 0, got {}",
        exit_code
    );
}

// =========================================================================
// Phase 4.2.3: replay command BDD tests
// =========================================================================

/// BDD-RPL-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_rpl_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["replay", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "replay for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-RPL-2: Run 9999 has no events; --json → exit code 2.
#[test]
fn bdd_rpl_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "replay",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(exit_code, 2, "replay absent run should return exit code 2");
}

/// BDD-RPL-4: Regression — run with actual events returns exit code 0.
#[test]
fn bdd_rpl_4_present_run_regression() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["replay", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 0,
        "replay for present run should return exit code 0, got {}",
        exit_code
    );
}

// =========================================================================
// Phase 4.2.4: trace command BDD tests
// =========================================================================

/// BDD-TRC-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_trc_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["trace", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "trace for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-TRC-3: Regression — run with actual events returns exit code 0.
#[test]
fn bdd_trc_3_present_run_regression() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["trace", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 0,
        "trace for present run should return exit code 0, got {}",
        exit_code
    );
}

// =========================================================================
// Phase 4.2.5: retry command BDD tests
// =========================================================================

/// BDD-RTY-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_rty_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["retry", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "retry for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-RTY-2: Run 9999 has no events; --json → exit code 2.
#[test]
fn bdd_rty_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["retry", "9999", "--db", db_path.to_str().unwrap(), "--json"]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(exit_code, 2, "retry absent run should return exit code 2");
}

// =========================================================================
// Phase 4.2.6: resume command BDD tests
// =========================================================================

/// BDD-RSM-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_rsm_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["resume", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "resume for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-RSM-2: Run 9999 has no events; --json → exit code 2.
#[test]
fn bdd_rsm_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "resume",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(exit_code, 2, "resume absent run should return exit code 2");
}

// =========================================================================
// Phase 4.2.7: diff command BDD tests
// =========================================================================

/// BDD-DFF-1: Both runs absent; text output → exit code 2.
#[test]
fn bdd_dff_1_both_runs_absent() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["diff", "9999", "9998", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "diff with both runs absent should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-DFF-2: Run A absent, run B present; text output → exit code 2.
#[test]
fn bdd_dff_2_run_a_absent() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["diff", "9999", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "diff with run A absent should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-DFF-3: Run A present, run B absent; text output → exit code 2.
#[test]
fn bdd_dff_3_run_b_absent() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["diff", "42", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "diff with run B absent should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-DFF-5: Regression — both runs present, identical → exit code 0.
#[test]
fn bdd_dff_5_both_runs_present_identical() {
    let guard = setup_journal_with_run_events(42);
    let db_path = guard.path();

    let args = make_args(&["diff", "42", "42", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 0,
        "diff with identical runs should return exit code 0, got {}",
        exit_code
    );
}

// =========================================================================
// Phase 4.2.8: incident command BDD tests
// =========================================================================

/// BDD-INC-1: Run 9999 has no events; text output → exit code 2.
#[test]
fn bdd_inc_1_absent_run_text_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["incident", "9999", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "incident for absent run should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-INC-2: Run 9999 has no events; --json → exit code 2.
#[test]
fn bdd_inc_2_absent_run_json_output() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "9999",
        "--db",
        db_path.to_str().unwrap(),
        "--json",
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "incident absent run should return exit code 2"
    );
}

// =========================================================================
// Phase 4.3: Cross-cutting BDD tests
// =========================================================================

/// BDD-CROSS-1: All absent-run commands return exactly exit code 2.
#[test]
fn bdd_cross_1_all_commands_return_exit_code_2() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let commands = [
        "events", "inspect", "replay", "trace", "retry", "resume", "diff", "incident",
    ];

    for &cmd in &commands {
        let args = make_args(&[cmd, "9999", "--db", db_path.to_str().unwrap()]);
        let output = run_cli(args);

        let exit_code = output.status.code().expect("exit code should be available");
        assert_eq!(
            exit_code, 2,
            "command '{}' for absent run should return exit code 2, got {}",
            cmd, exit_code
        );
    }
}

/// BDD-CROSS-4: Large run ID (u64::MAX) absent → exit code 2.
#[test]
fn bdd_cross_4_large_run_id_absent() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&[
        "events",
        "18446744073709551615",
        "--db",
        db_path.to_str().unwrap(),
    ]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "large run ID absent should return exit code 2, got {}",
        exit_code
    );
}

/// BDD-CROSS-5: Run ID 0 absent → exit code 2.
#[test]
fn bdd_cross_5_run_id_zero_absent() {
    let guard = setup_journal_with_run_events(1);
    let db_path = guard.path();

    let args = make_args(&["events", "0", "--db", db_path.to_str().unwrap()]);
    let output = run_cli(args);

    let exit_code = output.status.code().expect("exit code should be available");
    assert_eq!(
        exit_code, 2,
        "run ID 0 absent should return exit code 2, got {}",
        exit_code
    );
}
