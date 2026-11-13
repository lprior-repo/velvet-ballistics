#![forbid(unsafe_code)]
//! Action and wait replay for owned deterministic traces.

use super::driver::Driver;
use super::invalid;
use crate::recovery::RecoveryResult;
use vb_core::action::{ActionTicket, action_ticket_has_valid_key};
use vb_core::replay::{ReplayAction, SuspensionKind};
use vb_core::{CompiledNodeKind as K, SlotIdx, SlotValue, StepIdx, Taint};

/// One scheduled action boundary awaiting its durable completion envelope.
#[derive(Debug)]
pub(super) struct ScheduledAction {
    pub(super) ticket: ActionTicket,
    pub(super) output: SlotIdx,
}

impl Driver<'_> {
    pub(super) fn pending_boundary(&self) -> bool {
        match self.active.as_ref() {
            Some((_, ReplayAction::Suspended { kind, .. })) => match kind {
                SuspensionKind::ActionPending => self.action.is_some(),
                SuspensionKind::WaitUntil | SuspensionKind::WaitEvent => self.waiting.is_some(),
                SuspensionKind::AskPending => {
                    self.ask.is_some() && self.answer.is_none() && !self.answered
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub(super) fn scheduled_action(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    ) -> RecoveryResult<()> {
        let (active, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(ticket.step, "action schedule without active step"))?;
        if *active != ticket.step
            || !matches!(
                action,
                ReplayAction::Suspended {
                    kind: SuspensionKind::ActionPending,
                    ..
                }
            )
            || self.action.is_some()
        {
            return Err(invalid(ticket.step, "action schedule authority mismatch"));
        }
        let node = self
            .plan
            .node(ticket.step)
            .ok_or_else(|| invalid(ticket.step, "Do node missing"))?;
        let K::Do {
            action: expected,
            input: expected_input,
            ..
        } = &node.kind
        else {
            return Err(invalid(ticket.step, "action schedule on non-Do node"));
        };
        if ticket.action != *expected
            || input != *expected_input
            || node.output.slot() != Some(output)
            || ticket.attempt == 0
            || !action_ticket_has_valid_key(ticket)
        {
            return Err(invalid(
                ticket.step,
                "action ticket differs from compiled Do",
            ));
        }
        self.action = Some(ScheduledAction { ticket, output });
        Ok(())
    }

    pub(super) fn completed_action(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        value: &[u8],
        taint: Taint,
        value_digest: &[u8; 32],
        encoded_len: u32,
    ) -> RecoveryResult<()> {
        let scheduled = self
            .action
            .as_ref()
            .ok_or_else(|| invalid(ticket.step, "action completion without schedule"))?;
        let expected_len = u32::try_from(value.len())
            .map_err(|_| invalid(ticket.step, "action payload length overflow"))?;
        if scheduled.ticket != ticket
            || scheduled.output != output
            || encoded_len != expected_len
            || blake3::hash(value).as_bytes() != value_digest
        {
            return Err(invalid(ticket.step, "action completion evidence mismatch"));
        }
        let decoded: SlotValue = postcard::from_bytes(value)
            .map_err(|_| invalid(ticket.step, "invalid action output payload"))?;
        let next = self
            .plan
            .node(ticket.step)
            .and_then(|node| node.routing.next.step())
            .ok_or_else(|| invalid(ticket.step, "action completion missing next step"))?;
        self.frame
            .write_slot_with_taint(output, decoded, taint)
            .map_err(|e| invalid(ticket.step, e.to_string()))?;
        self.frame
            .mark_succeeded(ticket.step)
            .map_err(|e| invalid(ticket.step, e.to_string()))?;
        self.frame
            .set_pc(next)
            .map_err(|e| invalid(ticket.step, e.to_string()))?;
        self.frame
            .increment_executed()
            .map_err(|e| invalid(ticket.step, e.to_string()))?;
        self.action = None;
        self.active = None;
        Ok(())
    }

    pub(super) fn wait_scheduled(&mut self, step: StepIdx, attempt: u16) -> RecoveryResult<()> {
        let (active, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(step, "wait schedule without active step"))?;
        if *active != step
            || !matches!(
                action,
                ReplayAction::Suspended {
                    kind: SuspensionKind::WaitUntil | SuspensionKind::WaitEvent,
                    ..
                }
            )
            || attempt == 0
            || self.waiting.is_some()
        {
            return Err(invalid(step, "wait schedule authority mismatch"));
        }
        self.frame
            .mark_waiting(step)
            .map_err(|e| invalid(step, e.to_string()))?;
        self.waiting = Some((step, attempt));
        Ok(())
    }

    pub(super) fn wait_resolved(&mut self, step: StepIdx, attempt: u16) -> RecoveryResult<()> {
        if self.waiting != Some((step, attempt)) {
            return Err(invalid(step, "wait resolution differs from schedule"));
        }
        let next = self
            .plan
            .node(step)
            .and_then(|node| node.routing.next.step())
            .ok_or_else(|| invalid(step, "wait resolution missing next step"))?;
        self.frame
            .mark_running(step)
            .and_then(|()| self.frame.mark_succeeded(step))
            .and_then(|()| self.frame.set_pc(next))
            .map_err(|e| invalid(step, e.to_string()))?;
        self.waiting = None;
        self.active = None;
        Ok(())
    }

    pub(super) fn scheduled_ask(&mut self, step: StepIdx) -> RecoveryResult<()> {
        if !matches!(self.active.as_ref(), Some((active, ReplayAction::Suspended { step: suspended, kind: SuspensionKind::AskPending })) if *active == step && *suspended == step)
            || self.ask.is_some()
        {
            return Err(invalid(step, "Ask schedule without suspension"));
        }
        let ask = self
            .plan
            .node(step)
            .ok_or_else(|| invalid(step, "Ask node missing"))?;
        let resume = ask
            .routing
            .next
            .step()
            .ok_or_else(|| invalid(step, "Ask resume missing"))?;
        let Some(K::AskResume { answer }) = self.plan.node(resume).map(|node| &node.kind) else {
            return Err(invalid(step, "Ask resume authority missing"));
        };
        self.frame
            .mark_asking(step)
            .map_err(|e| invalid(step, e.to_string()))?;
        self.ask = Some((step, resume, *answer));
        Ok(())
    }

    pub(super) fn answered_ask(&mut self, step: StepIdx) -> RecoveryResult<()> {
        let (ask, resume, slot) = self
            .ask
            .ok_or_else(|| invalid(step, "Ask answer without schedule"))?;
        let (value, taint) = self
            .answer
            .take()
            .ok_or_else(|| invalid(step, "Ask answer payload missing"))?;
        if ask != step || self.answered {
            return Err(invalid(step, "Ask answer authority mismatch"));
        }
        self.frame
            .write_slot_with_taint(slot, value, taint)
            .map_err(|e| invalid(step, e.to_string()))?;
        self.frame
            .mark_running(step)
            .and_then(|()| self.frame.mark_succeeded(step))
            .and_then(|()| self.frame.set_pc(resume))
            .map_err(|e| invalid(step, e.to_string()))?;
        self.answered = true;
        Ok(())
    }
}
