use super::super::{driver::Driver, invalid};
use crate::{JournalEvent as E, recovery::RecoveryResult};
use vb_core::{ActionId, ActionTicket, StepIdx};

impl Driver<'_> {
    pub(super) fn require_failure_outcome(&self, tail: &[E]) -> RecoveryResult<()> {
        let ticket = self
            .action
            .as_ref()
            .ok_or_else(|| invalid(self.frame.pc(), "failed action missing"))?
            .ticket;
        if ticket.attempt <= 1 {
            return Ok(());
        }
        let next = tail
            .iter()
            .find(|event| !matches!(event, E::RunResumed { .. }));
        let resolved = match next {
            Some(E::RunFailedEvent { run, attempt, .. }) => {
                *run == ticket.run && *attempt == ticket.run_attempt
            }
            Some(E::StepStarted {
                run, step, attempt, ..
            }) => {
                *run == ticket.run
                    && *attempt == ticket.run_attempt
                    && self
                        .plan
                        .node(ticket.step)
                        .and_then(|node| node.routing.error.handler())
                        == Some(*step)
            }
            _ => false,
        };
        if resolved {
            Ok(())
        } else {
            Err(invalid(
                ticket.step,
                "action failure outcome evidence missing",
            ))
        }
    }

    pub(super) fn validate_failed_schedule(
        &self,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    ) -> RecoveryResult<()> {
        if self.action.as_ref().is_none_or(|issued| {
            issued.ticket.step != step
                || issued.ticket.action != action
                || issued.ticket.attempt != attempt
        }) {
            return Err(invalid(step, "action failure differs from issued ticket"));
        }
        Ok(())
    }

    pub(super) fn validate_retry_schedule(&mut self, ticket: ActionTicket) -> RecoveryResult<()> {
        let index = ticket.step.as_usize();
        let accepted = match self.retry_ticket.take() {
            Some(expected) => expected == ticket,
            None => self
                .issued_action_attempts
                .get(index)
                .is_some_and(|previous| ticket.attempt == (*previous).max(1)),
        };
        if !accepted {
            return Err(invalid(
                ticket.step,
                "action retry schedule lacks selected authority",
            ));
        }
        let issued = self
            .issued_action_attempts
            .get_mut(index)
            .ok_or_else(|| invalid(ticket.step, "issued action attempt index missing"))?;
        *issued = ticket.attempt;
        Ok(())
    }

    pub(super) fn selected_immediate_retry(
        &self,
        tail: &[E],
    ) -> RecoveryResult<Option<ActionTicket>> {
        let old = self
            .action
            .as_ref()
            .ok_or_else(|| invalid(self.frame.pc(), "failed action missing"))?;
        let Some(E::StepStarted {
            run,
            step,
            attempt,
            seq: started_seq,
        }) = tail.first()
        else {
            return Ok(None);
        };
        if *step != old.ticket.step {
            return Ok(None);
        }
        let Some(E::ActionScheduledTicket {
            run: scheduled_run,
            ticket,
            input,
            output,
            action_abi_digest,
            seq: scheduled_seq,
        }) = tail.get(1)
        else {
            return Err(invalid(*step, "immediate retry lacks full selected ticket"));
        };
        let policy = self
            .plan
            .node(*step)
            .and_then(|node| node.retry)
            .ok_or_else(|| invalid(*step, "immediate retry policy missing"))?;
        if policy.delay_ms != 0
            || *run != old.ticket.run
            || *scheduled_run != *run
            || *attempt != old.ticket.run_attempt
            || policy.max_attempts != old.ticket.capacity
            || old.ticket.attempt.checked_add(1) != Some(ticket.attempt)
            || ticket.attempt > policy.max_attempts
            || !same_retry_identity(old.ticket, *ticket)
            || old.input != *input
            || old.output != *output
            || old.abi_digest != *action_abi_digest
            || self.previous_seq.and_then(|seq| seq.checked_add(1)) != Some(started_seq.get())
            || started_seq.get().checked_add(1) != Some(scheduled_seq.get())
        {
            return Err(invalid(*step, "invalid selected immediate retry authority"));
        }
        Ok(Some(*ticket))
    }
}

fn same_retry_identity(old: ActionTicket, next: ActionTicket) -> bool {
    next.run == old.run
        && next.step == old.step
        && next.action == old.action
        && next.run_attempt == old.run_attempt
        && next.capacity == old.capacity
        && next.seq == old.seq
        && next.idempotency_key == old.idempotency_key
        && next.fencing_token > old.fencing_token
        && vb_core::action::action_ticket_has_valid_key(next)
}
