//! Incident data model: side effects and analysis results.
//!
//! Types and the core event-analysis pipeline that produces an
//! [`IncidentAnalysis`] from a journal event stream.

use crate::events::JournalEvent;

/// Whether an action succeeded or failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectCertainty {
    Confirmed,
    Failed,
}

/// Side effect recorded from an action event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffect {
    pub step: u16,
    pub action: u16,
    pub certainty: SideEffectCertainty,
}

/// Incident analysis result from scanning journal events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: String,
    pub failed_at_step: Option<u16>,
    pub side_effects: Vec<SideEffect>,
}

/// Build incident analysis from a run's event stream.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut result = IncidentAnalysis::default();
    let mut last_step_started: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                last_step_started = Some(step.get());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                result.side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Confirmed,
                });
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                result.side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Failed,
                });
            }
            JournalEvent::RunFailedEvent { .. } => {
                result.failure_found = true;
                result.failure_code = "RunFailed".to_string();
                result.failed_at_step = last_step_started;
            }
            JournalEvent::RunCancelled { .. } => {
                result.failure_found = true;
                result.failure_code = "RunCancelled".to_string();
                result.failed_at_step = last_step_started;
            }
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventSeq, JournalEvent};
    use vb_core::{ActionId, RunId, StepIdx};

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
        let analysis = analyze_incident_events(&[]);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.failure_code, "");
        assert!(analysis.failed_at_step.is_none());
        assert!(analysis.side_effects.is_empty());
    }

    // ---- T-002: RunFailedEvent ----
    #[test]
    fn t_002_run_failed_event() {
        let events = vec![step_event(1), run_failed()];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(1));
    }

    // ---- T-003: RunCancelled ----
    #[test]
    fn t_003_run_cancelled() {
        let events = vec![step_event(1), step_event(2), run_cancelled()];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunCancelled");
        assert_eq!(analysis.failed_at_step, Some(2));
    }

    // ---- T-004: ActionCompletedEvent side effects ----
    #[test]
    fn t_004_action_completed_side_effects() {
        let events = vec![action_completed(1, 100)];
        let analysis = analyze_incident_events(&events);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.side_effects.len(), 1);
        assert_eq!(analysis.side_effects[0].step, 1);
        assert_eq!(analysis.side_effects[0].action, 100);
        assert!(matches!(
            analysis.side_effects[0].certainty,
            SideEffectCertainty::Confirmed
        ));
    }

    // ---- T-005: ActionFailedEvent side effects ----
    #[test]
    fn t_005_action_failed_side_effects() {
        let events = vec![action_failed(2, 200)];
        let analysis = analyze_incident_events(&events);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.side_effects.len(), 1);
        assert!(matches!(
            analysis.side_effects[0].certainty,
            SideEffectCertainty::Failed
        ));
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
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(2));
        assert_eq!(analysis.side_effects.len(), 3);
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
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failed_at_step, Some(7));
    }

    // ---- T-008: Mixed events ----
    #[test]
    fn t_008_mixed_events() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            step_event(2),
            action_failed(2, 20),
            step_event(3),
            action_completed(3, 30),
            run_failed(),
        ];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(3));
        assert_eq!(analysis.side_effects.len(), 3);
    }
}
