#![forbid(unsafe_code)]

use super::super::super::{DurableActionOutcome, JournalEvent};
use super::super::tags::{
    TAG_ACTION_ABANDONED, TAG_ACTION_COMPLETED, TAG_ACTION_COMPLETED_ENVELOPE, TAG_ACTION_FAILED,
    TAG_ACTION_SCHEDULED, TAG_ACTION_SCHEDULED_TICKET,
};
use super::{serialize_record_kind_variant, serialize_routing_error};
use crate::EventSeq;
use serde::{Serialize, Serializer, ser::SerializeTuple};
use vb_core::{ActionTicket, RunId, SlotIdx, Taint, WorkflowDigest};

struct CompletedEnvelopePayload<'event> {
    head: CompletedEnvelopeHead<'event>,
    tail: CompletedEnvelopeTail<'event>,
}

struct CompletedEnvelopeHead<'event> {
    run: &'event RunId,
    seq: &'event EventSeq,
    ticket: &'event ActionTicket,
    output: &'event SlotIdx,
    outcome: &'event DurableActionOutcome,
}

struct CompletedEnvelopeTail<'event> {
    value: &'event [u8],
    encoded_len: &'event u32,
    taint: &'event Taint,
    value_digest: &'event [u8; 32],
    action_abi_digest: &'event WorkflowDigest,
}

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionScheduled { .. } => serialize_scheduled(event, serializer),
        JournalEvent::ActionCompletedEvent { .. } => serialize_completed(event, serializer),
        JournalEvent::ActionScheduledTicket { .. } => serialize_scheduled_ticket(event, serializer),
        JournalEvent::ActionCompletedEnvelope { .. } => {
            serialize_completed_envelope(event, serializer)
        }
        JournalEvent::ActionFailedEvent { .. } => serialize_failed(event, serializer),
        JournalEvent::ActionAbandoned { .. } => serialize_abandoned(event, serializer),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_scheduled<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            action,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ACTION_SCHEDULED,
            "ActionScheduled",
            &(run, seq, step, action, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_completed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ACTION_COMPLETED,
            "ActionCompletedEvent",
            &(run, seq, step, action, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_scheduled_ticket<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionScheduledTicket {
            run,
            seq,
            ticket,
            input,
            output,
            action_abi_digest,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ACTION_SCHEDULED_TICKET,
            "ActionScheduledTicket",
            &(run, seq, ticket, input, output, action_abi_digest),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_completed_envelope<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let Some(payload) = CompletedEnvelopePayload::from_event(event) else {
        return serialize_routing_error::<S>();
    };
    serialize_record_kind_variant(
        serializer,
        TAG_ACTION_COMPLETED_ENVELOPE,
        "ActionCompletedEnvelope",
        &payload,
    )
}

impl<'event> CompletedEnvelopePayload<'event> {
    fn from_event(event: &'event JournalEvent) -> Option<Self> {
        let head = CompletedEnvelopeHead::from_event(event)?;
        let tail = CompletedEnvelopeTail::from_event(event)?;
        Some(Self { head, tail })
    }
}

impl<'event> CompletedEnvelopeHead<'event> {
    fn from_event(event: &'event JournalEvent) -> Option<Self> {
        let JournalEvent::ActionCompletedEnvelope {
            run,
            seq,
            ticket,
            output,
            outcome,
            ..
        } = event
        else {
            return None;
        };
        Some(Self {
            run,
            seq,
            ticket,
            output,
            outcome,
        })
    }
}

impl<'event> CompletedEnvelopeTail<'event> {
    fn from_event(event: &'event JournalEvent) -> Option<Self> {
        let JournalEvent::ActionCompletedEnvelope {
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } = event
        else {
            return None;
        };
        Some(Self {
            value: value.as_slice(),
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        })
    }
}

impl Serialize for CompletedEnvelopePayload<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(10)?;
        self.head.serialize_into(&mut tuple)?;
        self.tail.serialize_into(&mut tuple)?;
        tuple.end()
    }
}

impl CompletedEnvelopeHead<'_> {
    fn serialize_into<T>(&self, tuple: &mut T) -> Result<(), T::Error>
    where
        T: SerializeTuple,
    {
        tuple.serialize_element(self.run)?;
        tuple.serialize_element(self.seq)?;
        tuple.serialize_element(self.ticket)?;
        tuple.serialize_element(self.output)?;
        tuple.serialize_element(self.outcome)?;
        Ok(())
    }
}

impl CompletedEnvelopeTail<'_> {
    fn serialize_into<T>(&self, tuple: &mut T) -> Result<(), T::Error>
    where
        T: SerializeTuple,
    {
        tuple.serialize_element(self.value)?;
        tuple.serialize_element(self.encoded_len)?;
        tuple.serialize_element(self.taint)?;
        tuple.serialize_element(self.value_digest)?;
        tuple.serialize_element(self.action_abi_digest)?;
        Ok(())
    }
}

fn serialize_failed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ACTION_FAILED,
            "ActionFailedEvent",
            &(run, seq, step, action, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_abandoned<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::ActionAbandoned { run, seq, ticket } => serialize_record_kind_variant(
            serializer,
            TAG_ACTION_ABANDONED,
            "ActionAbandoned",
            &(run, seq, ticket),
        ),
        _ => serialize_routing_error::<S>(),
    }
}
