#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use crate::RecordKind;
use serde::de::{self, VariantAccess};

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::CompatibilityDecision => deserialize_compat_decision(variant),
        _ => Err(de::Error::custom(
            "record kind is not a compatibility decision event",
        )),
    }
}

fn deserialize_compat_decision<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let payload: (
        vb_core::RunId,
        crate::EventSeq,
        vb_core::compat::CompatDecision,
    ) = variant.newtype_variant()?;
    let (run, seq, decision) = payload;
    Ok(JournalEvent::CompatibilityDecisionEvent { run, seq, decision })
}
