#![cfg(test)]

use crate::{DurableActionOutcome, EventSeq, JournalEvent};
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx, Taint};

pub(super) fn step_event(step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

pub(super) fn step_event_at(seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

pub(super) fn action_scheduled_at(seq: u64, step: u16, action: u16, attempt: u16) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt,
    }
}

pub(super) fn action_completed(step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

pub(super) fn action_completed_at(seq: u64, step: u16, action: u16, attempt: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt,
    }
}

pub(super) fn action_failed(step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

pub(super) fn action_failed_at(seq: u64, step: u16, action: u16, attempt: u16) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt,
    }
}

pub(super) fn slot_written_at(seq: u64, slot: u16) -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(slot),
        value: None,
        extra: None,
        attempt: 1,
    }
}

pub(super) fn step_succeeded_at(seq: u64, step: u16, output: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        output: SlotIdx::new(output),
    }
}

pub(super) fn run_finished_at(seq: u64, result: u16) -> JournalEvent {
    JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        result: SlotIdx::new(result),
        attempt: 1,
    }
}

pub(super) fn run_failed() -> JournalEvent {
    JournalEvent::RunFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(10),
        attempt: 1,
    }
}

pub(super) fn run_cancelled() -> JournalEvent {
    JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(10),
        attempt: 1,
        reason: None,
    }
}

pub(super) fn run_killed() -> JournalEvent {
    JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(10),
        attempt: 1,
        reason: None,
    }
}

fn action_ticket(step: u16, action: u16, attempt: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(step),
        seq: SeqNo::new(99),
        action: ActionId::new(action),
        attempt,
        idempotency_key: 123,
        capacity: 3,
        mock: Default::default(),
    }
}

pub(super) fn action_scheduled_ticket_at(seq: u64, step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        ticket: action_ticket(step, action, 2),
        input: SlotIdx::new(4),
        output: SlotIdx::new(5),
    }
}

pub(super) fn action_completed_envelope_at(seq: u64, step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEnvelope {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        ticket: action_ticket(step, action, 2),
        output: SlotIdx::new(5),
        outcome: DurableActionOutcome::Ready,
        value: Vec::new(),
        encoded_len: 0,
        taint: Taint::Clean,
        value_digest: [0; 32],
    }
}
