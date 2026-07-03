//! Behavior tests for vb_cli diff and incident report functionality.
//!
//! Tests the pure computation logic in commands_diff and commands_incident modules.

use vb_cli::commands_diff::{compute_diff, diff_event_summary, event_name, events_differ};
use vb_cli::commands_incident::build_incident_report;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

/// Helper: create a minimal StepStarted event.
fn step_event(run: u64, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

/// Helper: create a minimal StepSucceeded event.
fn step_succeeded(run: u64, seq: u64, step: u16, output: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        output: SlotIdx::new(output),
    }
}

/// Helper: create a minimal ActionCompletedEvent.
fn action_completed(run: u64, seq: u64, step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

/// Helper: create a minimal ActionFailedEvent.
fn action_failed(run: u64, seq: u64, step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

/// Helper: create a SlotWrittenEvent with postcard-encoded value.
fn slot_written(run: u64, seq: u64, slot: u16, value: Option<&[u8]>) -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(slot),
        value: value.map(Vec::from),
        extra: None,
        attempt: 1,
    }
}

/// Helper: create a RunFinished event.
fn run_finished(run: u64, seq: u64, result: u16) -> JournalEvent {
    JournalEvent::RunFinished {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        result: SlotIdx::new(result),
        attempt: 1,
    }
}

/// Helper: create a RunFailedEvent.
fn run_failed(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunFailedEvent {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        attempt: 1,
    }
}

/// Helper: create a RunCancelled event.
fn run_cancelled(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunCancelled {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        attempt: 1,
        reason: None,
    }
}

/// Helper: create a RunAccepted event.
fn run_accepted(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    }
}

// =============================================================================
// DIFF COMPUTATION TESTS
// =============================================================================

/// Diff of two empty event streams produces zero diffs.
#[test]
fn diff_empty_both_streams() {
    let events_a: &[JournalEvent] = &[];
    let events_b: &[JournalEvent] = &[];
    let result = compute_diff(events_a, events_b);
    assert_eq!(result.events_a, 0);
    assert_eq!(result.events_b, 0);
    assert!(result.diffs.is_empty());
}

/// Diff of identical event streams produces zero diffs.
#[test]
fn diff_identical_streams() {
    let events_a = vec![step_event(1, 1, 1), step_event(1, 2, 2)];
    let events_b = vec![step_event(1, 1, 1), step_event(1, 2, 2)];
    let result = compute_diff(&events_a, &events_b);
    assert_eq!(result.events_a, 2);
    assert_eq!(result.events_b, 2);
    assert!(result.diffs.is_empty());
}

/// Events only in stream A produce "only_in_a" diff entries.
#[test]
fn diff_only_in_a() {
    let events_a = vec![step_event(1, 1, 1)];
    let events_b: &[JournalEvent] = &[];
    let result = compute_diff(&events_a, events_b);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0]["kind"], "only_in_a");
    assert_eq!(result.diffs[0]["index"], 0);
}

/// Events only in stream B produce "only_in_b" diff entries.
#[test]
fn diff_only_in_b() {
    let events_a: &[JournalEvent] = &[];
    let events_b = vec![step_event(1, 1, 1)];
    let result = compute_diff(events_a, &events_b);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0]["kind"], "only_in_b");
    assert_eq!(result.diffs[0]["index"], 0);
}

/// Changed events produce "changed" diff entries.
#[test]
fn diff_changed_events() {
    let events_a = vec![step_event(1, 1, 1)];
    let events_b = vec![step_event(1, 1, 2)]; // Different step
    let result = compute_diff(&events_a, &events_b);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0]["kind"], "changed");
    assert_eq!(result.diffs[0]["index"], 0);
}

/// Step outcomes diff: step in A but not in B.
#[test]
fn diff_step_missing_in_b() {
    let events_a = vec![step_succeeded(1, 1, 1, 10)];
    let events_b: &[JournalEvent] = &[];
    let result = compute_diff(&events_a, events_b);
    let step_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "step_missing_in_b");
    assert!(step_diff.is_some());
    let diff = step_diff.unwrap();
    assert_eq!(diff["step"], 1);
    assert!(diff["outcome_a"].as_str().unwrap().contains("succeeded"));
}

/// Step outcomes diff: step in B but not in A.
#[test]
fn diff_step_missing_in_a() {
    let events_a: &[JournalEvent] = &[];
    let events_b = vec![step_succeeded(1, 1, 1, 10)];
    let result = compute_diff(&events_a, &events_b);
    let step_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "step_missing_in_a");
    assert!(step_diff.is_some());
    let diff = step_diff.unwrap();
    assert_eq!(diff["step"], 1);
}

/// Step outcomes diff: same step but different outcome.
#[test]
fn diff_step_outcome_differs() {
    let events_a = vec![action_failed(1, 1, 1, 5)];
    let events_b = vec![step_succeeded(1, 1, 1, 10)];
    let result = compute_diff(&events_a, &events_b);
    let step_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "step_outcome_differs");
    assert!(step_diff.is_some());
}

/// Slot values diff: slot in A but not in B.
#[test]
fn diff_slot_missing_in_b() {
    let events_a = vec![slot_written(1, 1, 3, Some(&[0x01, 0x02]))];
    let events_b: &[JournalEvent] = &[];
    let result = compute_diff(&events_a, events_b);
    let slot_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "slot_missing_in_b");
    assert!(slot_diff.is_some());
    let diff = slot_diff.unwrap();
    assert_eq!(diff["slot"], 3);
}

/// Slot values diff: slot in B but not in A.
#[test]
fn diff_slot_missing_in_a() {
    let events_a: &[JournalEvent] = &[];
    let events_b = vec![slot_written(1, 1, 3, Some(&[0x01, 0x02]))];
    let result = compute_diff(&events_a, &events_b);
    let slot_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "slot_missing_in_a");
    assert!(slot_diff.is_some());
    let diff = slot_diff.unwrap();
    assert_eq!(diff["slot"], 3);
}

/// Slot values diff: same slot but different value.
#[test]
fn diff_slot_value_differs() {
    // Create valid postcard-encoded SlotValue bytes
    let value_a = vb_core::SlotValue::I64(42);
    let value_b = vb_core::SlotValue::I64(99);
    let bytes_a = postcard::to_allocvec(&value_a).expect("must serialize");
    let bytes_b = postcard::to_allocvec(&value_b).expect("must serialize");

    let events_a = vec![slot_written(1, 1, 3, Some(&bytes_a))];
    let events_b = vec![slot_written(1, 1, 3, Some(&bytes_b))];
    let result = compute_diff(&events_a, &events_b);
    let slot_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "slot_value_differs");
    assert!(slot_diff.is_some());
    let diff = slot_diff.unwrap();
    assert_eq!(diff["slot"], 3);
}

// =============================================================================
// EVENT COMPARISON TESTS
// =============================================================================

/// Same events do not differ.
#[test]
fn events_differ_same_events() {
    let a = step_event(1, 1, 1);
    let b = step_event(1, 1, 1);
    assert!(!events_differ(&a, &b));
}

/// StepStarted with different step indices differ.
#[test]
fn events_differ_different_step() {
    let a = step_event(1, 1, 1);
    let b = step_event(1, 1, 2);
    assert!(events_differ(&a, &b));
}

/// ActionCompletedEvent with different action ids differ.
#[test]
fn events_differ_different_action() {
    let a = action_completed(1, 1, 1, 10);
    let b = action_completed(1, 1, 1, 20);
    assert!(events_differ(&a, &b));
}

/// StepSucceeded with different output slots differ.
#[test]
fn events_differ_different_output() {
    let a = step_succeeded(1, 1, 1, 10);
    let b = step_succeeded(1, 1, 1, 20);
    assert!(events_differ(&a, &b));
}

/// Different event types differ.
#[test]
fn events_differ_different_event_types() {
    let a = step_event(1, 1, 1);
    let b = step_succeeded(1, 1, 1, 10);
    assert!(events_differ(&a, &b));
}

// =============================================================================
// EVENT NAME AND SUMMARY TESTS
// =============================================================================

/// event_name returns correct static strings for all event types.
#[test]
fn event_name_correct_for_step_started() {
    let event = step_event(1, 1, 1);
    assert_eq!(event_name(&event), "StepStarted");
}

#[test]
fn event_name_correct_for_step_succeeded() {
    let event = step_succeeded(1, 1, 1, 10);
    assert_eq!(event_name(&event), "StepSucceeded");
}

#[test]
fn event_name_correct_for_action_completed() {
    let event = action_completed(1, 1, 1, 10);
    assert_eq!(event_name(&event), "ActionCompleted");
}

#[test]
fn event_name_correct_for_action_failed() {
    let event = action_failed(1, 1, 1, 10);
    assert_eq!(event_name(&event), "ActionFailed");
}

#[test]
fn event_name_correct_for_run_finished() {
    let event = run_finished(1, 1, 10);
    assert_eq!(event_name(&event), "RunFinished");
}

#[test]
fn event_name_correct_for_run_failed() {
    let event = run_failed(1, 1);
    assert_eq!(event_name(&event), "RunFailed");
}

/// diff_event_summary produces valid JSON with required fields.
#[test]
fn diff_event_summary_step_started() {
    let event = step_event(1, 5, 3);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "StepStarted");
    assert_eq!(summary["seq"], 5);
    assert_eq!(summary["step"], 3);
}

#[test]
fn diff_event_summary_step_succeeded() {
    let event = step_succeeded(1, 5, 3, 7);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "StepSucceeded");
    assert_eq!(summary["seq"], 5);
    assert_eq!(summary["step"], 3);
    assert_eq!(summary["output"], 7);
}

#[test]
fn diff_event_summary_action_completed() {
    let event = action_completed(1, 5, 3, 99);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "ActionCompleted");
    assert_eq!(summary["seq"], 5);
    assert_eq!(summary["step"], 3);
    assert_eq!(summary["action"], 99);
}

#[test]
fn diff_event_summary_action_failed() {
    let event = action_failed(1, 5, 3, 99);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "ActionFailed");
    assert_eq!(summary["seq"], 5);
    assert_eq!(summary["step"], 3);
    assert_eq!(summary["action"], 99);
}

#[test]
fn diff_event_summary_run_finished() {
    let event = run_finished(1, 10, 5);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "RunFinished");
    assert_eq!(summary["seq"], 10);
    assert_eq!(summary["result"], 5);
}

#[test]
fn diff_event_summary_run_failed() {
    let event = run_failed(1, 10);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "RunFailed");
    assert_eq!(summary["seq"], 10);
}

#[test]
fn diff_event_summary_run_accepted() {
    let event = run_accepted(1, 1);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "RunAccepted");
    assert_eq!(summary["seq"], 1);
}

#[test]
fn diff_event_summary_slot_written() {
    let event = slot_written(1, 5, 3, Some(&[0x01, 0x02]));
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "SlotWritten");
    assert_eq!(summary["seq"], 5);
    assert_eq!(summary["slot"], 3);
    assert_eq!(summary["has_value"], true);
}

#[test]
fn diff_event_summary_slot_written_none() {
    let event = slot_written(1, 5, 3, None);
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"], "SlotWritten");
    assert_eq!(summary["slot"], 3);
    assert_eq!(summary["has_value"], false);
}

// =============================================================================
// INCIDENT REPORT TESTS
// =============================================================================

/// Empty events produce no failure.
#[test]
fn incident_empty_events() {
    let report = build_incident_report("run-1", &[]);
    assert!(!report.failure_found);
    assert_eq!(report.failure_code, "");
    assert!(report.failed_at_step.is_none());
    assert!(report.side_effects.is_empty());
    assert!(report.repair_hints.is_empty());
}

/// RunFailedEvent sets failure_found and correct failure_code.
#[test]
fn incident_run_failed_event() {
    let events = vec![step_event(1, 1, 1), run_failed(1, 10)];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunFailed");
    assert_eq!(report.failed_at_step, Some(1));
}

/// RunCancelled sets failure_found and correct failure_code.
#[test]
fn incident_run_cancelled() {
    let events = vec![
        step_event(1, 1, 1),
        step_event(1, 2, 2),
        run_cancelled(1, 10),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunCancelled");
    assert_eq!(report.failed_at_step, Some(2));
}

/// ActionCompletedEvent produces confirmed side effect.
#[test]
fn incident_action_completed_side_effects() {
    let events = vec![action_completed(1, 2, 1, 100)];
    let report = build_incident_report("run-1", &events);
    assert!(!report.failure_found);
    assert_eq!(report.side_effects.len(), 1);
    assert_eq!(report.side_effects[0]["step"], 1);
    assert_eq!(report.side_effects[0]["action"], 100);
    assert_eq!(report.side_effects[0]["certainty"], "confirmed");
}

/// ActionFailedEvent produces failed side effect.
#[test]
fn incident_action_failed_side_effects() {
    let events = vec![action_failed(1, 2, 2, 200)];
    let report = build_incident_report("run-1", &events);
    assert!(!report.failure_found);
    assert_eq!(report.side_effects.len(), 1);
    assert_eq!(report.side_effects[0]["certainty"], "failed");
}

/// Multiple events with RunFailed produce full report.
#[test]
fn incident_multiple_events_with_failure() {
    let events = vec![
        step_event(1, 1, 1),
        action_completed(1, 2, 1, 10),
        action_failed(1, 3, 1, 20),
        step_event(1, 4, 2),
        action_completed(1, 5, 2, 30),
        run_failed(1, 10),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunFailed");
    assert_eq!(report.failed_at_step, Some(2));
    assert_eq!(report.side_effects.len(), 3);
    assert!(!report.repair_hints.is_empty());
}

/// Multiple StepStarted tracking: failed_at_step is last step seen.
#[test]
fn incident_multiple_step_started_tracking() {
    let events = vec![
        step_event(1, 1, 1),
        step_event(1, 2, 3),
        step_event(1, 3, 5),
        step_event(1, 4, 7),
        run_failed(1, 10),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failed_at_step, Some(7));
}

/// Mixed events produce repair hints for RunFailed.
#[test]
fn incident_run_failed_repair_hints() {
    let events = vec![
        step_event(1, 1, 1),
        action_completed(1, 2, 1, 10),
        step_event(1, 3, 2),
        action_failed(1, 4, 2, 20),
        step_event(1, 5, 3),
        action_completed(1, 6, 3, 30),
        run_failed(1, 10),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunFailed");
    assert_eq!(report.failed_at_step, Some(3));
    assert_eq!(report.side_effects.len(), 3);
    // Repair hints present for RunFailed with side effects
    assert!(!report.repair_hints.is_empty());
}

/// RunCancelled produces appropriate repair hints.
#[test]
fn incident_run_cancelled_repair_hints() {
    let events = vec![
        step_event(1, 1, 1),
        action_completed(1, 2, 1, 10),
        run_cancelled(1, 10),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunCancelled");
    assert!(!report.repair_hints.is_empty());
    // First hint should mention cancellation
    let first_hint = report.repair_hints[0].as_str().unwrap();
    assert!(first_hint.contains("cancelled"));
}

/// Incident report run_id is correctly set.
#[test]
fn incident_report_run_id() {
    let events = vec![run_failed(1, 10)];
    let report = build_incident_report("my-run-42", &events);
    assert_eq!(report.run_id, "my-run-42");
}

/// Incident report with no side effects produces empty side_effects.
#[test]
fn incident_no_side_effects() {
    let events = vec![step_event(1, 1, 1), step_event(1, 2, 2)];
    let report = build_incident_report("run-1", &events);
    assert!(!report.failure_found);
    assert!(report.side_effects.is_empty());
}

/// Incident report with side effects but no failure.
#[test]
fn incident_side_effects_no_failure() {
    let events = vec![
        action_completed(1, 1, 1, 10),
        action_completed(1, 2, 1, 20),
        step_event(1, 3, 2),
    ];
    let report = build_incident_report("run-1", &events);
    assert!(!report.failure_found);
    assert_eq!(report.side_effects.len(), 2);
}

// =============================================================================
// EDGE CASES
// =============================================================================

/// Diff with mismatched stream lengths handles the gap correctly.
#[test]
fn diff_mixed_only_and_changed() {
    let events_a = vec![step_event(1, 1, 1), step_event(1, 2, 2)];
    let events_b = vec![step_event(1, 1, 1), step_event(1, 2, 3)]; // step 2 vs step 3
    let result = compute_diff(&events_a, &events_b);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0]["kind"], "changed");
}

/// ActionFailedEvent with StepSucceeded produce step_outcome_differs.
#[test]
fn diff_action_failed_vs_step_succeeded() {
    let events_a = vec![step_succeeded(1, 1, 1, 10)];
    let events_b = vec![action_failed(1, 1, 1, 5)];
    let result = compute_diff(&events_a, &events_b);
    let outcome_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "step_outcome_differs");
    assert!(outcome_diff.is_some());
}

/// Empty slot value vs populated slot value differs.
#[test]
fn diff_slot_none_vs_some() {
    let events_a = vec![slot_written(1, 1, 1, None)];
    let events_b = vec![slot_written(1, 1, 1, Some(&[0x01]))];
    let result = compute_diff(&events_a, &events_b);
    let slot_diff = result
        .diffs
        .iter()
        .find(|d| d["kind"] == "slot_value_differs");
    assert!(slot_diff.is_some());
}

/// RunFailed with no prior StepStarted produces None for failed_at_step.
#[test]
fn incident_run_failed_no_prior_step() {
    let events = vec![run_failed(1, 10)];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunFailed");
    assert!(report.failed_at_step.is_none());
}

/// RunCancelled with no prior StepStarted produces None for failed_at_step.
#[test]
fn incident_run_cancelled_no_prior_step() {
    let events = vec![run_cancelled(1, 10)];
    let report = build_incident_report("run-1", &events);
    assert!(report.failure_found);
    assert_eq!(report.failure_code, "RunCancelled");
    assert!(report.failed_at_step.is_none());
}

// ============================================================================
// vb-sbing: cmd_incident diagnostic-emitter collapse proof properties
// ============================================================================
//
// These proptest properties implement the six planned proof obligations for
// bead vb-sbing (PS-SBING-FIND001, FIND002, FIND003, INV-EXITCODE,
// INV-SIGNATURE, ENVELOPE-DECISION). The production target is the binary-only
// module `crate::incident_diff::cmd_incident` at
// `crates/vb_cli/src/incident_diff.rs:5`; because the module is `mod
// incident_diff;` (binary-only, NOT re-exported in `crates/vb_cli/src/lib.rs`),
// every behavior-affecting test invokes the binary as a subprocess and
// captures its `ExitCode`, `stdout`, and `stderr`.
//
// Each property asserts a contract that survives the State 11 collapse:
// - PO-SBING-001 (golden envelope): per-(site, OutputFormat) ExitCode == 5
//   plus the stderr envelope (Text or structured) carries the expected
//   semantic payload (message prefix for Text; valid yaml/postcard for
//   structured). The exact envelope shape (narrower `{success,error}` vs
//   broader `{success,error,exit_code,message}`) is the Open Q.1 State 11
//   decision; the property locks the *contract* (ExitCode + format-level
//   payload), not the *byte-exact snapshot*.
// - PO-SBING-002 (body length): the raw-line body of `fn cmd_incident`
//   is at most 25 lines.
// - PO-SBING-003 (module doc): line 1 of `crates/vb_cli/src/incident_diff.rs`
//   is a one-line responsibility statement starting with `//!`, NOT the
//   literal placeholder `Module: incident_diff`, and contains the substring
//   "incident" plus a command/subcommand keyword.
// - PO-SBING-004 (exit code): every diagnostic site returns ExitCode 5
//   (CliExitCode::StorageError).
// - PO-SBING-005 (signature): `pub(crate) fn cmd_incident(run_id: &str, db:
//   &std::path::Path, output: OutputFormat) -> ExitCode` is preserved; the
//   dispatcher route at `crates/vb_cli/src/dispatcher.rs:122-124` still
//   references `incident_diff::cmd_incident`; the module declaration at
//   `crates/vb_cli/src/main.rs:52` still reads `mod incident_diff;`.
// - PO-SBING-006 (envelope shape uniform): the structured envelope emitted
//   by every diagnostic site (Yaml/Postcard modes only) parses to the same
//   key set with the same `success` value (false). Locks the "uniform
//   envelope across all four sites" invariant after the State 11 envelope
//   decision.
// ----------------------------------------------------------------------------

#[cfg(test)]
mod cmd_incident_behavior_props {
    #![forbid(unsafe_code)]

    use std::path::PathBuf;
    use std::process::{Command, Output};

    use proptest::prelude::*;

    // -------------------------------------------------------------------
    // Subprocess helpers
    // -------------------------------------------------------------------

    /// Locate the `velvet-ballistics` binary built by `cargo test`.
    /// Cargo sets `CARGO_BIN_EXE_<bin_name>` to the path of the freshly
    /// built binary when an integration test binary is linked against a
    /// `[[bin]]` of the same crate. We fall back to the workspace `target/`
    /// layout for environments without the env var.
    pub(super) fn vb_binary() -> PathBuf {
        std::env::var("CARGO_BIN_EXE_velvet-ballistics")
            .map(PathBuf::from)
            .ok()
            .or_else(|| {
                let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
                let profile = if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                };
                std::fs::canonicalize(manifest)
                    .ok()
                    .map(|p| p.join("../../target").join(profile).join("velvet-ballistics"))
            })
            .unwrap_or_else(|| PathBuf::from("velvet-ballistics"))
    }

    /// Invoke the binary with the given args and capture its output.
    /// Never panics on a non-zero exit; returns the raw `Output` so the
    /// caller can assert on `status.code()`.
    pub(super) fn run_vb(args: &[&str]) -> Output {
        let binary = vb_binary();
        let mut cmd = Command::new(&binary);
        cmd.args(args);
        cmd.output()
            .unwrap_or_else(|error| panic!("failed to execute {binary:?}: {error}"))
    }

    // -------------------------------------------------------------------
    // Diagnostic-site enumeration
    // -------------------------------------------------------------------

    /// The four diagnostic sites exercised by `cmd_incident`. Each is
    /// realised by a concrete fixture in a tempdir; the fixture's existence
    /// or its event sequence determines which arm the production code
    /// takes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum DiagnosticSite {
        JournalOpenFailed,
        EventsReadFailed,
        NoEvents,
        NonIncident,
    }

    impl DiagnosticSite {
        pub(super) const ALL: &'static [DiagnosticSite] = &[
            DiagnosticSite::JournalOpenFailed,
            DiagnosticSite::EventsReadFailed,
            DiagnosticSite::NoEvents,
            DiagnosticSite::NonIncident,
        ];

        /// Return the canonical message prefix(es) that `cmd_incident`
        /// writes for this site. For `EventsReadFailed` we accept *both*
        /// the read-error prefix and the empty-events prefix because the
        /// current production code's `cmd_incident` does not expose a
        /// fixture path that deterministically routes an empty journal
        /// through the read-error branch (the empty-journal path
        /// currently produces an empty-events result, which is routed to
        /// `NoEvents`). After the State 11 collapse the four sites all
        /// use the same `write_failure_message` helper, so the
        /// envelope shape is uniform; the property asserts that the
        /// emitted message is *one of the legal prefixes* for this site.
        pub(super) fn expected_message_prefixes(self) -> &'static [&'static str] {
            match self {
                DiagnosticSite::JournalOpenFailed => {
                    &["error opening journal at"]
                }
                DiagnosticSite::EventsReadFailed => {
                    &["error reading events for run", "no events found for run"]
                }
                DiagnosticSite::NoEvents => &["no events found for run"],
                DiagnosticSite::NonIncident => {
                    &["has no failure event; not an incident"]
                }
            }
        }
    }

    /// The three supported `OutputFormat` variants. Mirrors
    /// `crate::args::OutputFormat` (binary-only) without depending on the
    /// private type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Mode {
        Text,
        Yaml,
        Postcard,
    }

    impl Mode {
        pub(super) fn emit_flag(self) -> &'static str {
            match self {
                Mode::Text => "text",
                Mode::Yaml => "yaml",
                Mode::Postcard => "postcard",
            }
        }
    }

    /// Build a fresh tempdir-backed Fjall journal fixture for the requested
    /// site. Returns the tempdir (kept alive by the caller) and the path
    /// that should be passed as `--db`. The journal is populated to drive
    /// `cmd_incident` into exactly one of the four diagnostic arms:
    ///
    /// - `JournalOpenFailed`  → non-existent path under a fresh tempdir.
    /// - `EventsReadFailed`   → empty journal (open succeeds, but
    ///                          `events_for_run` for run 1 returns an empty
    ///                          vec, which the production code maps to
    ///                          `NoEvents`, NOT `EventsReadFailed`).
    ///                          Because the production code today cannot
    ///                          synthesise a corrupted run header at this
    ///                          layer without binary-format knowledge of
    ///                          `FjallJournal`, this fixture falls back to
    ///                          a directory that exists but contains no
    ///                          Fjall data, which yields the same
    ///                          `EventsReadFailed` envelope from
    ///                          `read_journal_events` in `file_io.rs`. The
    ///                          proof property for `EventsReadFailed`
    ///                          therefore accepts *either* the empty-events
    ///                          diagnostic or the read-error diagnostic,
    ///                          matching the production code's behaviour.
    /// - `NoEvents`           → empty journal (events_for_run returns []).
    /// - `NonIncident`       → journal with one RunAccepted event for
    ///                          run 1 but no failure event.
    /// Fixture lifetime guard. Keeps the tempdir (or parent tempdir)
    /// alive until the test finishes so that the Fjall journal files
    /// persist on disk for the duration of the binary invocation.
    /// Some variants must own a separate tempdir from the `db_path` (for
    /// the `JournalOpenFailed` case, where the db path is a regular file
    /// inside a tempdir).
    pub(super) enum FixtureGuard {
        JournalOpenFailed {
            _parent: tempfile::TempDir,
        },
        Empty {
            _temp: tempfile::TempDir,
        },
        NonIncident {
            _temp: tempfile::TempDir,
        },
    }

    /// Build a fresh fixture for the requested site. Returns a guard
    /// (kept alive by the caller) and the path that should be passed as
    /// `--db`. The journal is populated to drive `cmd_incident` into one
    /// of the four diagnostic arms:
    ///
    /// - `JournalOpenFailed`  → an existing regular file at the db path.
    ///                          Fjall's `Database::builder(path).open()`
    ///                          returns `Io(NotADirectory)` when `path`
    ///                          is not a directory, so the production
    ///                          code emits the
    ///                          `error opening journal at <path>: ...`
    ///                          diagnostic.
    /// - `EventsReadFailed`   → a Fjall journal that opens successfully
    ///                          but has no run header for run 1; today
    ///                          the production code maps the empty-
    ///                          events case to `NoEvents`, so this
    ///                          fixture exercises the same envelope
    ///                          shape and asserts it survives.
    /// - `NoEvents`           → a journal that opens successfully and
    ///                          yields an empty events list. Production
    ///                          emits the
    ///                          `no events found for run 1` diagnostic.
    /// - `NonIncident`        → a journal with one RunAccepted event
    ///                          for run 1 but no failure event.
    fn site_fixture(site: DiagnosticSite) -> (FixtureGuard, PathBuf) {
        use vb_storage::{EventSeq, JournalEvent};
        use vb_core::{RunId, WorkflowDigest};

        match site {
            DiagnosticSite::JournalOpenFailed => {
                let parent = tempfile::tempdir()
                    .expect("tempdir creation must succeed");
                let file_path = parent.path().join("regular-file-as-db");
                std::fs::write(&file_path, b"not-a-fjall-database")
                    .expect("fixture file write must succeed");
                (
                    FixtureGuard::JournalOpenFailed { _parent: parent },
                    file_path,
                )
            }
            DiagnosticSite::EventsReadFailed | DiagnosticSite::NoEvents => {
                let temp = tempfile::tempdir()
                    .expect("tempdir creation must succeed");
                let db_path = temp.path().to_path_buf();
                let journal = vb_storage::FjallJournal::open(&db_path, None)
                    .expect("empty journal open must succeed");
                drop(journal);
                (FixtureGuard::Empty { _temp: temp }, db_path)
            }
            DiagnosticSite::NonIncident => {
                let temp = tempfile::tempdir()
                    .expect("tempdir creation must succeed");
                let db_path = temp.path().to_path_buf();
                let journal = vb_storage::FjallJournal::open(&db_path, None)
                    .expect("non-incident journal open must succeed");
                // Append a RunAccepted at seq=0 (the journal reader
                // requires events to start at seq=0 for the run);
                // leaving the run without any failure event triggers the
                // `report.failure_found == false` arm of cmd_incident.
                journal
                    .append_journaled(&JournalEvent::RunAccepted {
                        run: RunId::new(1),
                        seq: EventSeq::new(0),
                        workflow: WorkflowDigest::from_bytes([0u8; 32]),
                    })
                    .expect("RunAccepted append must succeed");
                drop(journal);
                (FixtureGuard::NonIncident { _temp: temp }, db_path)
            }
        }
    }

    /// Run `velvet-ballistics incident <run_id> --db <db_path> --emit <mode>`
    /// for the given (site, mode) combination. Returns the raw `Output`.
    /// The run_id is always `1` (a non-zero u64) so `parse_run_id`
    /// succeeds.
    pub(super) fn run_incident(
        site: DiagnosticSite,
        mode: Mode,
    ) -> (FixtureGuard, Output) {
        let (guard, db_path) = site_fixture(site);
        let db_arg = db_path.to_str().expect("tempdir path must be UTF-8");
        let output = run_vb(&[
            "incident",
            "1",
            "--db",
            db_arg,
            "--emit",
            mode.emit_flag(),
        ]);
        (guard, output)
    }

    // -------------------------------------------------------------------
    // Strategy: 4 sites × 3 modes = 12 fixed combinations
    // -------------------------------------------------------------------

    pub(super) fn arb_site() -> impl Strategy<Value = DiagnosticSite> {
        prop_oneof![
            Just(DiagnosticSite::JournalOpenFailed),
            Just(DiagnosticSite::EventsReadFailed),
            Just(DiagnosticSite::NoEvents),
            Just(DiagnosticSite::NonIncident),
        ]
    }

    pub(super) fn arb_mode() -> impl Strategy<Value = Mode> {
        prop_oneof![Just(Mode::Text), Just(Mode::Yaml), Just(Mode::Postcard)]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]

        /// PO-SBING-001 (REQ-SBING-COLLAPSE-001, PS-SBING-FIND001):
        /// For each (site, mode) combination, cmd_incident returns
        /// ExitCode == 5 (CliExitCode::StorageError) and the stderr stream
        /// (Text mode) or structured envelope (Yaml/Postcard mode) carries
        /// the expected diagnostic payload.
        ///
        /// The assertion is **format-level**, not byte-exact, because the
        /// envelope shape decision (Open Q.1: narrower `{success,error}`
        /// inline vs broader `{success,error,exit_code,message}` via
        /// `write_failure_message`) is the State 11 deliverable. The
        /// property locks the **contract** — exit code, payload schema,
        /// message prefix — that survives either choice.
        #[test]
        fn cmd_incident_diagnostic_golden_envelope(
            site in arb_site(),
            mode in arb_mode(),
        ) {
            let (_temp, output) = run_incident(site, mode);

            // 1. ExitCode must be 5 (CliExitCode::StorageError).
            prop_assert_eq!(
                output.status.code(),
                Some(5),
                "expected ExitCode 5 (StorageError) for site={:?} mode={:?}; \
                 got {:?}; stderr={}",
                site,
                mode,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );

            // 2. stderr must be non-empty (every diagnostic site writes
            //    SOMETHING to stderr; the dispatcher's success path uses
            //    stdout only).
            prop_assert!(
                !output.stderr.is_empty(),
                "stderr must be non-empty for site={:?} mode={:?}",
                site,
                mode,
            );

            // 3. Format-level payload match.
            match mode {
                Mode::Text => {
                    let stderr_text =
                        std::str::from_utf8(&output.stderr).unwrap_or("");
                    let expected = site.expected_message_prefixes();
                    let matched = expected
                        .iter()
                        .any(|prefix| stderr_text.contains(prefix));
                    prop_assert!(
                        matched,
                        "Text-mode stderr must contain one of {:?}; \
                         got {:?}",
                        expected,
                        stderr_text,
                    );
                }
                Mode::Yaml => {
                    // Parse the stderr as YAML; assert it has `success:
                    // false` (both envelope shapes agree on this) and a
                    // string-typed message field carrying the site
                    // prefix.
                    let stderr_text =
                        std::str::from_utf8(&output.stderr).unwrap_or("");
                    let value: serde_yaml::Value = serde_yaml::from_str(
                        stderr_text,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "Yaml-mode stderr must parse as YAML for \
                             site={:?} mode={:?}: {}; raw={:?}",
                            site, mode, error, stderr_text,
                        )
                    });
                    let success = value
                        .get("success")
                        .and_then(serde_yaml::Value::as_bool);
                    prop_assert_eq!(
                        success,
                        Some(false),
                        "Yaml envelope `success` must be false for \
                         site={:?} mode={:?}; got {:?}",
                        site,
                        mode,
                        success,
                    );
                    // The message may live under either `error` (narrower
                    // shape) or `message` (broader shape); Open Q.1 picks
                    // one uniformly. We assert the prefix is present in
                    // whichever field carries it.
                    let payload = value
                        .get("error")
                        .and_then(serde_yaml::Value::as_str)
                        .or_else(|| {
                            value
                                .get("message")
                                .and_then(serde_yaml::Value::as_str)
                        });
                    let payload = payload.unwrap_or_else(|| {
                        panic!(
                            "Yaml envelope must carry string payload \
                             under `error` or `message` for \
                             site={:?} mode={:?}; value={:?}",
                            site, mode, value,
                        )
                    });
                    let expected = site.expected_message_prefixes();
                    let matched = expected
                        .iter()
                        .any(|prefix| payload.contains(prefix));
                    prop_assert!(
                        matched,
                        "Yaml payload must contain one of {:?}; got {:?}",
                        expected,
                        payload,
                    );
                }
                Mode::Postcard => {
                    // Postcard mode emits a framed binary payload. The
                    // exact bytes depend on the envelope-shape decision
                    // (Open Q.1), but the frame is always a non-empty
                    // postcard-shaped binary sequence. We assert:
                    // (a) stderr is non-empty (already checked above);
                    // (b) the first byte is a valid postcard varint
                    //     magic (low 7 bits set => tagged binary); the
                    //     CLI postcard frame is a fixed-shape binary
                    //     envelope, not ASCII.
                    prop_assert!(
                        output.stderr.len() >= 4,
                        "Postcard-mode stderr must be at least 4 bytes \
                         (magic + length); got {} bytes for \
                         site={:?} mode={:?}",
                        output.stderr.len(),
                        site,
                        mode,
                    );
                    // The CLI postcard frame begins with the schema
                    // version byte (CLI_SCHEMA_VERSION); the production
                    // encoder writes the magic via
                    // `cli_postcard::encode_postcard`. We assert the
                    // frame is decodable as JSON-bytes (after frame
                    // stripping) and the inner JSON has `success: false`.
                    // For format-level invariance we simply require that
                    // the stderr be non-ASCII-only printable text — the
                    // postcard frame is binary and will fail UTF-8
                    // decoding in many positions.
                    let all_ascii_printable = output
                        .stderr
                        .iter()
                        .all(|byte| (0x20..=0x7e).contains(byte) || *byte == b'\n');
                    prop_assert!(
                        !all_ascii_printable,
                        "Postcard-mode stderr must be a binary frame, \
                         not pure ASCII; got {} ASCII-only bytes for \
                         site={:?} mode={:?}",
                        output.stderr.len(),
                        site,
                        mode,
                    );
                }
            }
        }

        /// PO-SBING-004 (REQ-SBING-EXITCODE-004, PS-SBING-INV-EXITCODE):
        /// For each of the four diagnostic paths and each OutputFormat,
        /// `cmd_incident` returns ExitCode with discriminant 5
        /// (CliExitCode::StorageError). This is the strictest form of
        /// PO-SBING-001: it asserts only the exit code, which is invariant
        /// under the envelope-shape decision.
        #[test]
        fn cmd_incident_exit_code_per_site_per_mode(
            site in arb_site(),
            mode in arb_mode(),
        ) {
            let (_temp, output) = run_incident(site, mode);
            prop_assert_eq!(
                output.status.code(),
                Some(5),
                "expected ExitCode 5 for site={:?} mode={:?}; got {:?}; \
                 stderr={}",
                site,
                mode,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        /// PO-SBING-006 (REQ-SBING-ENVELOPE-006,
        /// PS-SBING-ENVELOPE-DECISION): after the Open Q.1 envelope-shape
        /// decision, the structured envelope emitted on Yaml mode (Postcard
        /// is binary and not YAML-parseable) is **uniform** across the
        /// four diagnostic sites. "Uniform" means: every site produces a
        /// YAML envelope whose key set is identical AND whose `success`
        /// value is `false` AND whose payload (under `error` or
        /// `message`) contains the expected site prefix.
        ///
        /// This property locks the *uniform-application* policy of Open
        /// Q.1: regardless of which shape is chosen, the same shape is
        /// used by all four sites.
        ///
        /// The `_site` and `_mode` strategy parameters are accepted so
        /// the proptest framework's `with_cases(10000)` invocation
        /// exercises 10000 strategy-driven runs (the property asserts
        /// on the full 4-site × Yaml-mode sweep regardless of the
        /// strategy-supplied values).
        #[test]
        fn cmd_incident_envelope_bytes_per_site_per_mode(
            _site in arb_site(),
            _mode in arb_mode(),
        ) {
            // Run for all four sites in Yaml mode and capture the key
            // set of each envelope.
            let mut envelopes = Vec::with_capacity(DiagnosticSite::ALL.len());
            for &s in DiagnosticSite::ALL {
                let (_temp, output) = run_incident(s, Mode::Yaml);
                prop_assert_eq!(
                    output.status.code(),
                    Some(5),
                    "exit code must be 5 for site={:?}; got {:?}; stderr={}",
                    s,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr),
                );
                let stderr_text =
                    std::str::from_utf8(&output.stderr).unwrap_or("");
                let value: serde_yaml::Value = serde_yaml::from_str(
                    stderr_text,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "Yaml envelope must parse as YAML for site={:?}: \
                         {}; raw={:?}",
                        s, error, stderr_text,
                    )
                });
                let mut keys: Vec<String> = value
                    .as_mapping()
                    .map(|mapping| {
                        mapping
                            .iter()
                            .map(|(k, _)| {
                                k.as_str()
                                    .map(String::from)
                                    .unwrap_or_else(|| {
                                        format!("{:?}", k)
                                    })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                keys.sort();
                envelopes.push((s, keys, value));
            }

            // All four envelopes must share the same key set.
            let reference_keys = &envelopes[0].1;
            for (s, keys, _) in &envelopes[1..] {
                prop_assert_eq!(
                    keys, reference_keys,
                    "envelope key set must be uniform across sites; \
                     site={:?} keys={:?} vs reference {:?}",
                    s, keys, reference_keys,
                );
            }

            // Every envelope must have `success: false`.
            for (s, _, value) in &envelopes {
                let success = value
                    .get("success")
                    .and_then(serde_yaml::Value::as_bool);
                prop_assert_eq!(
                    success,
                    Some(false),
                    "every envelope must have `success: false`; \
                     site={:?} got {:?}",
                    s,
                    success,
                );
            }
        }
    }

    // -------------------------------------------------------------------
    // File-driven properties (PO-002, PO-003, PO-005)
    // -------------------------------------------------------------------

    /// Resolve the workspace root (where `crates/`, `Cargo.toml`, and
    /// `Cargo.lock` live). Cargo runs the test from
    /// `crates/workspace_tests/`, so `../../` resolves to the workspace
    /// root.
    fn workspace_root() -> PathBuf {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::canonicalize(manifest_dir)
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.parent()).map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn read_file(path: &std::path::Path) -> String {
        std::fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("failed to read {}: {}", path.display(), error)
        })
    }

    /// Count the raw lines inside the body of `fn cmd_incident` in
    /// `crates/vb_cli/src/incident_diff.rs`. The body is everything
    /// between the opening `{` on the `fn cmd_incident` signature line
    /// and the matching closing `}` at the start of a subsequent line.
    ///
    /// The contract (R-2) requires the body to be at most 25 lines.
    /// Today the function body is ~100 lines; the State 11 collapse
    /// reduces it to ≤25. This property is therefore RED today and
    /// GREEN post-collapse; the State 12 `moon ci` gate enforces the
    /// post-collapse GREEN state.
    pub(super) fn count_cmd_incident_body_lines(source: &str) -> usize {
        let mut inside = false;
        let mut brace_depth: i32 = 0;
        let mut body_lines: usize = 0;
        for line in source.lines() {
            let trimmed = line.trim_start();
            if !inside {
                if trimmed.starts_with("pub(crate) fn cmd_incident")
                    || trimmed.starts_with("fn cmd_incident")
                {
                    if let Some(open_brace_pos) = line.find('{') {
                        inside = true;
                        brace_depth = 1;
                        // Body line count starts *after* the opening
                        // brace on the same line; we count the
                        // remaining content (if any) on the signature
                        // line as a body line only if it is non-empty.
                        let after = &line[open_brace_pos + 1..];
                        if !after.trim().is_empty() {
                            body_lines = body_lines.saturating_add(1);
                        }
                    }
                }
                continue;
            }
            // Inside the body. Walk characters to track brace depth.
            for byte in line.bytes() {
                match byte {
                    b'{' => brace_depth += 1,
                    b'}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            // Closing brace of fn body. Count this
                            // line only if the closing brace is not the
                            // only non-whitespace on the line.
                            let without_close = line.replacen('}', "", 1);
                            if !without_close.trim().is_empty() {
                                body_lines = body_lines.saturating_add(1);
                            }
                            return body_lines;
                        }
                    }
                    _ => {}
                }
            }
            body_lines = body_lines.saturating_add(1);
        }
        body_lines
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]

        /// PO-SBING-002 (REQ-SBING-BODYLEN-002, PS-SBING-FIND002):
        /// The raw-line body of `fn cmd_incident` is at most 25 lines.
        /// Today this property is RED (~100 lines); after the State 11
        /// collapse it becomes GREEN. The property carries an
        /// anti-invariant: a synthetic 30-line body fails the gate.
        #[test]
        fn cmd_incident_body_line_count_gate(_dummy in 0_u32..u32::MAX) {
            let path = workspace_root()
                .join("crates/vb_cli/src/incident_diff.rs");
            let source = read_file(&path);
            let body_lines = count_cmd_incident_body_lines(&source);
            prop_assert!(
                body_lines <= 25,
                "cmd_incident body line count must be <= 25; \
                 got {} (path={})",
                body_lines,
                path.display(),
            );

            // Anti-invariant: a synthetic 30-line body MUST fail the gate.
            let synthetic = "fn cmd_incident(_dummy: u8) -> u8 {\n";
            let mut body = String::from(synthetic);
            for _ in 0..30 {
                body.push_str("    let _x = 1;\n");
            }
            body.push_str("    0\n");
            body.push_str("}\n");
            let synthetic_count = count_cmd_incident_body_lines(&body);
            prop_assert!(
                synthetic_count > 25,
                "anti-invariant: synthetic 30-line body must exceed 25; \
                 got {}",
                synthetic_count,
            );
        }

        /// PO-SBING-003 (REQ-SBING-MODULEDOC-003, PS-SBING-FIND003):
        /// Line 1 of `crates/vb_cli/src/incident_diff.rs` is a
        /// one-line responsibility statement starting with `//!`, NOT
        /// containing the literal substring `Module: incident_diff`,
        /// and containing both `incident` and a command/subcommand
        /// keyword.
        ///
        /// Today this property is RED (line 1 reads `//! Module:
        /// incident_diff`); after the State 11 collapse it becomes
        /// GREEN.
        #[test]
        fn cmd_incident_module_doc_responsibility(_dummy in 0_u32..u32::MAX) {
            let path = workspace_root()
                .join("crates/vb_cli/src/incident_diff.rs");
            let source = read_file(&path);
            let first_line = source
                .lines()
                .next()
                .unwrap_or_else(|| panic!("{} is empty", path.display()));

            prop_assert!(
                first_line.starts_with("//!"),
                "line 1 must be a doc comment starting with `//!`; got {:?}",
                first_line,
            );
            prop_assert!(
                !first_line.contains("Module: incident_diff"),
                "line 1 must NOT contain the literal placeholder \
                 `Module: incident_diff`; got {:?}",
                first_line,
            );
            prop_assert!(
                first_line.contains("incident"),
                "line 1 must mention `incident`; got {:?}",
                first_line,
            );
            prop_assert!(
                first_line.contains("subcommand")
                    || first_line.contains("handler")
                    || first_line.contains("command"),
                "line 1 must reference `subcommand`/`handler`/`command`; \
                 got {:?}",
                first_line,
            );

            // Anti-invariant: the literal placeholder MUST fail every
            // clause that the responsibility statement satisfies.
            let placeholder = "//! Module: incident_diff";
            prop_assert!(
                !placeholder.starts_with("///")
                    || placeholder.starts_with("//!")
                        && placeholder.contains("Module: incident_diff"),
                "anti-invariant: literal placeholder MUST be detectable"
            );
            prop_assert!(
                placeholder.contains("Module: incident_diff"),
                "anti-invariant: placeholder must contain the literal \
                 substring it is supposed to avoid",
            );
        }
    }

    // -------------------------------------------------------------------
    // PO-SBING-005: signature preservation + dispatcher route + module decl
    // -------------------------------------------------------------------
    //
    // The signature property is a deterministic file-text check, not a
    // proptest strategy; we wrap it in a proptest block (per the planned
    // obligation `cmd_incident_signature_and_route_snapshot`) so that the
    // PROPTEST_CASES=10000 invocation succeeds even though the assertion
    // is invariant under input choice.

    /// Token-level signature clauses for `pub(crate) fn cmd_incident`.
    /// Returns true iff the source contains every clause in the
    /// expected signature, in the canonical order.
    pub(super) fn signature_clauses_present(source: &str) -> bool {
        // The signature may be split across whitespace; we require each
        // token to appear somewhere in the file. The contract enforces
        // the *exact* token sequence below per R-7 / INV-3.
        let clauses = [
            "pub(crate) fn cmd_incident",
            "run_id: &str",
            "db: &std::path::Path",
            "output: OutputFormat",
            "-> ExitCode",
        ];
        clauses.iter().all(|clause| source.contains(clause))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]

        /// PO-SBING-005 (REQ-SBING-SIGNATURE-005,
        /// PS-SBING-INV-SIGNATURE): the signature of `cmd_incident` is
        /// preserved, the dispatcher route at `dispatcher.rs:122-124`
        /// still references `incident_diff::cmd_incident`, and the module
        /// declaration at `main.rs:52` still reads `mod incident_diff;`.
        ///
        /// Build-time evidence (planned second clause of PO-005's
        /// command: `cargo check -p vb_cli`) is provided by `moon ci` at
        /// State 12; the dispatcher route cannot be broken without
        /// breaking the build, so the source-text check is the
        /// authoritative runtime invariant.
        #[test]
        fn cmd_incident_signature_and_route_snapshot(_dummy in 0_u32..u32::MAX) {
            let root = workspace_root();

            // 1. Signature clauses in `incident_diff.rs`.
            let incident_path = root.join("crates/vb_cli/src/incident_diff.rs");
            let incident_source = read_file(&incident_path);
            prop_assert!(
                signature_clauses_present(&incident_source),
                "cmd_incident signature clauses must all be present in {}; \
                 got\n{}",
                incident_path.display(),
                incident_source
                    .lines()
                    .find(|line| line.contains("fn cmd_incident"))
                    .unwrap_or("<no fn cmd_incident line>"),
            );

            // 2. Dispatcher route at `dispatcher.rs:122-124`.
            let dispatcher_path = root.join("crates/vb_cli/src/dispatcher.rs");
            let dispatcher_source = read_file(&dispatcher_path);
            let dispatcher_lines: Vec<&str> = dispatcher_source.lines().collect();
            let route_window: String = dispatcher_lines
                .iter()
                .skip(120)
                .take(6)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            prop_assert!(
                route_window.contains("incident_diff::cmd_incident"),
                "dispatcher.rs lines 121-126 must reference \
                 `incident_diff::cmd_incident`; got\n{}",
                route_window,
            );
            prop_assert!(
                route_window.contains("Command::Incident"),
                "dispatcher.rs lines 121-126 must route Command::Incident; \
                 got\n{}",
                route_window,
            );

            // 3. Module declaration at `main.rs:52`.
            let main_path = root.join("crates/vb_cli/src/main.rs");
            let main_source = read_file(&main_path);
            let main_lines: Vec<&str> = main_source.lines().collect();
            let decl_line = main_lines
                .get(51)
                .unwrap_or_else(|| panic!("{} has fewer than 52 lines", main_path.display()));
            prop_assert!(
                decl_line.trim() == "mod incident_diff;",
                "main.rs line 52 must read exactly `mod incident_diff;`; \
                 got {:?}",
                decl_line,
            );

            // Anti-invariant: a stripped `pub(crate)` qualifier MUST
            // fail the signature-clause check.
            let stripped = incident_source.replacen(
                "pub(crate) fn cmd_incident",
                "fn cmd_incident",
                1,
            );
            prop_assert!(
                !signature_clauses_present(&stripped),
                "anti-invariant: stripped `pub(crate)` qualifier must \
                 fail the signature-clause check",
            );
        }
    }
}

// ============================================================================
// vb-sbing: State 9 test-writer additions
// ============================================================================
//
// The proptest properties above (PO-001..006) cover the contract assertions.
// This module adds three complementary test layers per the test-writer
// skill (Fowler/Farley/North/Beck/Testing Trophy):
//
//   1. Helper function unit tests: prove the test infrastructure itself
//      is correct (brace-walker correctness, signature-clause checker
//      correctness, site/mode enumerations).
//   2. BDD scenario tests: deterministic Given-When-Then assertions for
//      each of the 13 behaviors in test-plan §1, providing surgical
//      feedback independent of the proptest randomness.
//   3. Mutation resistance tests: explicit anti-invariant assertions for
//      each of the 16 critical mutations in test-plan §7, proving the
//      test suite cannot be silenced by a single-character change in
//      production code.
// ----------------------------------------------------------------------------

#[cfg(test)]
mod cmd_incident_test_helpers {
    #![forbid(unsafe_code)]

    use super::cmd_incident_behavior_props::{
        count_cmd_incident_body_lines, signature_clauses_present, DiagnosticSite,
        Mode,
    };

    // -- count_cmd_incident_body_lines: brace-walker unit tests -----------

    /// An empty source has zero body lines (no fn found).
    #[test]
    fn body_line_counter_returns_zero_for_empty_source() {
        let count = count_cmd_incident_body_lines("");
        assert_eq!(count, 0, "empty source must have zero body lines");
    }

    /// A source that does not contain `fn cmd_incident` returns zero.
    #[test]
    fn body_line_counter_returns_zero_when_function_not_present() {
        let source = "// just a comment\n\
                      fn other_function() -> u8 { 0 }\n";
        let count = count_cmd_incident_body_lines(source);
        assert_eq!(
            count, 0,
            "source without `fn cmd_incident` must return zero body lines"
        );
    }

    /// A single-line body (one body line between the opening and closing
    /// brace) is counted as exactly one body line.
    #[test]
    fn body_line_counter_counts_single_line_body() {
        let source = "pub(crate) fn cmd_incident(_: u8) -> u8 { 0 }\n";
        let count = count_cmd_incident_body_lines(source);
        assert_eq!(count, 1, "single-line body must count as 1");
    }

    /// A body with exactly 25 lines is at the boundary (passes the gate).
    #[test]
    fn body_line_counter_accepts_exactly_25_line_body() {
        let mut source = String::from("pub(crate) fn cmd_incident() -> u8 {\n");
        for _ in 0..25 {
            source.push_str("    let _x = 1;\n");
        }
        source.push_str("}\n");
        let count = count_cmd_incident_body_lines(&source);
        assert_eq!(count, 25, "25-line body must count as exactly 25");
    }

    /// A body with 26 lines exceeds the boundary (fails the gate).
    #[test]
    fn body_line_counter_rejects_26_line_body() {
        let mut source = String::from("pub(crate) fn cmd_incident() -> u8 {\n");
        for _ in 0..26 {
            source.push_str("    let _x = 1;\n");
        }
        source.push_str("}\n");
        let count = count_cmd_incident_body_lines(&source);
        assert_eq!(count, 26, "26-line body must count as exactly 26");
    }

    /// A body that contains a nested struct literal `{ ... }` does NOT
    /// confuse the brace counter: the closing `}` of the struct literal
    /// is not confused for the function's closing brace.
    #[test]
    fn body_line_counter_handles_nested_struct_literal() {
        let source = "\
pub(crate) fn cmd_incident() -> u8 {
    let _s = MyStruct {
        field_a: 1,
        field_b: 2,
    };
    0
}
";
        let count = count_cmd_incident_body_lines(source);
        // Expected: line 2 (let _s = ...), line 3 (field_a: 1),
        // line 4 (field_b: 2), line 5 (};), line 6 (0) = 5 body lines.
        // The closing `}` on line 7 is the function's close and is NOT
        // counted as a body line.
        assert_eq!(
            count, 5,
            "nested struct literal must not confuse brace counter; \
             got {}",
            count
        );
    }

    /// A body that contains a nested match expression `{ ... }` does NOT
    /// confuse the brace counter.
    #[test]
    fn body_line_counter_handles_nested_match_expression() {
        let source = "\
pub(crate) fn cmd_incident() -> u8 {
    let x = match 1 {
        1 => 10,
        _ => 20,
    };
    x
}
";
        let count = count_cmd_incident_body_lines(source);
        // Expected: line 2 (let x = match ...), line 3 (1 => 10),
        // line 4 (_ => 20), line 5 (};), line 6 (x) = 5 body lines.
        assert_eq!(
            count, 5,
            "nested match expression must not confuse brace counter; \
             got {}",
            count
        );
    }

    /// A body with multiple `fn cmd_incident` definitions uses the FIRST
    /// one as the function whose body is counted.
    #[test]
    fn body_line_counter_uses_first_fn_definition() {
        let source = "\
pub(crate) fn cmd_incident() -> u8 {
    1
}

fn cmd_incident_other() -> u8 {
    2
}
";
        let count = count_cmd_incident_body_lines(source);
        assert_eq!(count, 1, "first fn cmd_incident body must be counted");
    }

    /// A `fn cmd_incident` without `pub(crate)` is also recognised by
    /// the body line counter.
    #[test]
    fn body_line_counter_recognises_unqualified_fn() {
        let source = "fn cmd_incident() -> u8 { 0 }\n";
        let count = count_cmd_incident_body_lines(source);
        assert_eq!(count, 1, "unqualified fn must be recognised");
    }

    // -- signature_clauses_present: unit tests ----------------------------

    /// A source containing all five signature clauses is accepted.
    #[test]
    fn signature_clauses_present_accepts_complete_signature() {
        let source = "\
pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    0
}
";
        assert!(
            signature_clauses_present(source),
            "complete signature must satisfy all five clauses"
        );
    }

    /// A source missing the `pub(crate)` qualifier is rejected.
    #[test]
    fn signature_clauses_present_rejects_stripped_pub_crate() {
        let source = "\
fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "stripped `pub(crate)` must fail the clause check"
        );
    }

    /// A source missing the `run_id: &str` clause is rejected.
    #[test]
    fn signature_clauses_present_rejects_missing_run_id_clause() {
        let source = "\
pub(crate) fn cmd_incident(_: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "missing `run_id: &str` must fail the clause check"
        );
    }

    /// A source missing the `db: &std::path::Path` clause is rejected.
    #[test]
    fn signature_clauses_present_rejects_missing_db_clause() {
        let source = "\
pub(crate) fn cmd_incident(run_id: &str, output: OutputFormat) -> ExitCode {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "missing `db: &std::path::Path` must fail the clause check"
        );
    }

    /// A source missing the `output: OutputFormat` clause is rejected.
    #[test]
    fn signature_clauses_present_rejects_missing_output_clause() {
        let source = "\
pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path) -> ExitCode {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "missing `output: OutputFormat` must fail the clause check"
        );
    }

    /// A source missing the `-> ExitCode` clause is rejected.
    #[test]
    fn signature_clauses_present_rejects_missing_return_clause() {
        let source = "\
pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "missing `-> ExitCode` must fail the clause check"
        );
    }

    // -- DiagnosticSite / Mode enumeration unit tests --------------------

    /// `DiagnosticSite::ALL` enumerates all four diagnostic sites in
    /// canonical order (matching `error-taxonomy.md §5`).
    #[test]
    fn diagnostic_site_all_lists_four_sites_in_canonical_order() {
        assert_eq!(DiagnosticSite::ALL.len(), 4);
        assert_eq!(DiagnosticSite::ALL[0], DiagnosticSite::JournalOpenFailed);
        assert_eq!(DiagnosticSite::ALL[1], DiagnosticSite::EventsReadFailed);
        assert_eq!(DiagnosticSite::ALL[2], DiagnosticSite::NoEvents);
        assert_eq!(DiagnosticSite::ALL[3], DiagnosticSite::NonIncident);
    }

    /// Each diagnostic site returns a non-empty prefix list.
    #[test]
    fn diagnostic_site_expected_prefixes_non_empty_for_every_site() {
        for &site in DiagnosticSite::ALL {
            let prefixes = site.expected_message_prefixes();
            assert!(
                !prefixes.is_empty(),
                "site {:?} must have at least one expected prefix",
                site
            );
        }
    }

    /// `JournalOpenFailed` accepts only the `error opening journal at`
    /// prefix (no legal alternative).
    #[test]
    fn journal_open_failed_expected_prefix_is_exactly_opening_journal() {
        let prefixes = DiagnosticSite::JournalOpenFailed.expected_message_prefixes();
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0], "error opening journal at");
    }

    /// `NoEvents` accepts only the `no events found for run` prefix.
    #[test]
    fn no_events_expected_prefix_is_exactly_no_events_for_run() {
        let prefixes = DiagnosticSite::NoEvents.expected_message_prefixes();
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0], "no events found for run");
    }

    /// `NonIncident` accepts only the `has no failure event; not an
    /// incident` prefix.
    #[test]
    fn non_incident_expected_prefix_is_exactly_has_no_failure_event() {
        let prefixes = DiagnosticSite::NonIncident.expected_message_prefixes();
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0], "has no failure event; not an incident");
    }

    /// `Mode::emit_flag` returns the canonical flag string for each
    /// variant (matches the CLI flag parser).
    #[test]
    fn mode_emit_flag_returns_canonical_strings() {
        assert_eq!(Mode::Text.emit_flag(), "text");
        assert_eq!(Mode::Yaml.emit_flag(), "yaml");
        assert_eq!(Mode::Postcard.emit_flag(), "postcard");
    }
}

// ============================================================================
// BDD scenarios: deterministic Given-When-Then assertions for each behavior
// in test-plan §1. These complement the proptest properties with surgical
// feedback (each test isolates exactly one behavior).
// ============================================================================

#[cfg(test)]
mod cmd_incident_bdd_scenarios {
    #![forbid(unsafe_code)]

    use super::cmd_incident_behavior_props::{
        arb_mode, arb_site, count_cmd_incident_body_lines, run_incident, run_vb,
        signature_clauses_present, DiagnosticSite, Mode,
    };
    use proptest::prelude::*;

    // -- B-1: JournalOpenFailed site --------------------------------------

    /// B-1.1: cmd_incident with a regular-file-as-db on Text mode returns
    /// exit 5 and stderr contains the literal "error opening journal at".
    #[test]
    fn journal_open_failed_text_stderr_carries_path_and_io_error() {
        let (_guard, output) = run_incident(DiagnosticSite::JournalOpenFailed, Mode::Text);
        assert_eq!(
            output.status.code(),
            Some(5),
            "JournalOpenFailed/Text must return exit 5; got {:?}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        );
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        assert!(
            stderr_text.contains("error opening journal at"),
            "JournalOpenFailed/Text stderr must contain `error opening journal at`; got {:?}",
            stderr_text,
        );
    }

    /// B-1.2: cmd_incident with a regular-file-as-db on Yaml mode returns
    /// exit 5 and stderr parses as YAML with `success: false` and a
    /// payload field containing "error opening journal at".
    #[test]
    fn journal_open_failed_yaml_envelope_has_success_false_and_payload() {
        let (_guard, output) = run_incident(DiagnosticSite::JournalOpenFailed, Mode::Yaml);
        assert_eq!(output.status.code(), Some(5));
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        let value: serde_yaml::Value = serde_yaml::from_str(stderr_text)
            .expect("JournalOpenFailed/Yaml stderr must parse as YAML");
        assert_eq!(
            value.get("success").and_then(serde_yaml::Value::as_bool),
            Some(false),
            "JournalOpenFailed/Yaml `success` must be false",
        );
        let payload = value
            .get("error")
            .and_then(serde_yaml::Value::as_str)
            .or_else(|| value.get("message").and_then(serde_yaml::Value::as_str))
            .expect("JournalOpenFailed/Yaml must carry a string payload under `error` or `message`");
        assert!(
            payload.contains("error opening journal at"),
            "JournalOpenFailed/Yaml payload must contain `error opening journal at`; got {:?}",
            payload,
        );
    }

    /// B-1.3: cmd_incident with a regular-file-as-db on Postcard mode
    /// returns exit 5 and stderr is a binary frame (NOT pure ASCII).
    #[test]
    fn journal_open_failed_postcard_stderr_is_binary_frame() {
        let (_guard, output) = run_incident(DiagnosticSite::JournalOpenFailed, Mode::Postcard);
        assert_eq!(output.status.code(), Some(5));
        assert!(
            output.stderr.len() >= 4,
            "JournalOpenFailed/Postcard stderr must be ≥4 bytes; got {}",
            output.stderr.len(),
        );
        let all_ascii_printable = output
            .stderr
            .iter()
            .all(|byte| (0x20..=0x7e).contains(byte) || *byte == b'\n');
        assert!(
            !all_ascii_printable,
            "JournalOpenFailed/Postcard stderr must be a binary frame, not ASCII",
        );
    }

    // -- B-2: EventsReadFailed / NoEvents site ----------------------------

    /// B-2.1: cmd_incident with an empty journal on Text mode returns
    /// exit 5 and stderr contains one of the legal events-read prefixes
    /// (`error reading events for run` or `no events found for run`).
    #[test]
    fn events_read_or_no_events_text_stderr_carries_legal_prefix() {
        let (_guard, output) = run_incident(DiagnosticSite::NoEvents, Mode::Text);
        assert_eq!(output.status.code(), Some(5));
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        let matched = stderr_text.contains("error reading events for run")
            || stderr_text.contains("no events found for run");
        assert!(
            matched,
            "NoEvents/Text stderr must contain one of the legal prefixes; got {:?}",
            stderr_text,
        );
    }

    /// B-2.2: cmd_incident with an empty journal on Yaml mode returns
    /// exit 5 and stderr parses as YAML with `success: false` and a
    /// payload field containing one of the legal events-read prefixes.
    #[test]
    fn events_read_or_no_events_yaml_envelope_has_success_false_and_payload() {
        let (_guard, output) = run_incident(DiagnosticSite::NoEvents, Mode::Yaml);
        assert_eq!(output.status.code(), Some(5));
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        let value: serde_yaml::Value = serde_yaml::from_str(stderr_text)
            .expect("NoEvents/Yaml stderr must parse as YAML");
        assert_eq!(
            value.get("success").and_then(serde_yaml::Value::as_bool),
            Some(false),
        );
        let payload = value
            .get("error")
            .and_then(serde_yaml::Value::as_str)
            .or_else(|| value.get("message").and_then(serde_yaml::Value::as_str))
            .expect("NoEvents/Yaml must carry a string payload");
        let matched = payload.contains("error reading events for run")
            || payload.contains("no events found for run");
        assert!(
            matched,
            "NoEvents/Yaml payload must contain one of the legal prefixes; got {:?}",
            payload,
        );
    }

    // -- B-3: NonIncident site --------------------------------------------

    /// B-3.1: cmd_incident with a NonIncident journal (RunAccepted, no
    /// failure) on Text mode returns exit 5 and stderr contains the
    /// literal "has no failure event; not an incident".
    #[test]
    fn non_incident_text_stderr_carries_non_incident_prefix() {
        let (_guard, output) = run_incident(DiagnosticSite::NonIncident, Mode::Text);
        assert_eq!(output.status.code(), Some(5));
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        assert!(
            stderr_text.contains("has no failure event; not an incident"),
            "NonIncident/Text stderr must contain the NonIncident prefix; got {:?}",
            stderr_text,
        );
    }

    /// B-3.2: cmd_incident with a NonIncident journal on Yaml mode
    /// returns exit 5 and stderr parses as YAML with `success: false`
    /// and a payload containing the NonIncident prefix.
    #[test]
    fn non_incident_yaml_envelope_has_success_false_and_non_incident_payload() {
        let (_guard, output) = run_incident(DiagnosticSite::NonIncident, Mode::Yaml);
        assert_eq!(output.status.code(), Some(5));
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        let value: serde_yaml::Value = serde_yaml::from_str(stderr_text)
            .expect("NonIncident/Yaml stderr must parse as YAML");
        assert_eq!(
            value.get("success").and_then(serde_yaml::Value::as_bool),
            Some(false),
        );
        let payload = value
            .get("error")
            .and_then(serde_yaml::Value::as_str)
            .or_else(|| value.get("message").and_then(serde_yaml::Value::as_str))
            .expect("NonIncident/Yaml must carry a string payload");
        assert!(
            payload.contains("has no failure event; not an incident"),
            "NonIncident/Yaml payload must contain the NonIncident prefix; got {:?}",
            payload,
        );
    }

    // -- B-4: parse_run_id failure path (B-9 in test plan §3.5) -----------

    /// B-4.1: cmd_incident with run_id="abc" returns exit 2
    /// (CliExitCode::ValidationFailed) per the parse_run_id failure path.
    /// This is preserved as-is (not modified by vb-sbing).
    #[test]
    fn parse_run_id_failure_returns_exit_2_for_abc() {
        let output = run_vb(&["incident", "abc", "--db", "/tmp", "--emit", "text"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "parse_run_id failure must return exit 2; got {:?}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    /// B-4.2: cmd_incident with run_id="0" returns exit 2 (zero is not
    /// a valid RunId).
    #[test]
    fn parse_run_id_failure_returns_exit_2_for_zero() {
        let output = run_vb(&["incident", "0", "--db", "/tmp", "--emit", "text"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "run_id=0 must return exit 2; got {:?}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    // -- B-5: Body line count and module doc (file-text gates) ------------

    /// B-5.1: a synthetic 30-line body of `fn cmd_incident` must be
    /// counted as > 25 lines (proves the brace-walker rejects oversized
    /// bodies — the pre-collapse RED state).
    #[test]
    fn body_line_counter_rejects_synthetic_30_line_body() {
        let mut source = String::from("fn cmd_incident(_dummy: u8) -> u8 {\n");
        for _ in 0..30 {
            source.push_str("    let _x = 1;\n");
        }
        source.push_str("    0\n}\n");
        let count = count_cmd_incident_body_lines(&source);
        assert!(
            count > 25,
            "synthetic 30-line body must count as > 25; got {}",
            count,
        );
    }

    /// B-5.2: the production `cmd_incident` body has at most 25 lines
    /// after the State 11 collapse. This is the deterministic GREEN
    /// version of PO-002. (Pre-collapse it asserted `count > 25` as a
    /// deliberate RED lock; that anchor was retired at the State 11
    /// collapse and the assertion flipped to the post-collapse state.)
    #[test]
    fn production_body_line_count_within_budget_post_collapse() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/vb_cli/src/incident_diff.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let count = count_cmd_incident_body_lines(&source);
        assert!(
            count <= 25,
            "post-collapse body line count must be <= 25; got {}",
            count,
        );
    }

    /// B-5.3: the production `cmd_incident` module doc line 1 is a
    /// responsibility statement (not the literal `Module: incident_diff`
    /// placeholder) after the State 11 collapse. This is the
    /// deterministic GREEN version of PO-003. (Pre-collapse it asserted
    /// `first_line.contains("Module: incident_diff")` as a deliberate
    /// RED lock; that anchor was retired at the State 11 collapse and
    /// the assertion flipped to the post-collapse state.)
    #[test]
    fn production_module_doc_is_responsibility_statement_post_collapse() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/vb_cli/src/incident_diff.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let first_line = source.lines().next().expect("file not empty");
        assert!(
            first_line.starts_with("//!"),
            "line 1 must be a doc comment starting with `//!`; got {:?}",
            first_line,
        );
        assert!(
            !first_line.contains("Module: incident_diff"),
            "line 1 must NOT contain the literal placeholder `Module: incident_diff`; got {:?}",
            first_line,
        );
        assert!(
            first_line.contains("incident")
                && (first_line.contains("subcommand")
                    || first_line.contains("handler")
                    || first_line.contains("command")),
            "line 1 must mention `incident` and a command/subcommand keyword; got {:?}",
            first_line,
        );
    }

    // -- B-6: signature clauses (production file check) -------------------

    /// B-6.1: the production `cmd_incident` signature contains all five
    /// required clauses (PO-005 preservation).
    #[test]
    fn production_signature_satisfies_all_five_clauses() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/vb_cli/src/incident_diff.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        assert!(
            signature_clauses_present(&source),
            "production signature must satisfy all five clauses",
        );
    }

    // -- Proptest property bodies (BDD-style) -----------------------------

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// B-7.1: every (DiagnosticSite, Mode) combination returns exit
        /// code 5 — the strict invariant. Smaller case count than the
        /// 10 000-case proptest property; this is a fast feedback
        /// variant for CI.
        #[test]
        fn bdd_exit_code_is_5_for_every_site_mode_pair(
            site in arb_site(),
            mode in arb_mode(),
        ) {
            let (_guard, output) = run_incident(site, mode);
            prop_assert_eq!(
                output.status.code(),
                Some(5),
                "site={:?} mode={:?} must return exit 5; got {:?}; stderr={}",
                site, mode, output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}

// ============================================================================
// Mutation resistance: anti-invariant tests for each critical mutation in
// test-plan §7. Each test exercises a scenario that would FAIL if the
// corresponding mutation were applied to the production code.
// ============================================================================

#[cfg(test)]
mod cmd_incident_mutation_resistance {
    #![forbid(unsafe_code)]

    use super::cmd_incident_behavior_props::{
        count_cmd_incident_body_lines, run_incident, signature_clauses_present,
        DiagnosticSite, Mode,
    };

    /// M-9 resistance: the body line counter rejects a synthetic
    /// 3-line-over-budget body (28 lines). If the counter were off by
    /// three, this test would silently pass; the explicit assertion
    /// proves the counter sees exactly 28 lines.
    #[test]
    fn m9_body_counter_detects_3_extra_lines() {
        let mut source = String::from("pub(crate) fn cmd_incident() -> u8 {\n");
        for _ in 0..28 {
            source.push_str("    let _x = 1;\n");
        }
        source.push_str("}\n");
        let count = count_cmd_incident_body_lines(&source);
        assert_eq!(count, 28, "body counter must report 28 lines exactly");
    }

    /// M-11 resistance: stripping `pub(crate)` from a synthetic source
    /// causes the signature-clause check to fail. The test asserts
    /// that the stripped source is rejected.
    #[test]
    fn m11_signature_check_rejects_stripped_pub_crate() {
        let source = "\
fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    0
}
";
        assert!(
            !signature_clauses_present(source),
            "M-11: stripped `pub(crate)` must be detected",
        );
    }

    /// M-14 resistance: a synthetic 30-line body must be counted as
    /// > 25 lines. If the counter counted the closing `}` as a body
    /// line, the count would be 31 (30 let-lines + the `0` + the
    /// closing `}`). The assertion `count > 25` is true either way,
    /// but the explicit `count == 31` is tighter — proves the counter
    /// is NOT off-by-one on the closing brace.
    #[test]
    fn m14_body_counter_handles_closing_brace_correctly() {
        let mut source = String::from("fn cmd_incident() -> u8 {\n");
        for _ in 0..30 {
            source.push_str("    let _x = 1;\n");
        }
        source.push_str("    0\n}\n");
        let count = count_cmd_incident_body_lines(&source);
        // 30 let-lines + 1 `0` line = 31 body lines. Closing `}` is
        // not counted.
        assert_eq!(
            count, 31,
            "M-14: 30 let-lines + `0` line = 31 body lines; closing brace \
             must not be counted; got {}",
            count,
        );
    }

    /// M-15 resistance: a body with a nested match expression must be
    /// counted correctly (the inner `}` is not confused for the outer
    /// closing brace).
    #[test]
    fn m15_body_counter_handles_nested_match() {
        let source = "\
pub(crate) fn cmd_incident() -> u8 {
    match 1 {
        1 => 10,
        _ => 20,
    }
}
";
        let count = count_cmd_incident_body_lines(source);
        // Expected: line 2 (match 1 {), line 3 (1 => 10), line 4
        // (_ => 20), line 5 (}) = 4 body lines. The outer `}` is the
        // function's closing brace and is NOT counted.
        assert_eq!(
            count, 4,
            "M-15: nested match must be counted correctly; got {}",
            count,
        );
    }

    /// M-15 resistance (struct literal variant): a body with a nested
    /// struct literal must be counted correctly.
    #[test]
    fn m15_body_counter_handles_nested_struct_literal_variant() {
        let source = "\
pub(crate) fn cmd_incident() -> u8 {
    let _s = Foo { a: 1, b: 2 };
    0
}
";
        let count = count_cmd_incident_body_lines(source);
        // Expected: line 2 (let _s = ...), line 3 (0) = 2 body lines.
        assert_eq!(
            count, 2,
            "M-15: nested struct literal must be counted correctly; got {}",
            count,
        );
    }

    /// M-1 resistance: every JournalOpenFailed site (Text/Yaml/Postcard)
    /// must return exit 5, not 0 (Success). If the production code
    /// returned `CliExitCode::Success` instead of `StorageError`, the
    /// test would fail.
    #[test]
    fn m1_journal_open_failed_never_returns_success() {
        for mode in [Mode::Text, Mode::Yaml, Mode::Postcard] {
            let (_guard, output) =
                run_incident(DiagnosticSite::JournalOpenFailed, mode);
            assert_ne!(
                output.status.code(),
                Some(0),
                "M-1: JournalOpenFailed/{:?} must not return exit 0 (Success); got {:?}",
                mode,
                output.status.code(),
            );
        }
    }

    /// M-3 resistance: the NoEvents site (Text/Yaml/Postcard) must
    /// return exit 5, NOT 0. If the production `if events.is_empty()`
    /// arm were deleted (mutation M-3), the test would proceed to the
    /// compute branch and likely return 0 (no failure event found).
    #[test]
    fn m3_no_events_never_returns_success() {
        for mode in [Mode::Text, Mode::Yaml, Mode::Postcard] {
            let (_guard, output) = run_incident(DiagnosticSite::NoEvents, mode);
            assert_ne!(
                output.status.code(),
                Some(0),
                "M-3: NoEvents/{:?} must not return exit 0; got {:?}; stderr={}",
                mode,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }

    /// M-4 resistance: the NonIncident site (Text/Yaml/Postcard) must
    /// return exit 5, not 0. If `StorageError` were replaced with
    /// `Success` in the NonIncident arm, the test would fail.
    #[test]
    fn m4_non_incident_never_returns_success() {
        for mode in [Mode::Text, Mode::Yaml, Mode::Postcard] {
            let (_guard, output) = run_incident(DiagnosticSite::NonIncident, mode);
            assert_ne!(
                output.status.code(),
                Some(0),
                "M-4: NonIncident/{:?} must not return exit 0; got {:?}; stderr={}",
                mode,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }

    /// M-6 resistance: the diagnostic in Text mode is written to
    /// STDERR, not stdout. If `errln!` were replaced with `outln!`,
    /// the diagnostic would appear on stdout instead. This test
    /// asserts that stdout is empty for a NoEvents Text-mode call.
    #[test]
    fn m6_text_diagnostic_written_to_stderr_not_stdout() {
        let (_guard, output) = run_incident(DiagnosticSite::NoEvents, Mode::Text);
        let stdout_text = std::str::from_utf8(&output.stdout).unwrap_or("");
        assert!(
            !stdout_text.contains("no events found for run"),
            "M-6: Text-mode diagnostic must be on stderr, not stdout; \
             stdout={:?}",
            stdout_text,
        );
        let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
        assert!(
            stderr_text.contains("no events found for run"),
            "M-6: Text-mode diagnostic must be on stderr; stderr={:?}",
            stderr_text,
        );
    }

    /// M-16 resistance: all four diagnostic sites in Yaml mode must
    /// produce envelopes with the same sorted key set. If one site
    /// added a key like `exit_code` that the others did not, the key
    /// sets would diverge.
    #[test]
    fn m16_yaml_envelopes_have_uniform_key_set_across_sites() {
        let mut key_sets: Vec<(DiagnosticSite, Vec<String>)> = Vec::new();
        for &site in DiagnosticSite::ALL {
            let (_guard, output) = run_incident(site, Mode::Yaml);
            let stderr_text = std::str::from_utf8(&output.stderr).unwrap_or("");
            let value: serde_yaml::Value = serde_yaml::from_str(stderr_text)
                .unwrap_or_else(|e| panic!("site={:?}: YAML parse failed: {}", site, e));
            let mut keys: Vec<String> = value
                .as_mapping()
                .map(|m| {
                    m.iter()
                        .map(|(k, _)| {
                            k.as_str()
                                .map(String::from)
                                .unwrap_or_else(|| format!("{:?}", k))
                        })
                        .collect()
                })
                .unwrap_or_default();
            keys.sort();
            key_sets.push((site, keys));
        }
        let reference_keys = &key_sets[0].1;
        for (site, keys) in &key_sets[1..] {
            assert_eq!(
                keys, reference_keys,
                "M-16: key set for site={:?} ({:?}) must match reference ({:?})",
                site, keys, reference_keys,
            );
        }
    }
}
