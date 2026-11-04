#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::{TAG_CAPABILITY_GRANT, TAG_GATE_DECISION, TAG_POLICY_DECISION};
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::PolicyDecisionEvent { run, seq, decision } => {
            if decision.ticket.run != *run {
                return Err(serde::ser::Error::custom(format_args!(
                    "PolicyDecisionEvent outer run {} must equal decision.ticket.run {}",
                    run.get(),
                    decision.ticket.run.get()
                )));
            }
            serialize_record_kind_variant(
                serializer,
                TAG_POLICY_DECISION,
                "PolicyDecisionEvent",
                &(run, seq, decision),
            )
        }
        JournalEvent::GateDecisionEvent { run, seq, record } => {
            if record.run != *run {
                return Err(serde::ser::Error::custom(format_args!(
                    "GateDecisionEvent outer run {} must equal record.run {}",
                    run.get(),
                    record.run.get()
                )));
            }
            serialize_record_kind_variant(
                serializer,
                TAG_GATE_DECISION,
                "GateDecisionEvent",
                &(run, seq, record),
            )
        }
        JournalEvent::CapabilityGrantEvent { run, seq, grant } => {
            if grant.run != *run {
                return Err(serde::ser::Error::custom(format_args!(
                    "CapabilityGrantEvent outer run {} must equal grant.run {}",
                    run.get(),
                    grant.run.get()
                )));
            }
            if !grant.is_valid() {
                return Err(serde::ser::Error::custom(
                    "CapabilityGrantEvent grant is structurally invalid",
                ));
            }
            serialize_record_kind_variant(
                serializer,
                TAG_CAPABILITY_GRANT,
                "CapabilityGrantEvent",
                &(run, seq, grant),
            )
        }
        _ => serialize_routing_error::<S>(),
    }
}
