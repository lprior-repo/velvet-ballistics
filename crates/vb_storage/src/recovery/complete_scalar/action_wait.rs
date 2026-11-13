#![forbid(unsafe_code)]
//! Action and wait replay for owned deterministic traces.

use super::driver::Driver;
use super::invalid;
use crate::recovery::RecoveryResult;
use crate::recovery::hydrate_support::verify_action_ticket_event;
use vb_core::action::ActionTicket;
use vb_core::replay::{ReplayAction, SuspensionKind};
use vb_core::{CompiledNodeKind as K, Deadline, SlotIdx, StepIdx, StepState, WorkflowDigest};

/// One scheduled action boundary awaiting its durable completion envelope.
#[derive(Debug)]
pub(super) struct ScheduledAction {
    pub(super) ticket: ActionTicket,
    pub(super) output: SlotIdx,
    pub(super) abi_digest: WorkflowDigest,
}

impl Driver<'_> {
    pub(super) fn pending_boundary(&self) -> bool {
        match self.active.as_ref() {
            Some((step, ReplayAction::Suspended { kind, .. })) => match kind {
                SuspensionKind::ActionPending => self.action.is_some(),
                SuspensionKind::WaitUntil => self.waiting.is_some(),
                SuspensionKind::WaitEvent => {
                    self.waiting.is_some() || self.untimed_wait_is_waiting(*step)
                }
                SuspensionKind::AskPending => {
                    self.ask.is_some() && self.answer.is_none() && !self.answered
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn untimed_wait_is_waiting(&self, step: StepIdx) -> bool {
        matches!(
            self.plan.node(step).map(|node| &node.kind),
            Some(K::WaitEvent {
                timeout_slot: Deadline::Unset,
                ..
            })
        ) && self.frame.step_state(step).ok() == Some(StepState::Waiting)
    }

    pub(super) fn scheduled_action(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        abi_digest: WorkflowDigest,
    ) -> RecoveryResult<()> {
        verify_action_ticket_event(self.frame.run_id(), ticket)?;
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
        let expected_capacity = node.retry.map_or(1, |policy| policy.max_attempts);
        if ticket.action != *expected
            || input != *expected_input
            || node.output.slot() != Some(output)
            || ticket.attempt == 0
            || ticket.capacity != expected_capacity
        {
            return Err(invalid(
                ticket.step,
                "action ticket differs from compiled Do",
            ));
        }
        self.action = Some(ScheduledAction {
            ticket,
            output,
            abi_digest,
        });
        Ok(())
    }

    pub(super) fn action_failed(&mut self, step: StepIdx) -> RecoveryResult<()> {
        let (active, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(step, "action failure without active step"))?;
        if *active != step
            || !matches!(
                action,
                ReplayAction::Suspended {
                    kind: SuspensionKind::ActionPending,
                    ..
                }
            )
            || self.action.is_none()
        {
            return Err(invalid(step, "action failure authority mismatch"));
        }
        self.frame
            .mark_failed(step)
            .map_err(|e| invalid(step, e.to_string()))?;
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

    pub(super) fn timed_out_ask(&mut self, step: StepIdx) -> RecoveryResult<()> {
        let (ask, resume, _) = self
            .ask
            .ok_or_else(|| invalid(step, "Ask timeout without schedule"))?;
        if ask != step
            || self.answered
            || !matches!(
                self.active.as_ref(),
                Some((active, ReplayAction::Suspended {
                    kind: SuspensionKind::AskPending,
                    ..
                })) if *active == step
            )
        {
            return Err(invalid(step, "Ask timeout authority mismatch"));
        }
        self.frame
            .mark_running(step)
            .and_then(|()| self.frame.mark_succeeded(step))
            .and_then(|()| self.frame.set_pc(resume))
            .map_err(|e| invalid(step, e.to_string()))?;
        self.ask = None;
        self.active = None;
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
