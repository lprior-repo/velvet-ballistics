#![forbid(unsafe_code)]
//! Incident analysis and lifecycle state derivation for workflow runs.
//!
//! Domain logic for analyzing journal events.

use crate::events::JournalEvent;
#[allow(unused_imports)]
use vb_core::{ActionId, RunId, StepIdx, workflow::LifecycleState};

/// Side effect recorded from an action event.
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: u16,
    pub action: u16,
    pub certainty: SideEffectCertainty,
}

/// Whether an action succeeded or failed.
#[derive(Debug, Clone)]
pub enum SideEffectCertainty {
    Confirmed,
    Failed,
}

/// Incident analysis result from scanning journal events.
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: String,
    pub failed_at_step: Option<u16>,
    pub side_effects: Vec<SideEffect>,
}

/// Build incident analysis from a run's event stream.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut failure_found = false;
    let mut failure_code = String::new();
    let mut failed_at_step: Option<u16> = None;
    let mut side_effects: Vec<SideEffect> = Vec::new();
    let mut last_step_started: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                last_step_started = Some(step.get());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Confirmed,
                });
            }
            JournalEvent::ActionAbandoned { .. } => {
                // ActionAbandoned is a run-cancellation side effect;
                // surface it as a confirmed side effect so downstream
                // rollback / compensation logic sees the cancellation.
                side_effects.push(SideEffect {
                    step: 0,
                    action: 0,
                    certainty: SideEffectCertainty::Confirmed,
                });
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Failed,
                });
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

    IncidentAnalysis {
        failure_found,
        failure_code,
        failed_at_step,
        side_effects,
    }
}

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[SideEffect],
    failed_at_step: Option<u16>,
) -> Vec<String> {
    let mut hints: Vec<String> = Vec::new();

    match failure_code {
        "RunFailed" => {
            hints.push("investigate step output and engine logs for the failed step".to_string());
            if !side_effects.is_empty() {
                hints.push(
                    "review side effects that completed before failure for compensating actions"
                        .to_string(),
                );
            }
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "consider retry from step {step} using the retry command"
                ));
            }
        }
        "RunCancelled" => {
            hints.push("run was cancelled; check if cancellation was intentional".to_string());
            if !side_effects.is_empty() {
                hints.push("review completed side effects for partial cleanup needs".to_string());
            }
        }
        _ => {}
    }

    hints
}

/// Maps a lifecycle state to a human-readable status string for the inspect command.
///
/// Terminal states map to their name; Active/WaitingAnswer map to "running".
#[must_use]
pub fn lifecycle_state_to_inspect_status(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Cancelled => "cancelled",
        LifecycleState::Completed => "finished",
        LifecycleState::Failed => "failed",
        LifecycleState::Pending | LifecycleState::Active | LifecycleState::WaitingAnswer => {
            "running"
        }
        _ => "running",
    }
}

/// Derives the final lifecycle state from a sequence of journal events.
///
/// The last event in the sequence determines the final state. Every known
/// `JournalEvent` variant is enumerated explicitly:
///
/// - `RunCancelled`, `RunKilled` → `Cancelled` (terminal)
/// - `RunFinished`, `RunAnswered` → `Completed` (terminal)
/// - `RunFailedEvent`, `ActionFailedEvent` → `Failed` (non-terminal; retry may
///   transition a run away from `Failed`)
/// - `WaitScheduledEvent`, `AskScheduledEvent`, `AskAnsweredEvent` →
///   `WaitingAnswer`
/// - All other variants (`RunAccepted`, `RunAdmission`, `StepStarted`,
///   `StepSucceeded`, `ActionScheduled`, `ActionScheduledTicket`,
///   `ActionCompletedEvent`, `ActionCompletedEnvelope`, `SlotWrittenEvent`,
///   `WaitResolvedEvent`, `RetryScheduledEvent`, `RunResumed`, `RunRetried`,
///   `AskTimedOutEvent`) → `Active`
///
/// No wildcard arm is used. `JournalEvent` is `#[non_exhaustive]`, but the
/// compiler still treats a match within the defining crate as exhaustive
/// when every variant is enumerated. If a new variant is added later the
/// build will fail, forcing it to be handled explicitly. Downstream crates
/// that consume this function may keep their own wildcards.
///
/// If no events exist, defaults to `Pending`.
#[must_use]
pub fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState {
    events.last().map(event_to_lifecycle).unwrap_or(LifecycleState::Pending)
}


/// Map a single `JournalEvent` to the lifecycle state implied by that event.
#[must_use]
pub fn event_to_lifecycle(event: &JournalEvent) -> LifecycleState {
    match event {
        JournalEvent::RunAccepted { .. } => LifecycleState::Active,
        JournalEvent::RunAdmission { .. } => LifecycleState::Active,
        JournalEvent::StepStarted { .. } => LifecycleState::Active,
        JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
        JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
        JournalEvent::ActionScheduledTicket { .. } => LifecycleState::Active,
        JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
        JournalEvent::ActionCompletedEnvelope { .. } => LifecycleState::Active,
        JournalEvent::ActionAbandoned { .. } => LifecycleState::Cancelled,
        JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
        JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
        JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
        JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
        JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
        JournalEvent::WaitResolvedEvent { .. } => LifecycleState::Active,
        JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
        JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
        JournalEvent::RunKilled { .. } => LifecycleState::Cancelled,
        JournalEvent::RunFinished { .. } => LifecycleState::Completed,
        JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
        JournalEvent::RunResumed { .. } => LifecycleState::Active,
        JournalEvent::RunRetried { .. } => LifecycleState::Active,
        JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
        JournalEvent::AskTimedOutEvent { .. } => LifecycleState::Active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use crate::JournalEvent;

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

    // ---- T-009: RunFailed repair hints (1 hint) ----
    #[test]
    fn t_009_run_failed_1_hint() {
        let hints = build_repair_hints("RunFailed", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
    }

    // ---- T-010: RunFailed repair hints (3 hints) ----
    #[test]
    fn t_010_run_failed_3_hints() {
        let side_effects = vec![SideEffect {
            step: 1,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunFailed", &side_effects, Some(3));
        assert_eq!(hints.len(), 3);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
        assert_eq!(
            hints[1],
            "review side effects that completed before failure for compensating actions"
        );
        assert_eq!(
            hints[2],
            "consider retry from step 3 using the retry command"
        );
    }

    // ---- T-011: RunCancelled repair hints (1 hint) ----
    #[test]
    fn t_011_run_cancelled_1_hint() {
        let hints = build_repair_hints("RunCancelled", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
    }

    // ---- T-012: RunCancelled repair hints (2 hints) ----
    #[test]
    fn t_012_run_cancelled_2_hints() {
        let side_effects = vec![SideEffect {
            step: 2,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunCancelled", &side_effects, None);
        assert_eq!(hints.len(), 2);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
        assert_eq!(
            hints[1],
            "review completed side effects for partial cleanup needs"
        );
    }

    // ---- T-013: Unknown failure code (0 hints) ----
    #[test]
    fn t_013_unknown_failure_code() {
        let hints = build_repair_hints("UnknownError", &[], None);
        assert!(hints.is_empty());
    }
}
