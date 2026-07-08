#![forbid(unsafe_code)]

use super::super::super::super::{DurableActionOutcome, JournalEvent};
use crate::EventSeq;
use vb_core::{ActionId, ActionTicket, RunId, SlotIdx, StepIdx, Taint, WorkflowDigest};

use super::super::LegacyJournalEvent;

struct CompletedEnvelopeParts {
    head: CompletedEnvelopeHead,
    tail: CompletedEnvelopeTail,
}

struct CompletedEnvelopeHead {
    run: RunId,
    seq: EventSeq,
    ticket: ActionTicket,
    output: SlotIdx,
    outcome: DurableActionOutcome,
}

struct CompletedEnvelopeTail {
    value: Vec<u8>,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
    action_abi_digest: WorkflowDigest,
}

impl CompletedEnvelopeHead {
    fn new(
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        output: SlotIdx,
        outcome: DurableActionOutcome,
    ) -> Self {
        Self {
            run,
            seq,
            ticket,
            output,
            outcome,
        }
    }
}

impl CompletedEnvelopeTail {
    fn new(
        value: Vec<u8>,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        action_abi_digest: WorkflowDigest,
    ) -> Self {
        Self {
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        }
    }
}

pub(super) fn from_legacy(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::ActionScheduled {
            run,
            seq,
            step,
            action,
            attempt,
        } => scheduled(run, seq, step, action, attempt),
        LegacyJournalEvent::ActionCompletedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        } => completed_event(run, seq, step, action, attempt),
        LegacyJournalEvent::ActionScheduledTicket {
            run,
            seq,
            ticket,
            input,
            output,
            action_abi_digest,
        } => scheduled_ticket(run, seq, ticket, input, output, action_abi_digest),
        other => envelope_failure_or_dispatch(other),
    }
}

fn envelope_failure_or_dispatch(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::ActionCompletedEnvelope {
            run,
            seq,
            ticket,
            output,
            outcome,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        } => completed_envelope(CompletedEnvelopeParts {
            head: CompletedEnvelopeHead::new(run, seq, ticket, output, outcome),
            tail: CompletedEnvelopeTail::new(
                value,
                encoded_len,
                taint,
                value_digest,
                action_abi_digest,
            ),
        }),
        other => failure_or_dispatch(other),
    }
}

fn failure_or_dispatch(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        } => failed(run, seq, step, action, attempt),
        LegacyJournalEvent::ActionAbandoned { run, seq, ticket } => abandoned(run, seq, ticket),
        other => super::into_current_by_category(other),
    }
}

pub(super) fn scheduled(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq,
        step,
        action,
        attempt,
    }
}

pub(super) fn completed_event(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq,
        step,
        action,
        attempt,
    }
}

pub(super) fn scheduled_ticket(
    run: RunId,
    seq: EventSeq,
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    action_abi_digest: WorkflowDigest,
) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run,
        seq,
        ticket,
        input,
        output,
        action_abi_digest,
    }
}

fn completed_envelope(parts: CompletedEnvelopeParts) -> JournalEvent {
    let CompletedEnvelopeParts { head, tail } = parts;
    JournalEvent::ActionCompletedEnvelope {
        run: head.run,
        seq: head.seq,
        ticket: head.ticket,
        output: head.output,
        outcome: head.outcome,
        value: tail.value,
        encoded_len: tail.encoded_len,
        taint: tail.taint,
        value_digest: tail.value_digest,
        action_abi_digest: tail.action_abi_digest,
    }
}

pub(super) fn failed(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run,
        seq,
        step,
        action,
        attempt,
    }
}

pub(super) fn abandoned(run: RunId, seq: EventSeq, ticket: ActionTicket) -> JournalEvent {
    JournalEvent::ActionAbandoned { run, seq, ticket }
}
