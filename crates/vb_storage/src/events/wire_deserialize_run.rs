#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use crate::RecordKind;
use serde::de::{self, VariantAccess};

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::RunAccepted => deserialize_accepted(variant),
        RecordKind::RunAdmission => deserialize_admission(variant),
        RecordKind::RunCancelled => deserialize_cancelled(variant),
        RecordKind::RunKilled => deserialize_killed(variant),
        RecordKind::RunFinished => deserialize_finished(variant),
        RecordKind::RunFailed => deserialize_failed(variant),
        RecordKind::RunResumed => deserialize_resumed(variant),
        RecordKind::RunRetried => deserialize_retried(variant),
        RecordKind::RunAnswered => deserialize_answered(variant),
        _ => Err(de::Error::custom("record kind is not a run event")),
    }
}

fn deserialize_accepted<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, workflow) = variant.newtype_variant()?;
    Ok(JournalEvent::RunAccepted { run, seq, workflow })
}

fn deserialize_admission<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, artifact_digest, granted_capabilities, policy) = variant.newtype_variant()?;
    Ok(JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest,
        granted_capabilities,
        policy,
    })
}

fn deserialize_cancelled<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, attempt, reason) = variant.newtype_variant()?;
    Ok(JournalEvent::RunCancelled {
        run,
        seq,
        attempt,
        reason,
    })
}

fn deserialize_killed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::RunKilled { run, seq, attempt })
}

fn deserialize_finished<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, result, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::RunFinished {
        run,
        seq,
        result,
        attempt,
    })
}

fn deserialize_failed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, attempt) = variant.newtype_variant()?;
    Ok(JournalEvent::RunFailedEvent { run, seq, attempt })
}

fn deserialize_resumed<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, timestamp) = variant.newtype_variant()?;
    Ok(JournalEvent::RunResumed {
        run,
        seq,
        timestamp,
    })
}

fn deserialize_retried<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, timestamp) = variant.newtype_variant()?;
    Ok(JournalEvent::RunRetried {
        run,
        seq,
        timestamp,
    })
}

fn deserialize_answered<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, slot_idx, answer, timestamp) = variant.newtype_variant()?;
    Ok(JournalEvent::RunAnswered {
        run,
        seq,
        slot_idx,
        answer,
        timestamp,
    })
}
