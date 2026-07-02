#![forbid(unsafe_code)]
//! Incident analysis and lifecycle state derivation for workflow runs.
//!
//! Domain logic for analyzing journal events.
//!
//! ## Failure-code vocabulary
//!
//! The analyzer classifies every modern `JournalEvent` variant. Failure
//! codes are kept as stable strings so CLI consumers (and the JSON
//! envelope) can pin them by name. The vocabulary is:
//!
//! | Code              | Emitted by                          |
//! |-------------------|--------------------------------------|
//! | (empty)           | no failure event observed            |
//! | `RunFailed`       | `RunFailedEvent`                    |
//! | `RunCancelled`    | `RunCancelled`                       |
//! | `RunKilled`       | `RunKilled`                          |
//! | `ActionAbandoned` | `ActionAbandoned`                    |
//! | `AskTimedOut`     | `AskTimedOutEvent`                   |
//!
//! All other modern variants
//! (`ActionScheduledTicket`, `ActionCompletedEnvelope`,
//! `WaitResolvedEvent`) are treated as normal progression and
//! contribute evidence (side effects, step-started tracking) without
//! triggering incident emission.

use crate::events::JournalEvent;
use vb_core::{
    ActionId, RunId, StepIdx,
    action::{ActionTicket, compute_action_idempotency_key},
    workflow::LifecycleState,
};

/// Stable failure-code vocabulary returned by [`analyze_incident_events`].
///
/// Centralizing these as a typed enum lets the CLI and tests pin exact
/// string literals without hard-coding each call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    /// No failure event was present in the run's event stream.
    None,
    /// `RunFailedEvent` was observed — the workflow engine reported a failure.
    RunFailed,
    /// `RunCancelled` was observed — the run was cancelled, possibly intentionally.
    RunCancelled,
    /// `RunKilled` was observed — runtime kill event terminated the run.
    RunKilled,
    /// `ActionAbandoned` was observed — a pending action was abandoned because
    /// the run was cancelled/killed before the action boundary completed.
    ActionAbandoned,
    /// `AskTimedOutEvent` was observed — an ask timed out and resumed along the
    /// ask-timeout path.
    AskTimedOut,
}

impl FailureCode {
    /// Stable string label, suitable for CLI JSON and repair-hint tags.
    ///
    /// Returns an empty string for [`FailureCode::None`] so callers can
    /// use the same field for both the "is this an incident?" probe and
    /// the diagnostic display.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FailureCode::None => "",
            FailureCode::RunFailed => "RunFailed",
            FailureCode::RunCancelled => "RunCancelled",
            FailureCode::RunKilled => "RunKilled",
            FailureCode::ActionAbandoned => "ActionAbandoned",
            FailureCode::AskTimedOut => "AskTimedOut",
        }
    }

    /// Whether this code represents an actual incident (vs. a clean run).
    #[must_use]
    pub const fn is_incident(self) -> bool {
        !matches!(self, FailureCode::None)
    }
}

/// Side effect recorded from an action event.
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: u16,
    pub action: u16,
    pub certainty: SideEffectCertainty,
}

/// Evidence attached to an [`IncidentAnalysis`] for the modern
/// action-ticket / envelope variants. Each entry is a minimal projection
/// of one terminal action event (either a durable success envelope or
/// an abandonment) so the CLI's incident report can surface the exact
/// ticket that was issued for a given step.
#[derive(Debug, Clone)]
pub struct ActionTicketEvidence {
    /// Step that issued the action.
    pub step: u16,
    /// Action identifier.
    pub action: u16,
    /// Attempt number (1-indexed).
    pub attempt: u16,
    /// Whether this entry was emitted by an abandonment (true) or a
    /// modern completion envelope (false).
    pub abandoned: bool,
    /// Capacity from the abandoned ticket, if any. Used to render the
    /// "abandoned before completing ticket capacity" hint.
    pub capacity: u16,
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
    /// Modern action-ticket evidence collected from
    /// `ActionScheduledTicket`, `ActionCompletedEnvelope`, and
    /// `ActionAbandoned` events so the CLI can surface the ticket as
    /// part of the incident report.
    pub action_evidence: Vec<ActionTicketEvidence>,
}

/// Build incident analysis from a run's event stream.
///
/// The walk enumerates every modern `JournalEvent` variant explicitly
/// so that adding a new variant produces a compile-time error in this
/// defining crate. Failures are captured with [`FailureCode`] values
/// pinned to exact strings; the analyzer never invents new codes
/// silently. Action-ticket evidence is collected for the modern
/// envelope variants (`ActionScheduledTicket`,
/// `ActionCompletedEnvelope`, `ActionAbandoned`) so the CLI can
/// surface ticket data alongside the legacy side-effect list.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut failure_code = FailureCode::None;
    let mut failed_at_step: Option<u16> = None;
    let mut side_effects: Vec<SideEffect> = Vec::new();
    let mut action_evidence: Vec<ActionTicketEvidence> = Vec::new();
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
            JournalEvent::ActionScheduledTicket {
                ticket,
                action_abi_digest: _,
                ..
            } => {
                action_evidence.push(ActionTicketEvidence {
                    step: ticket.step.get(),
                    action: ticket.action.get(),
                    attempt: ticket.attempt,
                    abandoned: false,
                    capacity: ticket.capacity,
                });
            }
            JournalEvent::ActionCompletedEnvelope {
                ticket,
                outcome,
                value,
                encoded_len: _,
                taint: _,
                value_digest: _,
                action_abi_digest: _,
                ..
            } => {
                action_evidence.push(ActionTicketEvidence {
                    step: ticket.step.get(),
                    action: ticket.action.get(),
                    attempt: ticket.attempt,
                    abandoned: false,
                    capacity: ticket.capacity,
                });
                // Only count a side effect when the durable outcome
                // committed successfully and the bytes look coherent.
                // We do not surface failure outcomes through the
                // legacy side-effect list — failure code already
                // captures those.
                if *outcome == crate::DurableActionOutcome::Ready && !value.is_empty() {
                    side_effects.push(SideEffect {
                        step: ticket.step.get(),
                        action: ticket.action.get(),
                        certainty: SideEffectCertainty::Confirmed,
                    });
                }
            }
            JournalEvent::ActionAbandoned { ticket, .. } => {
                action_evidence.push(ActionTicketEvidence {
                    step: ticket.step.get(),
                    action: ticket.action.get(),
                    attempt: ticket.attempt,
                    abandoned: true,
                    capacity: ticket.capacity,
                });
                failure_code = FailureCode::ActionAbandoned;
                failed_at_step = last_step_started;
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Failed,
                });
            }
            JournalEvent::WaitResolvedEvent { step, attempt, .. } => {
                // WaitResolvedEvent signals a successful resumption from
                // an external wait; record side-effect evidence at the
                // resolved step so downstream rollback logic can see
                // the wait resolution. This is non-incident behaviour.
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: u16::from(*attempt),
                    certainty: SideEffectCertainty::Confirmed,
                });
            }
            JournalEvent::AskTimedOutEvent { .. } => {
                failure_code = FailureCode::AskTimedOut;
                failed_at_step = last_step_started;
            }
            JournalEvent::RunFailedEvent { .. } => {
                failure_code = FailureCode::RunFailed;
                failed_at_step = last_step_started;
            }
            JournalEvent::RunCancelled { .. } => {
                failure_code = FailureCode::RunCancelled;
                failed_at_step = last_step_started;
            }
            JournalEvent::RunKilled { .. } => {
                failure_code = FailureCode::RunKilled;
                failed_at_step = last_step_started;
            }
            // Non-incident progression events: emit nothing but walk past.
            JournalEvent::RunAccepted { .. }
            | JournalEvent::RunAdmission { .. }
            | JournalEvent::StepSucceeded { .. }
            | JournalEvent::ActionScheduled { .. }
            | JournalEvent::SlotWrittenEvent { .. }
            | JournalEvent::WaitScheduledEvent { .. }
            | JournalEvent::AskScheduledEvent { .. }
            | JournalEvent::AskAnsweredEvent { .. }
            | JournalEvent::RetryScheduledEvent { .. }
            | JournalEvent::RunFinished { .. }
            | JournalEvent::RunResumed { .. }
            | JournalEvent::RunRetried { .. }
            | JournalEvent::RunAnswered { .. } => {}
        }
    }

    IncidentAnalysis {
        failure_found: failure_code.is_incident(),
        failure_code: failure_code.as_str().to_string(),
        failed_at_step,
        side_effects,
        action_evidence,
    }
}

/// Build a minimal [`ActionTicket`] for tests and replay seeds.
///
/// The capacity and idempotency-key fields are deterministic; callers
/// may override them when constructing fixtures that need to drive
/// specific validation paths.
#[must_use]
pub fn sample_action_ticket(
    run: RunId,
    step: StepIdx,
    seq: vb_core::SeqNo,
    action: ActionId,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, seq, action),
        capacity: 1,
    }
}

/// Build repair hints based on the failure code, side effects, and failed step.
///
/// Hints are exact-string lists so callers can pin them with
/// `assert_eq!`. Adding a new hint for a new failure code must be done
/// here (and reflected in `failure_code_hint_*` tests below); the
/// [`FailureCode`] enum pins the vocabulary itself.
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
        "RunKilled" => {
            hints.push(
                "run was killed by runtime; inspect kill reason and exit the run immediately"
                    .to_string(),
            );
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "review pending actions at step {step} and finalize with ActionAbandoned where applicable"
                ));
            }
            if !side_effects.is_empty() {
                hints.push(
                    "review side effects that completed before kill for compensation".to_string(),
                );
            }
        }
        "ActionAbandoned" => {
            hints.push(
                "action abandoned before completing; ticket capacity must be released without re-execution"
                    .to_string(),
            );
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "abandoned ticket belongs to step {step}; recovery can drop the resume queue entry"
                ));
            }
            if !side_effects.is_empty() {
                hints.push(
                    "review earlier side effects for partial rollback requirements before resuming"
                        .to_string(),
                );
            }
        }
        "AskTimedOut" => {
            hints.push(
                "ask timed out; increase ask deadline or supply a default answer to keep the run moving"
                    .to_string(),
            );
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "ask timeout occurred at step {step}; confirm the wait condition can be retried"
                ));
            }
        }
        _ => {}
    }

    hints
}

/// Pinned hint text emitted for [`FailureCode::ActionAbandoned`].
///
/// Exposed so the CLI test suite and any downstream tooling can pin the
/// exact hint string without depending on
/// `build_repair_hints` call ordering.
pub const HINT_ACTION_ABANDONED_PRIMARY: &str =
    "action abandoned before completing; ticket capacity must be released without re-execution";

/// Pinned hint template for the ask-timeout step fragment.
pub const HINT_ASK_TIMED_OUT_PRIMARY: &str =
    "ask timed out; increase ask deadline or supply a default answer to keep the run moving";

/// Pinned hint prefix emitted for [`FailureCode::RunKilled`].
pub const HINT_RUN_KILLED_PRIMARY: &str =
    "run was killed by runtime; inspect kill reason and exit the run immediately";

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
    events
        .last()
        .map(event_to_lifecycle)
        .unwrap_or(LifecycleState::Pending)
}

/// Map a single `JournalEvent` to the lifecycle state implied by that event.
///
/// Enumerates every known variant of `JournalEvent` so that adding a new
/// variant produces a compile-time error in the defining crate.
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

    /// Helper: create a RunKilled event.
    fn run_killed() -> JournalEvent {
        JournalEvent::RunKilled {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        }
    }

    /// Helper: create an ActionAbandoned event for the given ticket.
    fn action_abandoned(step: u16, action: u16, capacity: u16) -> JournalEvent {
        let seq = vb_core::SeqNo::new(9);
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(step),
            seq,
            action: ActionId::new(action),
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(
                RunId::new(1),
                seq,
                ActionId::new(action),
            ),
            capacity,
        };
        JournalEvent::ActionAbandoned {
            run: RunId::new(1),
            seq: EventSeq::new(9),
            ticket,
        }
    }

    /// Helper: create a minimal ActionCompletedEnvelope event.
    fn action_completed_envelope(step: u16, action: u16) -> JournalEvent {
        let seq = vb_core::SeqNo::new(5);
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(step),
            seq,
            action: ActionId::new(action),
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(
                RunId::new(1),
                seq,
                ActionId::new(action),
            ),
            capacity: 1,
        };
        JournalEvent::ActionCompletedEnvelope {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            ticket,
            output: vb_core::SlotIdx::new(2),
            outcome: crate::DurableActionOutcome::Ready,
            value: vec![0xAA],
            encoded_len: 1,
            taint: vb_core::Taint::Clean,
            value_digest: [0u8; 32],
            action_abi_digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        }
    }

    /// Helper: create an AskTimedOutEvent.
    fn ask_timed_out(step: u16) -> JournalEvent {
        JournalEvent::AskTimedOutEvent {
            run: RunId::new(1),
            seq: EventSeq::new(7),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    /// Helper: create a WaitResolvedEvent.
    fn wait_resolved(step: u16) -> JournalEvent {
        JournalEvent::WaitResolvedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(8),
            step: StepIdx::new(step),
            attempt: 1,
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

    // ---- T-014: ActionCompletedEnvelope modern evidence (non-incident) ----
    #[test]
    fn t_014_action_completed_envelope_modern_evidence() {
        let events = vec![step_event(2), action_completed_envelope(2, 42)];
        let analysis = analyze_incident_events(&events);
        // Envelope with a Ready outcome is non-incident.
        assert!(!analysis.failure_found);
        assert_eq!(analysis.failure_code, "");
        // The envelope contributes one side effect (confirmed at step 2).
        assert_eq!(analysis.side_effects.len(), 1);
        assert_eq!(analysis.side_effects[0].step, 2);
        assert_eq!(analysis.side_effects[0].action, 42);
        // And one ticket-evidence entry for the modern path.
        assert_eq!(analysis.action_evidence.len(), 1);
        assert!(!analysis.action_evidence[0].abandoned);
        assert_eq!(analysis.action_evidence[0].capacity, 1);
    }

    // ---- T-015: ActionAbandoned standalone failure ----
    #[test]
    fn t_015_action_abandoned_standalone() {
        let events = vec![step_event(3), action_abandoned(3, 7, 4)];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "ActionAbandoned");
        assert_eq!(analysis.failed_at_step, Some(3));
        assert_eq!(analysis.action_evidence.len(), 1);
        assert!(analysis.action_evidence[0].abandoned);
        assert_eq!(analysis.action_evidence[0].capacity, 4);
        assert_eq!(analysis.action_evidence[0].step, 3);
        assert_eq!(analysis.action_evidence[0].action, 7);
    }

    // ---- T-016: AskTimedOut standalone failure ----
    #[test]
    fn t_016_ask_timed_out_standalone() {
        let events = vec![step_event(5), ask_timed_out(5)];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "AskTimedOut");
        assert_eq!(analysis.failed_at_step, Some(5));
    }

    // ---- T-017: RunKilled standalone failure ----
    #[test]
    fn t_017_run_killed_standalone() {
        let events = vec![step_event(2), step_event(4), run_killed()];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunKilled");
        assert_eq!(analysis.failed_at_step, Some(4));
    }

    // ---- T-018: WaitResolved contributes non-incident side-effect ----
    #[test]
    fn t_018_wait_resolved_is_non_incident() {
        let events = vec![step_event(2), wait_resolved(2)];
        let analysis = analyze_incident_events(&events);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.failure_code, "");
        // WaitResolved contributes a confirmed side effect at the resolved step.
        assert_eq!(analysis.side_effects.len(), 1);
        assert_eq!(analysis.side_effects[0].step, 2);
        assert!(matches!(
            analysis.side_effects[0].certainty,
            SideEffectCertainty::Confirmed
        ));
    }

    // ---- T-019: ActionAbandoned repair hints are exact ----
    #[test]
    fn t_019_action_abandoned_hints_exact() {
        let side_effects = vec![SideEffect {
            step: 1,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("ActionAbandoned", &side_effects, Some(3));
        assert_eq!(hints.len(), 3);
        // Primary hint is the documented constant — and per-step / side-effect hints follow.
        assert_eq!(hints[0], HINT_ACTION_ABANDONED_PRIMARY);
        assert_eq!(
            hints[1],
            "abandoned ticket belongs to step 3; recovery can drop the resume queue entry"
        );
        assert_eq!(
            hints[2],
            "review earlier side effects for partial rollback requirements before resuming"
        );
    }

    // ---- T-020: AskTimedOut repair hints are exact ----
    #[test]
    fn t_020_ask_timed_out_hints_exact() {
        let hints = build_repair_hints("AskTimedOut", &[], Some(5));
        assert_eq!(hints.len(), 2);
        // Primary hint is the documented constant; step-fragment hint mentions step 5.
        assert_eq!(hints[0], HINT_ASK_TIMED_OUT_PRIMARY);
        assert_eq!(
            hints[1],
            "ask timeout occurred at step 5; confirm the wait condition can be retried"
        );
    }

    // ---- T-021: RunKilled repair hints are exact ----
    #[test]
    fn t_021_run_killed_hints_exact() {
        let hints = build_repair_hints("RunKilled", &[], Some(2));
        assert_eq!(hints.len(), 2);
        // Primary hint is the documented constant; step-fragment hint mentions step 2.
        assert_eq!(hints[0], HINT_RUN_KILLED_PRIMARY);
        assert_eq!(
            hints[1],
            "review pending actions at step 2 and finalize with ActionAbandoned where applicable"
        );
    }

    // ---- T-022: FailureCode vocabulary pins all six codes ----
    #[test]
    fn t_022_failure_code_vocabulary() {
        // The FailureCode enum pins the vocabulary so adding a new
        // code triggers a compile-time update at every consumer site.
        assert_eq!(FailureCode::None.as_str(), "");
        assert!(!FailureCode::None.is_incident());
        assert_eq!(FailureCode::RunFailed.as_str(), "RunFailed");
        assert!(FailureCode::RunFailed.is_incident());
        assert_eq!(FailureCode::RunCancelled.as_str(), "RunCancelled");
        assert!(FailureCode::RunCancelled.is_incident());
        assert_eq!(FailureCode::RunKilled.as_str(), "RunKilled");
        assert!(FailureCode::RunKilled.is_incident());
        assert_eq!(FailureCode::ActionAbandoned.as_str(), "ActionAbandoned");
        assert!(FailureCode::ActionAbandoned.is_incident());
        assert_eq!(FailureCode::AskTimedOut.as_str(), "AskTimedOut");
        assert!(FailureCode::AskTimedOut.is_incident());
    }
}
