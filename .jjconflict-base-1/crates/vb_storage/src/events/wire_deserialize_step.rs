#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use crate::RecordKind;
use serde::de::{self, VariantAccess};

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::StepStarted => deserialize_started(variant),
        RecordKind::StepSucceeded => deserialize_succeeded(variant),
        RecordKind::StepFailed => deserialize_failed(variant),
        RecordKind::SlotWritten => deserialize_slot_written(variant),
        _ => Err(de::Error::custom("record kind is not a step event")),
    }
}

fn deserialize_started<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::StepStarted {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_succeeded<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, output) = variant.newtype_variant()?;
    Ok(JournalEvent::StepSucceeded {
        run,
        seq,
        step,
        output,
    })
}

fn deserialize_failed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, step, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::StepFailed {
        run,
        seq,
        step,
        attempt,
    })
}

fn deserialize_slot_written<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, slot, value, extra, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt,
    })
}
