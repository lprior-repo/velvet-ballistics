//! Incident checkpoint projection from journal events.

use super::types::IncidentCheckpoint;
use crate::events::JournalEvent;

pub(super) fn checkpoint_from_event(event: &JournalEvent) -> IncidentCheckpoint {
    IncidentCheckpoint {
        seq: event.seq(),
        kind: event.record_kind(),
        step: checkpoint_step(event),
        action: checkpoint_action(event),
        slot: checkpoint_slot(event),
        attempt: event.attempt(),
    }
}

fn checkpoint_step(event: &JournalEvent) -> Option<u16> {
    match event {
        JournalEvent::StepStarted { step, .. }
        | JournalEvent::StepSucceeded { step, .. }
        | JournalEvent::ActionScheduled { step, .. }
        | JournalEvent::ActionCompletedEvent { step, .. }
        | JournalEvent::ActionFailedEvent { step, .. }
        | JournalEvent::WaitScheduledEvent { step, .. }
        | JournalEvent::AskScheduledEvent { step, .. }
        | JournalEvent::AskAnsweredEvent { step, .. }
        | JournalEvent::RetryScheduledEvent { step, .. } => Some(step.get()),
        JournalEvent::ActionScheduledTicket { ticket, .. }
        | JournalEvent::ActionCompletedEnvelope { ticket, .. } => Some(ticket.step.get()),
        _ => None,
    }
}

fn checkpoint_action(event: &JournalEvent) -> Option<u16> {
    match event {
        JournalEvent::ActionScheduled { action, .. }
        | JournalEvent::ActionCompletedEvent { action, .. }
        | JournalEvent::ActionFailedEvent { action, .. } => Some(action.get()),
        JournalEvent::ActionScheduledTicket { ticket, .. }
        | JournalEvent::ActionCompletedEnvelope { ticket, .. } => Some(ticket.action.get()),
        _ => None,
    }
}

fn checkpoint_slot(event: &JournalEvent) -> Option<u16> {
    match event {
        JournalEvent::StepSucceeded { output, .. }
        | JournalEvent::ActionCompletedEnvelope { output, .. } => Some(output.get()),
        JournalEvent::SlotWrittenEvent { slot, .. } => Some(slot.get()),
        JournalEvent::RunFinished { result, .. } => Some(result.get()),
        JournalEvent::RunAnswered { slot_idx, .. } => Some(slot_idx.get()),
        JournalEvent::ActionScheduledTicket { output, .. } => Some(output.get()),
        _ => None,
    }
}
