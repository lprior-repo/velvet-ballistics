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
        _ => Err(de::Error::custom(
            "record kind is not a policy decision event",
        )),
    }
}

fn deserialize_policy_decision<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let payload: (
        vb_core::RunId,
        crate::EventSeq,
        vb_core::policy::DecisionRecord,
    ) = variant.newtype_variant()?;
    let (run, seq, decision) = payload;
    if decision.ticket.run != run {
        return Err(de::Error::custom(format_args!(
            "PolicyDecisionEvent outer run {} must equal decision.ticket.run {}",
            run.get(),
            decision.ticket.run.get()
        )));
    }
    Ok(JournalEvent::PolicyDecisionEvent { run, seq, decision })
}
