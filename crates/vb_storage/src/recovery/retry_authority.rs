use super::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::retry_timer::RetryTimerV1;
use vb_core::{ActionTicket, SlotIdx, WorkflowDigest};

impl ActionReplayTracker {
    pub(crate) fn authorize_retry(&mut self, timer: &RetryTimerV1) -> RecoveryResult<()> {
        timer.validate().map_err(|_| invalid(timer.failed_ticket))?;
        let ticket = timer.failed_ticket;
        let key = (ticket.action, ticket.step);
        if !self.failed.contains(&key)
            || self.completed.contains(&key)
            || self
                .scheduled_tickets
                .get(&key)
                .is_none_or(|old| old.ticket != ticket)
            || self
                .retry_attempts
                .insert(key, timer.next_attempt)
                .is_some()
        {
            return Err(invalid(ticket));
        }
        Ok(())
    }

    pub(crate) fn consume_retry(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        abi: WorkflowDigest,
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        let Some(next) = self.retry_attempts.get(&key).copied() else {
            return Ok(());
        };
        let old = self
            .scheduled_tickets
            .get(&key)
            .ok_or_else(|| invalid(ticket))?;
        let previous = old.ticket;
        if !self.failed.contains(&key)
            || self.completed.contains(&key)
            || ticket.attempt != next
            || ticket.run != previous.run
            || ticket.run_attempt != previous.run_attempt
            || ticket.capacity != previous.capacity
            || ticket.idempotency_key != previous.idempotency_key
            || ticket.seq != previous.seq
            || old.input != input
            || old.output != output
            || old.action_abi_digest != abi
            || !vb_core::action::action_ticket_has_valid_key(ticket)
        {
            return Err(invalid(ticket));
        }
        self.retry_attempts.remove(&key);
        self.failed.remove(&key);
        self.scheduled_tickets.remove(&key);
        Ok(())
    }
}

fn invalid(ticket: ActionTicket) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: "invalid durable retry authority".into(),
    }
}
