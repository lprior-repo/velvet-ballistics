#![forbid(unsafe_code)]
//! Durable action completion replay validation.

use super::driver::Driver;
use super::invalid;
use crate::DurableActionOutcome;
use crate::recovery::RecoveryResult;
use crate::recovery::hydrate_support::verified_action_envelope_digest;
use vb_core::action::ActionTicket;
use vb_core::{SlotIdx, SlotValue, WorkflowDigest};

pub(super) struct ActionCompletion<'a> {
    pub(super) ticket: ActionTicket,
    pub(super) outcome: DurableActionOutcome,
    pub(super) output: SlotIdx,
    pub(super) value: &'a [u8],
    pub(super) taint: vb_core::Taint,
    pub(super) value_digest: &'a [u8; 32],
    pub(super) encoded_len: u32,
    pub(super) abi_digest: WorkflowDigest,
}

impl Driver<'_> {
    pub(super) fn completed_action(
        &mut self,
        completion: ActionCompletion<'_>,
    ) -> RecoveryResult<()> {
        if completion.outcome != DurableActionOutcome::Ready {
            return Err(invalid(
                completion.ticket.step,
                "action completion outcome mismatch",
            ));
        }
        verified_action_envelope_digest(
            self.frame.run_id(),
            completion.ticket,
            completion.outcome,
            completion.value,
            completion.encoded_len,
            *completion.value_digest,
        )?;
        let scheduled = self
            .action
            .as_ref()
            .ok_or_else(|| invalid(completion.ticket.step, "action completion without schedule"))?;
        if scheduled.ticket != completion.ticket
            || scheduled.output != completion.output
            || scheduled.abi_digest != completion.abi_digest
        {
            return Err(invalid(
                completion.ticket.step,
                "action completion evidence mismatch",
            ));
        }
        let decoded: SlotValue = postcard::from_bytes(completion.value)
            .map_err(|_| invalid(completion.ticket.step, "invalid action output payload"))?;
        let next = self
            .plan
            .node(completion.ticket.step)
            .and_then(|node| node.routing.next.step())
            .ok_or_else(|| {
                invalid(
                    completion.ticket.step,
                    "action completion missing next step",
                )
            })?;
        self.frame
            .write_slot_with_taint(completion.output, decoded, completion.taint)
            .map_err(|e| invalid(completion.ticket.step, e.to_string()))?;
        self.frame
            .mark_succeeded(completion.ticket.step)
            .map_err(|e| invalid(completion.ticket.step, e.to_string()))?;
        self.frame
            .set_pc(next)
            .map_err(|e| invalid(completion.ticket.step, e.to_string()))?;
        self.frame
            .increment_executed()
            .map_err(|e| invalid(completion.ticket.step, e.to_string()))?;
        self.action = None;
        self.active = None;
        Ok(())
    }
}
