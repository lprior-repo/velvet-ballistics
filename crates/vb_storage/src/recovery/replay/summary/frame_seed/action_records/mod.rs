#![forbid(unsafe_code)]
//! Action-related FrameSeedAccumulator record methods.
//!
//! Provides action scheduling, completion, and failure recorders.

use super::accumulator::{ActionEnvelopeView, FrameSeedAccumulator};
use crate::recovery::action_digest::verified_action_envelope_digest;
use crate::recovery::types::ActionReplayEffect;
use crate::recovery::{ActionReplayTracker, RecoveryError, RecoveryResult};
use vb_core::{ActionId, SlotIdx, SlotValue, StepIdx, Taint};

impl FrameSeedAccumulator {
    pub(super) fn record_action_completion_envelope(
        mut self,
        envelope: ActionEnvelopeView<'_>,
    ) -> RecoveryResult<Self> {
        self.envelope_event_seen = true;
        let verified_digest = verified_action_envelope_digest(
            envelope.run,
            envelope.ticket,
            envelope.outcome,
            envelope.value,
            envelope.encoded_len,
            envelope.value_digest,
        )?;
        self.action_tracker
            .require_scheduled_ticket(envelope.ticket, envelope.output)?;
        let effect = self.action_tracker.mark_completed_envelope_effect(
            envelope.ticket,
            envelope.output,
            envelope.encoded_len,
            envelope.taint,
            verified_digest,
        )?;
        if effect == ActionReplayEffect::Duplicate {
            return Ok(self);
        }
        self.summary.actions_resolved = self.summary.actions_resolved.saturating_add(1);
        self.summary.steps_succeeded = self.summary.steps_succeeded.saturating_add(1);
        self.summary.slots_written = self.summary.slots_written.saturating_add(1);
        self.pending_actions
            .remove(&(envelope.ticket.action, envelope.ticket.step));
        self.record_step(
            envelope.ticket.step,
            crate::recovery::RecoveredStepState::Succeeded,
        )
        .record_last_succeeded(envelope.ticket.step)
        .record_envelope_slot(envelope.output, envelope.value, envelope.taint)
    }

    pub(super) fn record_envelope_slot(
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

    pub(super) fn record_action_scheduled(
        mut self,
        action: ActionId,
        step: StepIdx,
    ) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.pending_actions.insert((action, step));
        Ok(self)
    }

    pub(super) fn record_action_scheduled_ticket(
        mut self,
        ticket: vb_core::ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    ) -> RecoveryResult<Self> {
        self.envelope_event_seen = true;
        let effect = self
            .action_tracker
            .mark_scheduled_ticket_effect(ticket, input, output)?;
        if effect == ActionReplayEffect::Apply {
            self.summary.actions_scheduled = self.summary.actions_scheduled.saturating_add(1);
            self.pending_actions.insert((ticket.action, ticket.step));
        }
        Ok(self)
    }

    pub(super) fn record_action_completed(
        mut self,
        action: ActionId,
        step: StepIdx,
    ) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_completed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }

    pub(super) fn record_action_failed(
        mut self,
        action: ActionId,
        step: StepIdx,
    ) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_failed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }
}

fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    current.map_or(Some(candidate), |slot| Some(slot.max(candidate)))
}
