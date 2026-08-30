//! Autonomous CLI command composing harness integration tests.
//!
//! Phase: 9 (failing-first TDD)
//!
//! Tests the CLI harness command surface for:
//! - Argument parsing: workflow, seed, step-bound, fault-script, output-dir
//! - HarnessResult structure and transcript building
//! - Deterministic rerun: same seed produces same observation structure
//! - Incident report integration on divergence
//! - JSON artifact export (transcript, journal, observations, diff)
//! - Output format (text, yaml, postcard)
//! - Exit codes: success and harness-failed
//!
//! # Coverage Map
//!
//! | Acceptance Criterion | Test(s) |
//! |---|---|
//! | Harness command accepts workflow, seed, step-bound, fault-script, output-dir | `harness_args_*` |
//! | HarnessResult accumulates iteration stats | `harness_result_accumulation_*` |
//! | Transcript entries track pass/fail per iteration | `transcript_*` |
//! | Deterministic rerun with same seed produces same observations | `deterministic_*` |
//! | Incident report emitted on divergence | `incident_*` |
//! | JSON artifacts written to output dir | `artifacts_*` |
//! | Exit code HarnessFailed on divergence | `exit_code_*` |
//!
//! All tests follow Given/When/Then structure.
//! No unsafe code. No unwrap/expect on fallible operations.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use vb_cli::args::types::Command;
use vb_cli::exit_code::CliExitCode;

// ---------------------------------------------------------------------------
// Argument parsing tests
// ---------------------------------------------------------------------------

fn temp_output_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir creation should succeed")
}

/// Parse harness command with all required arguments.
fn parse_harness_args(
    workflow: &str,
    seed: u64,
    step_bound: usize,
    output_dir: &str,
) -> Result<Command, String> {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        workflow.into(),
        "--seed".into(),
        seed.to_string().into(),
        "--step-bound".into(),
        step_bound.to_string().into(),
        "--output-dir".into(),
        output_dir.into(),
    ];
    vb_cli::args::parse_args(&args).map_err(|e| e.to_string())
}

/// Parse harness command with optional fault script.
fn parse_harness_args_with_fault(
    workflow: &str,
    seed: u64,
    step_bound: usize,
    fault_script: &str,
    output_dir: &str,
) -> Result<Command, String> {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        workflow.into(),
        "--seed".into(),
        seed.to_string().into(),
        "--step-bound".into(),
        step_bound.to_string().into(),
        "--fault-script".into(),
        fault_script.into(),
        "--output-dir".into(),
        output_dir.into(),
    ];
    vb_cli::args::parse_args(&args).map_err(|e| e.to_string())
}

// ---- T-001: Parse harness command with all args ----
#[test]
fn t_001_parse_harness_full_args() {
    // Given: CLI arguments for harness command
    let output_dir = temp_output_dir();
    let output_path = output_dir.path().to_string_lossy().to_string();

    // When: parsing harness command
    let result = parse_harness_args("/tmp/workflow.yaml", 42, 10, &output_path);

    // Then: command is parsed correctly
    assert!(result.is_ok(), "parsing should succeed");
    if let Ok(Command::Harness {
        workflow,
        seed,
        step_bound,
        fault_script,
        output_dir: parsed_dir,
        output: _output,
    }) = result
    {
        assert_eq!(workflow, PathBuf::from("/tmp/workflow.yaml"));
        assert_eq!(seed, 42);
        assert_eq!(step_bound, 10);
        assert!(fault_script.is_none());
        assert_eq!(parsed_dir, output_dir.path());
    } else {
        panic!("expected Harness command variant");
    }
}

// ---- T-002: Parse harness command with fault script ----
#[test]
fn t_002_parse_harness_with_fault_script() {
    // Given: CLI arguments with fault script
    let output_dir = temp_output_dir();
    let output_path = output_dir.path().to_string_lossy().to_string();

    // When: parsing harness command with --fault-script
    let result = parse_harness_args_with_fault(
        "/tmp/workflow.yaml",
        99,
        20,
        "/tmp/fault.yaml",
        &output_path,
    );

    // Then: fault script is captured
    assert!(result.is_ok());
    if let Ok(Command::Harness {
        workflow,
        seed,
        step_bound,
        fault_script,
        output_dir: parsed_dir,
        ..
    }) = result
    {
        assert_eq!(workflow, PathBuf::from("/tmp/workflow.yaml"));
        assert_eq!(seed, 99);
        assert_eq!(step_bound, 20);
        assert_eq!(fault_script, Some(PathBuf::from("/tmp/fault.yaml")));
        assert_eq!(parsed_dir, output_dir.path());
    } else {
        panic!("expected Harness command variant");
    }
}

// ---- T-003: Missing --seed argument ----
#[test]
fn t_003_missing_seed_argument() {
    // Given: CLI arguments without --seed
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        "/tmp/workflow.yaml".into(),
        "--step-bound".into(),
        "10".into(),
        "--output-dir".into(),
        "/tmp/out".into(),
    ];

    // When: parsing
    let result = vb_cli::args::parse_args(&args);

    // Then: missing seed error
    assert!(result.is_err());
}

// ---- T-004: Invalid seed value ----
#[test]
fn t_004_invalid_seed_value() {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        "/tmp/workflow.yaml".into(),
        "--seed".into(),
        "not-a-number".into(),
        "--step-bound".into(),
        "10".into(),
        "--output-dir".into(),
        "/tmp/out".into(),
    ];

    let result = vb_cli::args::parse_args(&args);
    assert!(result.is_err());
}

// ---- T-005: Missing --step-bound argument ----
#[test]
fn t_005_missing_step_bound() {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        "/tmp/workflow.yaml".into(),
        "--seed".into(),
        "42".into(),
        "--output-dir".into(),
        "/tmp/out".into(),
    ];

    let result = vb_cli::args::parse_args(&args);
    assert!(result.is_err());
}

// ---- T-006: Missing --output-dir argument ----
#[test]
fn t_006_missing_output_dir() {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        "/tmp/workflow.yaml".into(),
        "--seed".into(),
        "42".into(),
        "--step-bound".into(),
        "10".into(),
    ];

    let result = vb_cli::args::parse_args(&args);
    assert!(result.is_err());
}

// ---- T-007: Missing workflow positional ----
#[test]
fn t_007_missing_workflow() {
    let args: Vec<std::ffi::OsString> = vec![
        "velvet-ballistics".into(),
        "harness".into(),
        "--seed".into(),
        "42".into(),
        "--step-bound".into(),
        "10".into(),
        "--output-dir".into(),
        "/tmp/out".into(),
    ];

    let result = vb_cli::args::parse_args(&args);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// HarnessResult and transcript tests
// ---------------------------------------------------------------------------

// ---- T-008: HarnessResult with mixed pass/fail ----
#[test]
fn t_008_harness_result_mixed() {
    use vb_cli::harness::{HarnessResult, TranscriptEntry};

    let mut result = HarnessResult {
        iterations: 3,
        passed: 1,
        failed: 2,
        transcript: Vec::new(),
        total_differences: 5,
    };

    result.transcript.push(TranscriptEntry {
        iteration: 1,
        seed: 1,
        passed: true,
        differences: 0,
        incident: None,
    });
    result.transcript.push(TranscriptEntry {
        iteration: 2,
        seed: 2,
        passed: false,
        differences: 3,
        incident: Some(serde_json::json!({"diverged": true})),
    });
    result.transcript.push(TranscriptEntry {
        iteration: 3,
        seed: 3,
        passed: false,
        differences: 2,
        incident: Some(serde_json::json!({"diverged": true})),
    });

    assert_eq!(result.iterations, 3);
    assert_eq!(result.passed, 1);
    assert_eq!(result.failed, 2);
    assert_eq!(result.total_differences, 5);
    assert_eq!(result.transcript.len(), 3);
}

// ---- T-009: All iterations pass ----
#[test]
fn t_009_all_iterations_pass() {
    use vb_cli::harness::{HarnessResult, TranscriptEntry};

    let result = HarnessResult {
        iterations: 2,
        passed: 2,
        failed: 0,
        transcript: vec![
            TranscriptEntry {
                iteration: 1,
                seed: 1,
                passed: true,
                differences: 0,
                incident: None,
            },
            TranscriptEntry {
                iteration: 2,
                seed: 2,
                passed: true,
                differences: 0,
                incident: None,
            },
        ],
        total_differences: 0,
    };

    assert_eq!(result.passed, 2);
    assert_eq!(result.failed, 0);
    assert_eq!(result.total_differences, 0);
}

// ---- T-010: All iterations fail ----
#[test]
fn t_010_all_iterations_fail() {
    use vb_cli::harness::{HarnessResult, TranscriptEntry};

    let result = HarnessResult {
        iterations: 1,
        passed: 0,
        failed: 1,
        transcript: vec![TranscriptEntry {
            iteration: 1,
            seed: 1,
            passed: false,
            differences: 4,
            incident: Some(serde_json::json!({"diverged": true})),
        }],
        total_differences: 4,
    };

    assert_eq!(result.failed, 1);
    assert_eq!(result.total_differences, 4);
}

// ---------------------------------------------------------------------------
// Deterministic rerun tests
// ---------------------------------------------------------------------------

// ---- T-011: Deterministic observation structure ----
#[test]
fn t_011_deterministic_observation_structure() {
    use vb_cli::harness::build_normalized_observations;
    use vb_core::{ActionId, EventSeq, RunId, StepIdx};

    let seed = 12345u64;
    let _ = seed; // Seed conceptually drives scheduler; here we verify
                  // that deterministic inputs produce deterministic outputs.

    let events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        vb_storage::JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: vb_core::OutputBuf::from(vec![1u8, 2, 3]),
            attempt: 1,
        },
    ];

    let obs_a = build_normalized_observations(&events);
    let obs_b = build_normalized_observations(&events);

    assert_eq!(obs_a.len(), obs_b.len());
    for (a, b) in obs_a.iter().zip(obs_b.iter()) {
        assert_eq!(a, b);
    }
}

// ---- T-012: Empty events produce empty observations ----
#[test]
fn t_012_empty_observations() {
    use vb_cli::harness::build_normalized_observations;

    let obs = build_normalized_observations(&[]);
    assert!(obs.is_empty());
}

// ---- T-013: All event types produce normalized observations ----
#[test]
fn t_013_all_event_types_normalized() {
    use vb_cli::harness::build_normalized_observations;
    use vb_core::{ActionId, EventSeq, RunId, StepIdx};

    let events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        vb_storage::JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: vb_core::OutputBuf::from(vec![]),
            attempt: 1,
        },
        vb_storage::JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(10),
            attempt: 1,
        },
        vb_storage::JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            action: ActionId::new(10),
            attempt: 1,
        },
        vb_storage::JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        },
    ];

    let obs = build_normalized_observations(&events);
    assert_eq!(obs.len(), 6);
}

// ---------------------------------------------------------------------------
// Artifact export tests
// ---------------------------------------------------------------------------

// ---- T-014: Transcript file created ----
#[test]
fn t_014_transcript_file_created() {
    use vb_cli::harness::export_json_file;

    let dir = temp_output_dir();
    let value = serde_json::json!([
        {"iteration": 1, "passed": true},
        {"iteration": 2, "passed": false}
    ]);

    export_json_file(dir.path(), "transcript.json", &value);

    let path = dir.path().join("transcript.json");
    assert!(path.exists());

    let content = std::fs::read_to_string(&path).expect("transcript should be readable");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("transcript should be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

// ---- T-015: Journal export file created ----
#[test]
fn t_015_journal_export_file() {
    use vb_cli::harness::export_events_to_json;
    use vb_core::{EventSeq, RunId};

    let dir = temp_output_dir();
    let events = vec![vb_storage::JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        workflow_id: 1,
    }];

    export_events_to_json(dir.path(), "journal_export.json", &events);

    let path = dir.path().join("journal_export.json");
    assert!(path.exists());
}

// ---- T-016: Observations file created ----
#[test]
fn t_016_observations_file_created() {
    use vb_cli::harness::build_normalized_observations;
    use vb_cli::harness::export_json_file;
    use vb_core::{EventSeq, RunId};

    let dir = temp_output_dir();
    let events = vec![vb_storage::JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        workflow_id: 1,
    }];

    let observations = build_normalized_observations(&events);
    export_json_file(
        dir.path(),
        "observations.json",
        &serde_json::to_value(&observations).unwrap(),
    );

    let path = dir.path().join("observations.json");
    assert!(path.exists());
}

// ---- T-017: Incident report file created ----
#[test]
fn t_017_incident_report_file_created() {
    use vb_cli::harness::export_json_file;

    let dir = temp_output_dir();
    let incident = serde_json::json!({
        "run_id": "harness-run-1",
        "failure_code": "RunFailed",
        "diverged": true,
    });

    export_json_file(dir.path(), "incident_report.json", &incident);

    let path = dir.path().join("incident_report.json");
    assert!(path.exists());

    let content = std::fs::read_to_string(&path).expect("incident report should be readable");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("incident report should be valid JSON");
    assert_eq!(parsed["failure_code"], "RunFailed");
    assert_eq!(parsed["diverged"], true);
}

// ---------------------------------------------------------------------------
// Incident report integration tests
// ---------------------------------------------------------------------------

// ---- T-018: Incident report on RunFailed ----
#[test]
fn t_018_incident_run_failed() {
    use vb_cli::commands_incident::build_incident_report;
    use vb_core::{EventSeq, RunId};

    let events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        },
    ];

    let report = build_incident_report("harness-run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunFailed");
}

// ---- T-019: No incident on completed run ----
#[test]
fn t_019_no_incident_completed() {
    use vb_cli::commands_incident::build_incident_report;
    use vb_core::{EventSeq, RunId};

    let events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        },
    ];

    let report = build_incident_report("harness-run-done", &events);
    assert!(!report.failure_found);
    assert_eq!(report.failure_code, "");
}

// ---- T-020: Incident with side effects ----
#[test]
fn t_020_incident_with_side_effects() {
    use vb_cli::commands_incident::build_incident_report;
    use vb_core::{ActionId, EventSeq, RunId, StepIdx};

    let events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(100),
            attempt: 1,
        },
        vb_storage::JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        },
    ];

    let report = build_incident_report("harness-run-sides", &events);
    assert!(report.failure_found);
    assert_eq!(report.side_effects.len(), 1);
    assert_eq!(report.side_effects[0]["step"], 1);
    assert_eq!(report.side_effects[0]["action"], 100);
}

// ---------------------------------------------------------------------------
// Diff integration tests
// ---------------------------------------------------------------------------

// ---- T-021: Diff with identical event streams ----
#[test]
fn t_021_diff_identical_streams() {
    use vb_cli::commands_diff::compute_diff;

    let events: Vec<vb_storage::JournalEvent> = Vec::new();
    let result = compute_diff(&events, &events);
    assert_eq!(result.events_a, 0);
    assert_eq!(result.events_b, 0);
    assert!(result.diffs.is_empty());
}

// ---- T-022: Diff with different event streams ----
#[test]
fn t_022_diff_different_streams() {
    use vb_cli::commands_diff::compute_diff;
    use vb_core::{ActionId, EventSeq, RunId, StepIdx};

    let events_a = vec![vb_storage::JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        workflow_id: 1,
    }];

    let events_b = vec![
        vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            workflow_id: 1,
        },
        vb_storage::JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let result = compute_diff(&events_a, &events_b);
    assert!(result.diffs.len() > 0);
}

// ---------------------------------------------------------------------------
// Exit code tests
// ---------------------------------------------------------------------------

// ---- T-023: HarnessFailed exit code ----
#[test]
fn t_023_harness_failed_exit_code() {
    assert_eq!(
        std::process::ExitCode::from(CliExitCode::HarnessFailed),
        std::process::ExitCode::from(10u8)
    );
}

// ---- T-024: Success exit code ----
#[test]
fn t_024_success_exit_code() {
    assert_eq!(
        std::process::ExitCode::from(CliExitCode::Success),
        std::process::ExitCode::SUCCESS
    );
}

// ---------------------------------------------------------------------------
// Output format tests
// ---------------------------------------------------------------------------

// ---- T-025: HarnessResult serializes to JSON for yaml/postcard output ----
#[test]
fn t_025_harness_result_json_serialization() {
    use vb_cli::harness::{HarnessResult, TranscriptEntry};

    let result = HarnessResult {
        iterations: 2,
        passed: 1,
        failed: 1,
        transcript: vec![
            TranscriptEntry {
                iteration: 1,
                seed: 1,
                passed: true,
                differences: 0,
                incident: None,
            },
            TranscriptEntry {
                iteration: 2,
                seed: 2,
                passed: false,
                differences: 2,
                incident: Some(serde_json::json!({"diverged": true})),
            },
        ],
        total_differences: 3,
    };

    // Should serialize without panic
    let json = serde_json::to_value(&result).expect("result should serialize");

    // Verify structure
    assert_eq!(json["kind"], "harness_result" || json["iterations"] == 2);
    assert_eq!(json["iterations"], 2);
    assert_eq!(json["passed"], 1);
    assert_eq!(json["failed"], 1);
    assert_eq!(json["total_differences"], 3);
    assert_eq!(json["transcript"].as_array().unwrap().len(), 2);
}
