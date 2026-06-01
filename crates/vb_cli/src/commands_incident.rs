#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Incident report computation for CLI output.
//!
//! Delegates domain analysis to vb_storage::journal::incident.
//! This CLI module builds the CLI-specific IncidentReport with JSON values.

use vb_storage::events::JournalEvent;
use vb_storage::journal::incident::{
    SideEffectCertainty, analyze_incident_events, build_repair_hints,
};

/// Structured incident report for CLI output.
pub(crate) struct IncidentReport {
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
pub(crate) fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport {
    let analysis = analyze_incident_events(events);
    let hints = build_repair_hints(
        &analysis.failure_code,
        &analysis.side_effects,
        analysis.failed_at_step,
    );

    IncidentReport {
        run_id: run_id.to_string(),
        failure_code: analysis.failure_code,
        failure_found: analysis.failure_found,
        failed_at_step: analysis.failed_at_step,
        side_effects: analysis
            .side_effects
            .into_iter()
            .map(|se| {
                serde_json::json!({
                    "step": se.step,
                    "action": se.action,
                    "certainty": match se.certainty {
                        SideEffectCertainty::Confirmed => "confirmed",
                        SideEffectCertainty::Failed => "failed",
                    }
                })
            })
            .collect(),
        repair_hints: hints.into_iter().map(serde_json::Value::String).collect(),
    }
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

    // T-009 through T-013 removed: build_repair_hints logic is now tested
    // in vb_storage::journal::incident (domain tests). CLI tests cover the
    // full build_incident_report pipeline which includes repair hints.
}
