#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure incident report computation logic, separated from I/O and formatting.

use vb_storage::events::JournalEvent;

/// Structured incident report for a single run.
pub struct IncidentReport {
    /// The run id string (as provided by the caller).
    pub run_id: String,
    /// Failure code, e.g. "RunFailed" or "RunCancelled". Empty if no failure.
    pub failure_code: String,
    /// Whether a failure event was found.
    pub failure_found: bool,
    /// Step at which the failure occurred, if known.
    pub failed_at_step: Option<u16>,
    /// Side effects collected from action completed/failed events.
    pub side_effects: Vec<serde_json::Value>,
    /// Repair hints based on failure type.
    pub repair_hints: Vec<serde_json::Value>,
}

/// Build an incident report from a run's event stream.
pub fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport {
    let mut failure_found = false;
    let mut failure_code = String::new();
    let mut failed_at_step: Option<u16> = None;
    let mut side_effects: Vec<serde_json::Value> = Vec::new();
    let mut last_step_started: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                last_step_started = Some(step.get());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                side_effects.push(serde_json::json!({
                    "step": step.get(),
                    "action": action.get(),
                    "certainty": "confirmed"
                }));
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                side_effects.push(serde_json::json!({
                    "step": step.get(),
                    "action": action.get(),
                    "certainty": "failed"
                }));
            }
            JournalEvent::RunFailedEvent { .. } => {
                failure_found = true;
                failure_code = "RunFailed".to_string();
                failed_at_step = last_step_started;
            }
            JournalEvent::RunCancelled { .. } => {
                failure_found = true;
                failure_code = "RunCancelled".to_string();
                failed_at_step = last_step_started;
            }
            _ => {}
        }
    }

    let repair_hints = build_repair_hints(&failure_code, &side_effects, failed_at_step);

    IncidentReport {
        run_id: run_id.to_string(),
        failure_code,
        failure_found,
        failed_at_step,
        side_effects,
        repair_hints,
    }
}

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[serde_json::Value],
    failed_at_step: Option<u16>,
) -> Vec<serde_json::Value> {
    let mut hints: Vec<serde_json::Value> = Vec::new();

    match failure_code {
        "RunFailed" => {
            hints.push(serde_json::Value::String(
                "investigate step output and engine logs for the failed step".to_string(),
            ));
            if !side_effects.is_empty() {
                hints.push(serde_json::Value::String(
                    "review side effects that completed before failure for compensating actions"
                        .to_string(),
                ));
            }
            if let Some(step) = failed_at_step {
                hints.push(serde_json::Value::String(format!(
                    "consider retry from step {step} using the retry command"
                )));
            }
        }
        "RunCancelled" => {
            hints.push(serde_json::Value::String(
                "run was cancelled; check if cancellation was intentional".to_string(),
            ));
            if !side_effects.is_empty() {
                hints.push(serde_json::Value::String(
                    "review completed side effects for partial cleanup needs".to_string(),
                ));
            }
        }
        _ => {}
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::{ActionId, RunId, StepIdx};
    use vb_storage::{EventSeq, JournalEvent};

    /// Helper: create a minimal StepStarted event.
    fn step_event(step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    /// Helper: create a minimal ActionCompletedEvent.
    fn action_completed(step: u16, action: u16) -> JournalEvent {
        JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    /// Helper: create a minimal ActionFailedEvent.
    fn action_failed(step: u16, action: u16) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    /// Helper: create a RunFailedEvent.
    fn run_failed() -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        }
    }

    /// Helper: create a RunCancelled event.
    fn run_cancelled() -> JournalEvent {
        JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        }
    }

    // ---- T-001: Empty events ----
    #[test]
    fn t_001_empty_events() {
        let report = build_incident_report("run-1", &[]);
        assert!(!report.failure_found);
        assert_eq!(report.failure_code, "");
        assert!(report.failed_at_step.is_none());
        assert!(report.side_effects.is_empty());
    }

    // ---- T-002: RunFailedEvent ----
    #[test]
    fn t_002_run_failed_event() {
        let events = vec![step_event(1), run_failed()];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(1));
    }

    // ---- T-003: RunCancelled ----
    #[test]
    fn t_003_run_cancelled() {
        let events = vec![step_event(1), step_event(2), run_cancelled()];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunCancelled");
        assert_eq!(report.failed_at_step, Some(2));
    }

    // ---- T-004: ActionCompletedEvent side effects ----
    #[test]
    fn t_004_action_completed_side_effects() {
        let events = vec![action_completed(1, 100)];
        let report = build_incident_report("run-1", &events);
        assert!(!report.failure_found);
        assert_eq!(report.side_effects.len(), 1);
        assert_eq!(report.side_effects[0]["step"], 1);
        assert_eq!(report.side_effects[0]["action"], 100);
        assert_eq!(report.side_effects[0]["certainty"], "confirmed");
    }

    // ---- T-005: ActionFailedEvent side effects ----
    #[test]
    fn t_005_action_failed_side_effects() {
        let events = vec![action_failed(2, 200)];
        let report = build_incident_report("run-1", &events);
        assert!(!report.failure_found);
        assert_eq!(report.side_effects.len(), 1);
        assert_eq!(report.side_effects[0]["certainty"], "failed");
    }

    // ---- T-006: Multiple events ----
    #[test]
    fn t_006_multiple_events() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            action_failed(1, 20),
            step_event(2),
            action_completed(2, 30),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(2));
        assert_eq!(report.side_effects.len(), 3);
    }

    // ---- T-007: Multiple StepStarted tracking ----
    #[test]
    fn t_007_multiple_step_started_tracking() {
        let events = vec![
            step_event(1),
            step_event(3),
            step_event(5),
            step_event(7),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        // failed_at_step should be the last step_started (7)
        assert_eq!(report.failed_at_step, Some(7));
    }

    // ---- T-008: Mixed events full report ----
    #[test]
    fn t_008_mixed_events_full_report() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            step_event(2),
            action_failed(2, 20),
            step_event(3),
            action_completed(3, 30),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(3));
        assert_eq!(report.side_effects.len(), 3);
        assert!(!report.repair_hints.is_empty());
    }

    // ---- T-009: RunFailed repair hints (1 hint) ----
    #[test]
    fn t_009_run_failed_1_hint() {
        let hints = build_repair_hints("RunFailed", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0].as_str(),
            Some("investigate step output and engine logs for the failed step")
        );
    }

    // ---- T-010: RunFailed repair hints (3 hints) ----
    #[test]
    fn t_010_run_failed_3_hints() {
        let side_effects = vec![serde_json::json!({"step": 1})];
        let hints = build_repair_hints("RunFailed", &side_effects, Some(3));
        assert_eq!(hints.len(), 3);
        assert_eq!(
            hints[0].as_str(),
            Some("investigate step output and engine logs for the failed step")
        );
        assert_eq!(
            hints[1].as_str(),
            Some("review side effects that completed before failure for compensating actions")
        );
        assert_eq!(
            hints[2].as_str(),
            Some("consider retry from step 3 using the retry command")
        );
    }

    // ---- T-011: RunCancelled repair hints (1 hint) ----
    #[test]
    fn t_011_run_cancelled_1_hint() {
        let hints = build_repair_hints("RunCancelled", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0].as_str(),
            Some("run was cancelled; check if cancellation was intentional")
        );
    }

    // ---- T-012: RunCancelled repair hints (2 hints) ----
    #[test]
    fn t_012_run_cancelled_2_hints() {
        let side_effects = vec![serde_json::json!({"step": 2})];
        let hints = build_repair_hints("RunCancelled", &side_effects, None);
        assert_eq!(hints.len(), 2);
        assert_eq!(
            hints[0].as_str(),
            Some("run was cancelled; check if cancellation was intentional")
        );
        assert_eq!(
            hints[1].as_str(),
            Some("review completed side effects for partial cleanup needs")
        );
    }

    // ---- T-013: Unknown failure code (0 hints) ----
    #[test]
    fn t_013_unknown_failure_code() {
        let hints = build_repair_hints("UnknownError", &[], None);
        assert!(hints.is_empty());
    }
}
