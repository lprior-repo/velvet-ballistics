#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use crate::RecordKind;
use serde::de::{self, VariantAccess};

pub(super) fn deserialize<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    match kind {
        RecordKind::PolicyDecision => deserialize_policy_decision(variant),
        _ => Err(de::Error::custom("record kind is not a policy decision event")),
    }
}

fn deserialize_policy_decision<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let (run, seq, decision) = variant.newtype_variant()?;
    Ok(JournalEvent::PolicyDecisionEvent { run, seq, decision })
}