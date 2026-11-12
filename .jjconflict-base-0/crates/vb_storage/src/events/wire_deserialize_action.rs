#![forbid(unsafe_code)]

use super::super::super::{DurableActionOutcome, JournalEvent};
use crate::{EventSeq, RecordKind};
use serde::de::{self, VariantAccess};
use vb_core::{ActionTicket, RunId, SlotIdx, Taint, WorkflowDigest};

type ActionCompletedEnvelopeFields = (
    RunId,
    EventSeq,
    ActionTicket,
    SlotIdx,
    DurableActionOutcome,
    Vec<u8>,
    u32,
    Taint,
    [u8; 32],
    WorkflowDigest,
);

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::ActionScheduled => deserialize_scheduled(variant),
        RecordKind::ActionCompleted => deserialize_completed(variant),
        RecordKind::ActionScheduledTicket => deserialize_scheduled_ticket(variant),
        RecordKind::ActionCompletedEnvelope => deserialize_completed_envelope(variant),
        RecordKind::ActionFailed => deserialize_failed(variant),
        RecordKind::ActionAbandoned => deserialize_abandoned(variant),
        _ => Err(de::Error::custom("record kind is not an action event")),
    }
}

fn deserialize_scheduled<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, action, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::ActionScheduled {
        run,
        seq,
        step,
        action,
        attempt,
    })
}

fn deserialize_completed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, action, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::ActionCompletedEvent {
        run,
        seq,
        step,
        action,
        attempt,
    })
}

fn deserialize_scheduled_ticket<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, ticket, input, output, action_abi_digest) = variant.newtype_variant()?;
    Ok(JournalEvent::ActionScheduledTicket {
        run,
        seq,
        ticket,
        input,
        output,
        action_abi_digest,
    })
}

fn deserialize_completed_envelope<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let fields = variant.newtype_variant()?;
    Ok(completed_envelope_from_fields(fields))
}

fn deserialize_failed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, action, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::ActionFailedEvent {
        run,
        seq,
        step,
        action,
        attempt,
    })
}

fn deserialize_abandoned<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, ticket) = variant.newtype_variant()?;
    Ok(JournalEvent::ActionAbandoned { run, seq, ticket })
}

fn completed_envelope_from_fields(fields: ActionCompletedEnvelopeFields) -> JournalEvent {
    let (run, seq, ticket, output, outcome, value, encoded_len, taint, value_digest, digest) =
        fields;
    JournalEvent::ActionCompletedEnvelope {
        run,
        seq,
        ticket,
        output,
        outcome,
        value,
        encoded_len,
        taint,
        value_digest,
        action_abi_digest: digest,
    }
}
