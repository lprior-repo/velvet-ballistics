#![forbid(unsafe_code)]
//! Runtime summary construction from journal events.
//!
//! Provides:
//! - `apply_summary_event` / `apply_summary_event_checked` — event → summary
//! - `summarize_recovery_events` — full summary hydration
//! - `recover_run_admission_from_events` — latest admission metadata

use crate::recovery::action_digest::{verified_action_envelope_digest, verify_action_ticket_event};
use crate::recovery::types::ActionReplayEffect;
use crate::recovery::{ActionReplayTracker, RecoveryError, RecoveryResult, RecoveryRuntimeSummary};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

// ── Event → summary application ──────────────────────────────────────────────

/// Applies an event's effects to a runtime summary.
pub fn apply_summary_event(summary: &mut RecoveryRuntimeSummary, event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            summary.workflow = Some(*workflow);
        }
        JournalEvent::RunAdmission { .. } => {}
        JournalEvent::StepStarted { .. } => {
            summary.steps_started = summary.steps_started.saturating_add(1);
        }
        JournalEvent::StepSucceeded { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::ActionScheduled { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionScheduledTicket { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
        }
        JournalEvent::ActionCompletedEnvelope { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
            summary.slots_written = summary.slots_written.saturating_add(1);
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            summary.slots_written = summary.slots_written.saturating_add(1);
        }
        JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. } => {
            summary.suspensions = summary.suspensions.saturating_add(1);
        }
        JournalEvent::AskAnsweredEvent { .. } => {}
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(crate::recovery::RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunKilled { .. } => {
            summary.terminal = Some(crate::recovery::RecoveryTerminalState::Killed);
        }
        JournalEvent::RunFinished { result, .. } => {
            summary.terminal =
                Some(crate::recovery::RecoveryTerminalState::Finished { result: *result });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(crate::recovery::RecoveryTerminalState::Failed);
        }
        // Lifecycle events (RunResumed, RunRetried, RunAnswered) do not carry sequence
        // numbers and are not part of the durable event log ordering for recovery summary.
        JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. }
        | JournalEvent::RunAnswered { .. } => {}
    }
}

/// Rejected a non-idempotent action that has already been resolved during replay.
fn reject_resolved_summary_action(
    tracker: &ActionReplayTracker,
    action: ActionId,
    step: StepIdx,
) -> RecoveryResult<()> {
    if tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    Ok(())
}

// ── Checked summary event application ────────────────────────────────────────

fn apply_summary_event_checked(
    summary: &mut RecoveryRuntimeSummary,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<()> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionScheduledTicket {
            run,
            ticket,
            input,
            output,
            ..
        } => {
            verify_action_ticket_event(*run, *ticket)?;
            let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
            if effect == ActionReplayEffect::Apply {
                apply_summary_event(summary, event);
            }
            Ok(())
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionCompletedEnvelope {
            run,
            ticket,
            output,
            outcome,
            value,
            encoded_len,
            taint,
            value_digest,
            ..
        } => {
            let verified_digest = verified_action_envelope_digest(
                *run,
                *ticket,
                *outcome,
                value,
                *encoded_len,
                *value_digest,
            )?;
            tracker.require_scheduled_ticket(*ticket, *output)?;
            let effect = tracker.mark_completed_envelope_effect(
                *ticket,
                *output,
                *encoded_len,
                *taint,
                verified_digest,
            )?;
            if effect == ActionReplayEffect::Apply {
                apply_summary_event(summary, event);
            }
            Ok(())
        }
        _ => {
            apply_summary_event(summary, event);
            Ok(())
        }
    }
}

// ── Admission metadata recovery ──────────────────────────────────────────────

/// Recovers the latest admission metadata from ordered journal events.
#[must_use]
pub fn recover_run_admission_from_events(
    events: &[JournalEvent],
) -> Option<crate::recovery::RecoveredRunAdmission> {
    events.iter().rev().find_map(|event| match event {
        JournalEvent::RunAdmission {
            run,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => Some(crate::recovery::RecoveredRunAdmission {
            artifact_digest: *artifact_digest,
            run_id: *run,
            granted_capabilities: granted_capabilities.clone(),
            policy: *policy,
        }),
        _ => None,
    })
}

// ── Full summary hydration ──────────────────────────────────────────────────

/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(
    events: &[JournalEvent],
) -> RecoveryResult<crate::recovery::RecoveryHydration> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
    };
    let run = first.run_id();
    let mut summary = RecoveryRuntimeSummary {
        run,
        first_seq: first.seq(),
        last_seq: first.seq(),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let mut tracker = ActionReplayTracker::new();

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery summary received events for multiple runs".to_owned(),
            });
        }
        if event.seq() == EventSeq::MAX {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "overflow sentinel sequence {} is not valid",
                    event.seq().get()
                ),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event_checked(&mut summary, event, &mut tracker)?;
    }

    Ok(crate::recovery::RecoveryHydration::Summary(summary))
}
