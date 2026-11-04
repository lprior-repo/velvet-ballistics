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
        RecordKind::GateDecision => deserialize_gate_decision(variant),
        // Retired: pre-migration grant records decode to an explicit
        // error instead of a resurrected variant. Tag 46 stays reserved
        // in RecordKind so old records fail loudly, never silently.
        RecordKind::CapabilityGrant => Err(de::Error::custom(
            "capability grant records are retired; reissue under the scoped grant lifecycle",
        )),
        _ => Err(de::Error::custom(
            "record kind is not a policy decision event",
        )),
    }
}

fn deserialize_gate_decision<'de, A>(variant: A) -> Result<JournalEvent, A::Error>
where
    A: VariantAccess<'de>,
{
    let payload: (
        vb_core::RunId,
        crate::EventSeq,
        vb_core::manual_gate::GateOutcomeRecord,
    ) = variant.newtype_variant()?;
    let (run, seq, record) = payload;
    if record.run != run {
        return Err(de::Error::custom(format_args!(
            "GateDecisionEvent outer run {} must equal record.run {}",
            run.get(),
            record.run.get()
        )));
    }
    Ok(JournalEvent::GateDecisionEvent {
        run,
        seq,
        record: Box::new(record),
    })
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
