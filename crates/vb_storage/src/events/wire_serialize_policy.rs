#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::TAG_POLICY_DECISION;
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::PolicyDecisionEvent {
            run,
            seq,
            decision,
        } => serialize_record_kind_variant(
            serializer,
            TAG_POLICY_DECISION,
            "PolicyDecisionEvent",
            &(run, seq, decision),
        ),
        _ => serialize_routing_error::<S>(),
    }
}