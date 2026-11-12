#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use crate::RecordKind;
use serde::de::{self, VariantAccess};

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::WaitScheduled => deserialize_wait_scheduled(variant),
        RecordKind::AskScheduled => deserialize_ask_scheduled(variant),
        RecordKind::AskAnswered => deserialize_ask_answered(variant),
        RecordKind::WaitResolved => deserialize_wait_resolved(variant),
        RecordKind::RetryScheduled => deserialize_retry_scheduled(variant),
        RecordKind::AskTimedOut => deserialize_ask_timed_out(variant),
        _ => Err(de::Error::custom("record kind is not a wait or ask event")),
    }
}

fn deserialize_wait_scheduled<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::WaitScheduledEvent {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_ask_scheduled<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::AskScheduledEvent {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_ask_answered<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::AskAnsweredEvent {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_wait_resolved<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::WaitResolvedEvent {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_retry_scheduled<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::RetryScheduledEvent {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_ask_timed_out<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::AskTimedOutEvent {
        run,
        seq,
        step,
        attempt,
    })
}
