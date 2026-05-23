//! Behavior tests for vb_cli diff and incident report functionality.
//!
//! Tests the pure computation logic in commands_diff and commands_incident modules.

use vb_cli::commands_diff::{
    compute_diff, diff_event_summary, event_name, events_differ,
};
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
    let events = vec![step_event(1, 1, 1), step_event(1, 2, 2), run_cancelled(1, 10)];
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
