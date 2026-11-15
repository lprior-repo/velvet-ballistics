#![forbid(unsafe_code)]
//! Incident analysis and lifecycle state derivation for workflow runs.
//!
//! Domain logic for analyzing journal events.

use crate::EventSeq;
use crate::events::JournalEvent;
#[allow(unused_imports)]
use vb_core::{ActionId, ActionTicket, RunId, StepIdx, WorkflowDigest, workflow::LifecycleState};

mod analysis;
pub use analysis::analyze_incident_events;

/// Side effect recorded from an action event.
#[derive(Debug, Clone)]
pub struct SideEffect {
    /// Run that owns this action fact.
    pub run: RunId,
    /// Durable event sequence for this fact.
    pub seq: EventSeq,
    /// Attempt number carried by the event (run attempt for ticket events).
    pub attempt: u16,
    /// Action step, when available in the event.
    pub step: u16,
    /// Action identity, when available in the event.
    pub action: u16,
    /// Full invocation identity, absent on sparse historical events.
    pub ticket: Option<ActionTicket>,
    /// Accepted action ABI digest, absent on historical or abandoned events.
    pub action_abi_digest: Option<WorkflowDigest>,
    /// Recorded phase of this action fact.
    pub certainty: SideEffectCertainty,
}

/// Durable action phase observed in the retained event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectCertainty {
    /// The invocation has a durable scheduling event.
    Scheduled,
    /// The invocation has a durable completion record.
    Confirmed,
    /// The invocation has a durable action-failure record.
    Failed,
    /// The invocation was cancelled before its action boundary completed.
    Abandoned,
}

/// Incident analysis result from scanning journal events.
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: String,
    pub failed_at_step: Option<u16>,
    pub side_effects: Vec<SideEffect>,
}

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[SideEffect],
    failed_at_step: Option<u16>,
) -> Vec<String> {
    match failure_code {
        "RunFailed" | "StepFailed" => failed_run_hints(side_effects, failed_at_step),
        "RunCancelled" => cancelled_run_hints(side_effects),
        _ => Vec::new(),
    }
}

fn failed_run_hints(side_effects: &[SideEffect], failed_at_step: Option<u16>) -> Vec<String> {
    let mut hints = vec!["investigate step output and engine logs for the failed step".to_string()];
    if !side_effects.is_empty() {
        hints.push(
            "review recorded action outcomes before choosing compensating actions".to_string(),
        );
    }
    if let Some(step) = failed_at_step {
        hints.push(format!(
            "inspect the retry contract before retrying from step {step}"
        ));
    }
    hints
}

fn cancelled_run_hints(side_effects: &[SideEffect]) -> Vec<String> {
    let mut hints = vec!["run was cancelled; check if cancellation was intentional".to_string()];
    if !side_effects.is_empty() {
        hints.push("review recorded action outcomes for partial cleanup needs".to_string());
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
            | JournalEvent::GateOpenedBoundEvent { .. }
            | JournalEvent::GateDecisionBoundEvent { .. }
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
        JournalEvent::GateOpenedBoundEvent { .. } => LifecycleState::Active,
        JournalEvent::GateDecisionBoundEvent { .. } => LifecycleState::Active,
        JournalEvent::CompatibilityDecisionEvent { .. } => LifecycleState::Active,
        JournalEvent::TenantAdmissionEvent { .. } => LifecycleState::Active,
        JournalEvent::RunBudgetOriginEvent { .. } => LifecycleState::Active,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod ticket_lifecycle_tests;
