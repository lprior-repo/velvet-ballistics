#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::TAG_COMPATIBILITY_DECISION;
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::CompatibilityDecisionEvent { run, seq, decision } => {
            serialize_record_kind_variant(
                serializer,
                TAG_COMPATIBILITY_DECISION,
                "CompatibilityDecisionEvent",
                &(run, seq, decision),
            )
        }
        _ => serialize_routing_error::<S>(),
    }
}
