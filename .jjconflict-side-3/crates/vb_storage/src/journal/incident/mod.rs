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
            JournalEvent::ActionAbandoned { ticket, .. } => {
                // ActionAbandoned is a run-cancellation side effect;
                // surface it as a confirmed side effect so downstream
                // rollback / compensation logic sees the cancellation.
                // The ticket carries the real step/action identity; never
                // synthesize 0, which reads as blaming step 0.
                side_effects.push(SideEffect {
                    step: ticket.step.get(),
                    action: ticket.action.get(),
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
            JournalEvent::StepFailed { step, .. } => {
                failure_found = true;
                failure_code = "StepFailed".to_string();
                failed_at_step = Some(step.get());
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
        "RunFailed" | "StepFailed" => {
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
/// Scans events in reverse and returns the lifecycle state implied by the
/// last non-neutral event. Policy decision events (`PolicyDecisionEvent`),
/// gate decision events (`GateDecisionEvent`), and gate opened events
/// (`GateOpenedEvent`) are neutral facts that do not affect lifecycle; they
/// are skipped during derivation so a terminal state such as `Failed`,
/// `Cancelled`, or `Completed` is not overwritten by a subsequent policy or
/// gate event.
///
/// Every known `JournalEvent` variant is enumerated explicitly in the
/// `event_to_lifecycle` mapping:
///
/// - `RunCancelled`, `RunKilled` → `Cancelled` (terminal)
/// - `RunFinished`, `RunAnswered` → `Completed` (terminal)
/// - `RunFailedEvent`, `StepFailed`, `ActionFailedEvent` → `Failed` (non-terminal; retry may
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
/// If no events exist or all events are neutral, defaults to `Pending`.
#[must_use]
pub fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState {
    for event in events.iter().rev() {
        if is_lifecycle_affecting(event) {
            return event_to_lifecycle(event);
        }
    }
    LifecycleState::Pending
}

/// Returns true if the event affects lifecycle state derivation.
///
/// Policy decision, gate decision, gate opened, compatibility decision,
/// tenant admission, and capability grant events are neutral facts and do
/// not affect lifecycle.
#[inline]
fn is_lifecycle_affecting(event: &JournalEvent) -> bool {
    !matches!(
        event,
        JournalEvent::PolicyDecisionEvent { .. }
            | JournalEvent::GateDecisionEvent { .. }
            | JournalEvent::GateOpenedEvent { .. }
            | JournalEvent::CompatibilityDecisionEvent { .. }
            | JournalEvent::TenantAdmissionEvent { .. }
            | JournalEvent::RunContinuation { .. }
    )
}

/// Map a single `JournalEvent` to the lifecycle state implied by that event.
///
/// Enumerates every known variant of `JournalEvent` so that adding a new
/// variant produces a compile-time error in the defining crate.
#[must_use]
pub fn event_to_lifecycle(event: &JournalEvent) -> LifecycleState {
    match event {
        JournalEvent::RunContinuation { .. } => LifecycleState::Active,
        JournalEvent::RunAccepted { .. } => LifecycleState::Active,
        JournalEvent::RunAdmission { .. } => LifecycleState::Active,
        JournalEvent::RunAdmissionBudget { .. } => LifecycleState::Active,
        JournalEvent::RetryCheckpoint { .. } => LifecycleState::Active,
        JournalEvent::StepStarted { .. } => LifecycleState::Active,
        JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
        JournalEvent::StepFailed { .. } => LifecycleState::Failed,
        JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
        JournalEvent::ActionScheduledTicket { .. } => LifecycleState::Active,
        JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
        JournalEvent::ActionCompletedEnvelope { .. } => LifecycleState::Active,
        JournalEvent::ActionAbandoned { .. } => LifecycleState::Cancelled,
        JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
        JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
        JournalEvent::RetryTimerScheduled { .. } => LifecycleState::WaitingAnswer,
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
        JournalEvent::PolicyDecisionEvent { .. } => LifecycleState::Active,
        JournalEvent::GateDecisionEvent { .. } => LifecycleState::Active,
        JournalEvent::GateOpenedEvent { .. } => LifecycleState::Active,
        JournalEvent::CompatibilityDecisionEvent { .. } => LifecycleState::Active,
        JournalEvent::TenantAdmissionEvent { .. } => LifecycleState::Active,
    }
}

#[cfg(test)]
mod tests;
