#![forbid(unsafe_code)]
//! Action-completion envelope summary application.

use vb_core::WorkflowDigest;
use crate::recovery::types::{
    ActionReplayEffect, RecoveredStepState, RecoveryError, RecoveryResult,
};
use vb_core::{ActionTicket, RunId, SlotIdx, SlotValue, StepIdx, Taint};

use super::super::accumulator::FrameSeedAccumulator;
use super::super::hydrate::max_slot;

#[derive(Clone, Copy)]
pub(crate) struct ActionCompletionEnvelopeApply<'a> {
    pub(crate) run: RunId,
    pub(crate) ticket: ActionTicket,
    pub(crate) output: SlotIdx,
    pub(crate) outcome: crate::DurableActionOutcome,
    pub(crate) value: &'a [u8],
    pub(crate) encoded_len: u32,
    pub(crate) taint: Taint,
    pub(crate) value_digest: [u8; 32],
    pub(crate) action_abi_digest: WorkflowDigest,
}

impl FrameSeedAccumulator {
    /// Records a completed action envelope onto the accumulator, applying
    /// the action-replay effect and promoting the underlying step.
    pub(crate) fn record_action_completion_envelope(
        mut self,
        envelope: ActionCompletionEnvelopeApply<'_>,
    ) -> RecoveryResult<Self> {
        let verified_digest = verify_action_envelope_digest_for_apply(
            envelope.run,
            envelope.ticket,
            envelope.outcome,
            envelope.value,
            envelope.encoded_len,
            envelope.value_digest,
        )?;
        self.action_tracker
            .require_scheduled_ticket(envelope.ticket, envelope.output, envelope.action_abi_digest)?;
        let effect = self.action_tracker.mark_completed_envelope_effect(
            envelope.ticket,
            envelope.output,
            envelope.encoded_len,
            envelope.taint,
            verified_digest,
            envelope.action_abi_digest,
        )?;
        if effect == ActionReplayEffect::Duplicate {
            return Ok(self);
        }
        self.summary.actions_resolved = self.summary.actions_resolved.saturating_add(1);
        self.summary.steps_succeeded = self.summary.steps_succeeded.saturating_add(1);
        self.summary.slots_written = self.summary.slots_written.saturating_add(1);
        self.pending_actions
            .remove(&(envelope.ticket.action, envelope.ticket.step));
        self.record_step(envelope.ticket.step, RecoveredStepState::Succeeded)
            .record_last_succeeded(envelope.ticket.step)
            .record_envelope_slot(envelope.output, envelope.value, envelope.taint)
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
