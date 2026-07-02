#![forbid(unsafe_code)]
//! `FrameSeedAccumulator` state machine for journal recovery.
//!
//! Walks ordered journal events to produce the data needed to rebuild a
//! `RecoveryFrameSeed`: step states, slot values/taints, pending actions,
//! the program counter, and the action replay tracker.
//!
//! Fields are `pub(super)` so the sibling `apply` and `hydrate` modules can
//! host extra `impl` blocks (e.g. envelope action completion, slot write
//! recording) without breaking encapsulation at the crate boundary.

use std::collections::{HashMap, HashSet};

use vb_core::{ActionId, ActionTicket, RunId, SlotIdx, SlotValue, StepIdx, Taint};

use crate::recovery::hydrate_support::verify_action_ticket_event;
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveredStepState, RecoveryError, RecoveryResult,
    RecoveryRuntimeSummary,
};
use crate::{EventSeq, JournalEvent};

use super::apply::{ActionCompletionEnvelopeApply, apply_summary_event};
use super::hydrate::{max_slot, max_step, min_step, record_slot_write};

/// Walks journal events, accumulating per-run state needed to rebuild a
/// `RecoveryFrameSeed`.
#[derive(Debug)]
pub(super) struct FrameSeedAccumulator {
    pub(super) run: RunId,
    pub(super) summary: RecoveryRuntimeSummary,
    pub(super) step_states: HashMap<StepIdx, RecoveredStepState>,
    pub(super) slot_values: HashMap<SlotIdx, SlotValue>,
    pub(super) slot_taint: HashMap<SlotIdx, Taint>,
    pub(super) pending_actions: HashSet<(ActionId, StepIdx)>,
    pub(super) action_tracker: ActionReplayTracker,
    pub(super) max_step_idx: Option<StepIdx>,
    pub(super) min_step_idx: Option<StepIdx>,
    pub(super) max_slot_idx: Option<SlotIdx>,
    pub(super) pc: StepIdx,
    pub(super) last_succeeded_step: Option<StepIdx>,
    pub(super) missing_slot_values: bool,
    pub(super) event_slot_taint_unsupported: bool,
}

impl FrameSeedAccumulator {
    /// Constructs a fresh accumulator pinned to the run/sequence observed
    /// at the start of the recovery event slice.
    pub(super) fn new(run: RunId, first_seq: crate::EventSeq) -> Self {
        Self {
            run,
            summary: RecoveryRuntimeSummary {
                run,
                first_seq,
                last_seq: first_seq,
                workflow: None,
                steps_started: 0,
                steps_succeeded: 0,
                actions_scheduled: 0,
                actions_resolved: 0,
                suspensions: 0,
                slots_written: 0,
                terminal: None,
            },
            step_states: HashMap::new(),
            slot_values: HashMap::new(),
            slot_taint: HashMap::new(),
            pending_actions: HashSet::new(),
            action_tracker: ActionReplayTracker::new(),
            max_step_idx: None,
            min_step_idx: None,
            max_slot_idx: None,
            pc: StepIdx::ZERO,
            last_succeeded_step: None,
            missing_slot_values: false,
            event_slot_taint_unsupported: false,
        }
    }

    /// Applies a single event to the accumulator, returning the updated
    /// accumulator or a `RecoveryError` on divergence / overflow.
    pub(super) fn apply(mut self, event: &JournalEvent) -> RecoveryResult<Self> {
        if event.run_id() != self.run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "frame seed recovery received events for multiple runs".to_owned(),
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
        self.summary.last_seq = event.seq();
        if !matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
                | JournalEvent::ActionScheduledTicket { .. }
        ) {
            apply_summary_event(&mut self.summary, event);
        }
        self.apply_frame_event(event)
    }

    fn apply_frame_event(self, event: &JournalEvent) -> RecoveryResult<Self> {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Running))
            }
            JournalEvent::StepSucceeded { step, .. } => Ok(self
                .record_step(*step, RecoveredStepState::Succeeded)
                .record_last_succeeded(*step)),
            JournalEvent::ActionScheduled { action, step, .. } => {
                self.record_action_scheduled(*action, *step)
            }
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } => {
                verify_action_ticket_event(*run, *ticket)?;
                self.record_action_scheduled_ticket(*ticket, *input, *output, *action_abi_digest)
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                self.record_action_completed(*action, *step)
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                self.record_action_failed(*action, *step)
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
                action_abi_digest,
                ..
            } => self.record_action_completion_envelope(ActionCompletionEnvelopeApply {
                run: *run,
                ticket: *ticket,
                output: *output,
                outcome: *outcome,
                value,
                encoded_len: *encoded_len,
                taint: *taint,
                value_digest: *value_digest,
                action_abi_digest: *action_abi_digest,
            }),
            JournalEvent::WaitScheduledEvent { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Waiting))
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Asking))
            }
            JournalEvent::AskTimedOutEvent { step, .. } => Ok(self
                .record_step(*step, RecoveredStepState::Succeeded)
                .record_last_succeeded(*step)),
            JournalEvent::SlotWrittenEvent {
                slot, value, extra, ..
            } => record_slot_write(self, *slot, value, extra),
            JournalEvent::RunFinished { result, .. } => Ok(self.record_slot(*result)),
            JournalEvent::ActionAbandoned { ticket, .. } => self.record_action_abandoned(*ticket),
            JournalEvent::RunCancelled { .. } => Ok(self),
            _ => Ok(self),
        }
    }

    pub(super) fn record_step(mut self, step: StepIdx, state: RecoveredStepState) -> Self {
        self.max_step_idx = max_step(self.max_step_idx, step);
        self.min_step_idx = min_step(self.min_step_idx, step);
        self.pc = max_step(Some(self.pc), step).map_or(self.pc, |value| value);
        self.step_states.insert(step, state);
        self
    }

    pub(super) fn record_last_succeeded(mut self, step: StepIdx) -> Self {
        self.last_succeeded_step = Some(step);
        self
    }

    fn record_action_scheduled(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.pending_actions.insert((action, step));
        Ok(self)
    }

    fn record_action_scheduled_ticket(
        mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: vb_core::WorkflowDigest,
    ) -> RecoveryResult<Self> {
        let effect = self.action_tracker.mark_scheduled_ticket_effect(
            ticket,
            input,
            output,
            action_abi_digest,
        )?;
        if effect == ActionReplayEffect::Apply {
            self.summary.actions_scheduled = self.summary.actions_scheduled.saturating_add(1);
            self.pending_actions.insert((ticket.action, ticket.step));
            // Master §45.18 Do-node: extend slot dimension to action
            // output (and input) at schedule time so a crash before
            // the completion envelope doesn't truncate the slot
            // array. See sweep `vb-cc2my` / `vb-1rqz7.7`.
            self.max_slot_idx = max_slot(self.max_slot_idx, output);
            self.max_slot_idx = max_slot(self.max_slot_idx, input);
        }
        let mut frame = self;
        frame.max_step_idx = max_step(frame.max_step_idx, ticket.step);
        frame.min_step_idx = min_step(frame.min_step_idx, ticket.step);
        frame.pc = max_step(Some(frame.pc), ticket.step).map_or(frame.pc, |value| value);
        frame
            .step_states
            .insert(ticket.step, RecoveredStepState::Running);
        Ok(frame)
    }

    fn record_action_abandoned(mut self, ticket: ActionTicket) -> RecoveryResult<Self> {
        if !self.action_tracker.is_resolved(ticket.action, ticket.step) {
            self.action_tracker.mark_failed(ticket.action, ticket.step);
        }
        self.pending_actions.remove(&(ticket.action, ticket.step));
        Ok(self)
    }

    fn record_action_completed(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_completed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }

    fn record_action_failed(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_failed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }

    /// First step observed for this run, falling back to `StepIdx::ZERO`
    /// when no step events have been seen.
    pub(super) fn first_step(&self) -> StepIdx {
        self.min_step_idx.map_or(StepIdx::ZERO, |step| step)
    }
}
