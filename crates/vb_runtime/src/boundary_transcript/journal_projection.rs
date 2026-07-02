#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Cold-path projection of runtime journal events into boundary events.
//!
//! This projection captures everything the runtime journal *itself* knows
//! about a boundary event. For events whose authority the journal cannot
//! preserve (timer generation/deadline, ask answer payload, legacy
//! completion output), the runtime must additionally push directly via
//! the [`record`](super::record) methods.
//!
//! See the [`BoundaryEvent`] variants [`BoundaryEvent::TimerCaptured`],
//! [`BoundaryEvent::TimerFired`], and [`BoundaryEvent::AskAnswered`] for
//! the events that require direct capture.

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::boundary_transcript::event::BoundaryEvent;
use crate::boundary_transcript::shared_transcript::SharedBoundaryTranscript;
use crate::boundary_transcript::transcript::BoundaryTranscriptError;
use crate::journal::RuntimeJournalEvent;

/// Cold-path projection of runtime journal events into boundary events.
#[derive(Debug, Clone)]
pub struct BoundaryTranscriptJournal {
    pub(crate) transcript: SharedBoundaryTranscript,
}

impl BoundaryTranscriptJournal {
    /// Creates a journal projection that writes into the supplied transcript.
    #[must_use]
    pub fn new(transcript: SharedBoundaryTranscript) -> Self {
        Self { transcript }
    }

    /// Returns the underlying shared transcript.
    #[must_use]
    pub fn transcript(&self) -> &SharedBoundaryTranscript {
        &self.transcript
    }

    /// Projects a single runtime journal event into a boundary event.
    ///
    /// Returns `Some(event)` for events that carry enough information in
    /// the journal to reconstruct the boundary transcript entry. Returns
    /// `None` for events whose authority is not preserved in the journal
    /// — those events must be captured via direct push through
    /// [`record`](super::record).
    #[must_use]
    pub fn project(&self, event: &RuntimeJournalEvent) -> Option<BoundaryEvent> {
        match event {
            RuntimeJournalEvent::ActionScheduledTicket { ticket, .. } => {
                Self::project_action_scheduled_envelope(ticket)
            }
            RuntimeJournalEvent::ActionScheduled { run, step, action } => {
                Self::project_action_scheduled_legacy(*run, *step, *action)
            }
            RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                encoded_len,
                taint,
                value_digest,
                ..
            } => Self::project_action_completed_envelope(
                ticket,
                *output,
                *encoded_len,
                *taint,
                *value_digest,
            ),
            RuntimeJournalEvent::ActionCompleted { run, step, action } => {
                Self::project_action_completed_legacy(*run, *step, *action)
            }
            RuntimeJournalEvent::ActionFailed {
                run,
                step,
                action,
                attempt,
            } => Self::project_action_failed(*run, *step, *action, *attempt),
            RuntimeJournalEvent::ActionAbandoned { ticket } => {
                Self::project_action_abandoned(ticket)
            }
            RuntimeJournalEvent::WaitScheduled { run, step } => {
                Self::project_wait_scheduled(*run, *step)
            }
            RuntimeJournalEvent::WaitResolved { run, step } => {
                Self::project_wait_resolved(*run, *step)
            }
            RuntimeJournalEvent::AskScheduled { run, step } => {
                Self::project_ask_scheduled(*run, *step)
            }
            RuntimeJournalEvent::AskAnswered { run, step, slot } => {
                Self::project_ask_answered(*run, *step, *slot)
            }
            RuntimeJournalEvent::AskTimedOut { run, step } => {
                Self::project_ask_timed_out(*run, *step)
            }
            RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    /// Convenience: project and push a journal event into the transcript.
    ///
    /// Returns `Ok(())` when no event was projectable (lifecycle events
    /// that the boundary transcript does not cover) or when the push
    /// succeeded. Returns `Err` for mutex poisoning, allocation failure,
    /// or sequence saturation.
    pub fn record(&self, event: &RuntimeJournalEvent) -> Result<(), BoundaryTranscriptError> {
        if let Some(boundary) = self.project(event) {
            self.transcript.push(boundary)?;
        }
        Ok(())
    }

    /// Projects `ActionScheduledTicket` (modern envelope) with full ticket authority.
    fn project_action_scheduled_envelope(
        ticket: &vb_core::action::ActionTicket,
    ) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionScheduled {
            run: ticket.run,
            ticket: *ticket,
        })
    }

    /// Projects `ActionScheduled` (legacy path) — no ticket authority.
    fn project_action_scheduled_legacy(
        run: RunId,
        step: StepIdx,
        action: vb_core::ids::ActionId,
    ) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionScheduledLegacy { run, step, action })
    }

    /// Projects `ActionCompletedEnvelope` (modern envelope) with full payload.
    #[allow(clippy::too_many_arguments)]
    fn project_action_completed_envelope(
        ticket: &vb_core::action::ActionTicket,
        output_slot: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
    ) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionCompletedModern {
            run: ticket.run,
            ticket: *ticket,
            output_slot,
            encoded_len,
            taint,
            value_digest,
        })
    }

    /// Projects `ActionCompleted` (legacy path) — no output payload.
    fn project_action_completed_legacy(
        run: RunId,
        step: StepIdx,
        action: vb_core::ids::ActionId,
    ) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionCompletedLegacy { run, step, action })
    }

    /// Projects `ActionFailed`. The journal does not carry failure code /
    /// retry policy / taint — those are filled by the direct `record_action_failed`
    /// call site. The journal-projected version uses the historical
    /// placeholders (`0` / `0` / `Clean`) so the parity test stays
    /// deterministic.
    fn project_action_failed(
        run: RunId,
        step: StepIdx,
        action: vb_core::ids::ActionId,
        attempt: u16,
    ) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
            failure_code: 0,
            retry_policy_tag: 0,
            taint: Taint::Clean,
        })
    }

    /// Projects `ActionAbandoned` with the abandoned ticket.
    fn project_action_abandoned(ticket: &vb_core::action::ActionTicket) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::ActionAbandoned {
            run: ticket.run,
            ticket: *ticket,
        })
    }

    /// Projects `WaitScheduled` as the recoverable surrogate for
    /// `TimerCaptured { kind = Wait }` (the journal has no timer
    /// authority to project).
    fn project_wait_scheduled(run: RunId, step: StepIdx) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::WaitScheduled { run, step })
    }

    /// Projects `WaitResolved` as the recoverable surrogate for
    /// `TimerFired { kind = Wait }` (the journal has no timer authority
    /// to project).
    fn project_wait_resolved(run: RunId, step: StepIdx) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::WaitResolved { run, step })
    }

    /// Projects `AskScheduled` as the recoverable surrogate for
    /// `TimerCaptured { kind = Ask }` (the journal has no timer
    /// authority to project).
    fn project_ask_scheduled(run: RunId, step: StepIdx) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::AskScheduled { run, step })
    }

    /// Projects `AskAnswered` with placeholders for the journal's missing
    /// authority fields (`taint = Clean`, `encoded_len = 0`, and
    /// `resume_step = ask_step`). The full payload is pushed by the
    /// direct `record_ask_answered` call site.
    fn project_ask_answered(run: RunId, step: StepIdx, slot: SlotIdx) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::AskAnswered {
            run,
            ask_step: step,
            resume_step: step,
            slot,
            taint: Taint::Clean,
            encoded_len: 0,
        })
    }

    /// Projects `AskTimedOut` as the recoverable surrogate for
    /// `TimerFired { kind = Ask }` (the journal has no timer authority
    /// to project).
    fn project_ask_timed_out(run: RunId, step: StepIdx) -> Option<BoundaryEvent> {
        Some(BoundaryEvent::AskTimedOut { run, step })
    }
}
