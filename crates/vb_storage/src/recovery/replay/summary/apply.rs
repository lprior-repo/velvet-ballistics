#![forbid(unsafe_code)]
//! Event-to-summary application for journal recovery.
//!
//! `apply_summary_event` performs a single unconditional event-to-summary
//! transition. `apply_summary_event_checked` is the idempotency-aware variant
//! used by `summarize_recovery_events` to honour `ActionReplayTracker` state.
//! The envelope action completion recorders live as an `impl` block on
//! `FrameSeedAccumulator` to keep envelope digest verification co-located
//! with the rest of the action-replay path.

use crate::recovery::hydrate_support::verify_action_ticket_event;
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveredStepState, RecoveryError, RecoveryHydration,
    RecoveryResult, RecoveryRuntimeSummary, RecoveryTerminalState,
};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint};

use super::accumulator::FrameSeedAccumulator;
use super::hydrate::max_slot;

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
        JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionFailedEvent { .. }
        | JournalEvent::ActionAbandoned { .. } => {
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
        // WaitResolvedEvent is a resumption, not a fresh suspension: the run
        // unblocks because the wait condition was satisfied. It must not
        // inflate the suspension counter (regression check for RE-009).
        JournalEvent::WaitResolvedEvent { .. } => {}
        JournalEvent::AskTimedOutEvent { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunKilled { .. } => {
            summary.terminal = Some(RecoveryTerminalState::Killed);
        }
        JournalEvent::RunFinished { result, .. } => {
            summary.terminal = Some(RecoveryTerminalState::Finished { result: *result });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(RecoveryTerminalState::Failed);
        }
        // Lifecycle events (RunResumed, RunRetried, RunAnswered) do not carry sequence
        // numbers and are not part of the durable event log ordering for recovery summary.
        JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. }
        | JournalEvent::RunAnswered { .. } => {}
    }
}

/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration> {
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

    Ok(RecoveryHydration::Summary(summary))
}

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
            let verified_digest =
                crate::recovery::hydrate_support::verified_action_envelope_digest(
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

impl FrameSeedAccumulator {
    /// Records a completed action envelope onto the accumulator, applying
    /// the action-replay effect (skipping `Duplicate` envelopes) and
    /// promoting the underlying step to `Succeeded`.
    pub(super) fn record_action_completion_envelope(
        mut self,
        run: RunId,
        ticket: vb_core::ActionTicket,
        output: SlotIdx,
        outcome: crate::DurableActionOutcome,
        value: &[u8],
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
    ) -> RecoveryResult<Self> {
        let verified_digest = verify_action_envelope_digest_for_apply(
            run,
            ticket,
            outcome,
            value,
            encoded_len,
            value_digest,
        )?;
        self.action_tracker
            .require_scheduled_ticket(ticket, output)?;
        let effect = self.action_tracker.mark_completed_envelope_effect(
            ticket,
            output,
            encoded_len,
            taint,
            verified_digest,
        )?;
        if effect == ActionReplayEffect::Duplicate {
            return Ok(self);
        }
        self.summary.actions_resolved = self.summary.actions_resolved.saturating_add(1);
        self.summary.steps_succeeded = self.summary.steps_succeeded.saturating_add(1);
        self.summary.slots_written = self.summary.slots_written.saturating_add(1);
        self.pending_actions.remove(&(ticket.action, ticket.step));
        self.record_step(ticket.step, RecoveredStepState::Succeeded)
            .record_last_succeeded(ticket.step)
            .record_envelope_slot(output, value, taint)
    }

    fn record_envelope_slot(
        mut self,
        slot: SlotIdx,
        value: &[u8],
        taint: Taint,
    ) -> RecoveryResult<Self> {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        match postcard::from_bytes::<SlotValue>(value) {
            Ok(slot_value) => {
                self.slot_values.insert(slot, slot_value);
                self.slot_taint.insert(slot, taint);
                Ok(self)
            }
            Err(_) => Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!("slot value decode failed for slot {:?}", slot),
            }),
        }
    }
}

fn verify_action_envelope_digest_for_apply(
    run: RunId,
    ticket: vb_core::ActionTicket,
    outcome: crate::DurableActionOutcome,
    value: &[u8],
    encoded_len: u32,
    value_digest: [u8; 32],
) -> RecoveryResult<[u8; 32]> {
    crate::recovery::hydrate_support::verified_action_envelope_digest(
        run,
        ticket,
        outcome,
        value,
        encoded_len,
        value_digest,
    )
}
