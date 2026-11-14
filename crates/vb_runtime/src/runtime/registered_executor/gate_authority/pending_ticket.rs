use super::{ExecutorError, RegisteredExecutionError};
use vb_core::action::ActionTicket;
use vb_storage::JournalEvent;

pub(super) type PendingTicket = (ActionTicket, u64);

pub(super) fn advance_pending_ticket(
    pending: &mut Option<PendingTicket>,
    event: &JournalEvent,
    opening_seq: u64,
    action: vb_core::ActionId,
) -> Result<(), RegisteredExecutionError> {
    match event {
        JournalEvent::ActionScheduledTicket {
            run, seq, ticket, ..
        } if seq.get() < opening_seq && ticket.action == action && *run == ticket.run => {
            schedule_pending_ticket(pending, *ticket, seq.get())?;
        }
        JournalEvent::ActionCompletedEnvelope { seq, ticket, .. }
        | JournalEvent::ActionAbandoned { seq, ticket, .. }
            if seq.get() < opening_seq && completion_matches(*pending, *ticket) =>
        {
            *pending = None;
        }
        JournalEvent::ActionFailedEvent { seq, .. }
            if seq.get() < opening_seq
                && pending.is_some_and(|entry| failure_matches_pending(entry, event)) =>
        {
            *pending = None;
        }
        _ => {}
    }
    Ok(())
}

fn schedule_pending_ticket(
    pending: &mut Option<PendingTicket>,
    ticket: ActionTicket,
    scheduled_seq: u64,
) -> Result<(), RegisteredExecutionError> {
    if pending.is_some() {
        return Err(ExecutorError::Unauthorized.into());
    }
    *pending = Some((ticket, scheduled_seq));
    Ok(())
}

fn completion_matches(pending: Option<PendingTicket>, ticket: ActionTicket) -> bool {
    pending.is_some_and(|(current, _)| current == ticket)
}

fn failure_matches_pending(pending: PendingTicket, event: &JournalEvent) -> bool {
    let (ticket, scheduled_seq) = pending;
    match event {
        JournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        } => {
            *run == ticket.run
                && seq.get() > scheduled_seq
                && *step == ticket.step
                && *action == ticket.action
                && *attempt == ticket.attempt
        }
        _ => false,
    }
}
