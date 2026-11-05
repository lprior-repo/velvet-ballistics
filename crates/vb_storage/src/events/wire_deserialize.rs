#![forbid(unsafe_code)]

#[path = "wire_deserialize_action.rs"]
mod action;
#[path = "wire_deserialize_compat.rs"]
mod compat;
#[path = "wire_deserialize_policy.rs"]
mod policy;
#[path = "wire_deserialize_run.rs"]
mod run;
#[path = "wire_deserialize_step.rs"]
mod step;
#[path = "wire_deserialize_wait_ask.rs"]
mod wait_ask;

use super::super::JournalEvent;
use super::category::{JournalRecordCategory, journal_record_category};
use super::tags::JOURNAL_EVENT_VARIANTS;
use crate::{JournalError, RecordKind, types::RecordEnvelope};
use serde::{
    Deserialize, Deserializer,
    de::{self, EnumAccess, Visitor},
};
use std::fmt;

struct JournalEventVisitor;

impl<'de> Visitor<'de> for JournalEventVisitor {
    type Value = JournalEvent;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stable record-kind tagged JournalEvent variant")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (tag, variant) = data.variant::<u16>()?;
        let kind = record_kind_from_tag::<A::Error>(tag)?;
        deserialize_known_kind(kind, variant)
    }
}

fn record_kind_from_tag<E>(tag: u16) -> Result<RecordKind, E>
where
    E: de::Error,
{
    RecordKind::from_id(tag).ok_or_else(|| E::custom("unknown stable JournalEvent record kind tag"))
}

fn deserialize_known_kind<'de, A>(kind: RecordKind, variant: A) -> Result<JournalEvent, A::Error>
where
    A: de::VariantAccess<'de>,
{
    match journal_record_category(kind) {
        Some(JournalRecordCategory::Continuation) => {
            Err(de::Error::custom("bounded carrier context required"))
        }
        Some(JournalRecordCategory::Run) => run::deserialize(kind, variant),
        Some(JournalRecordCategory::Step) => step::deserialize(kind, variant),
        Some(JournalRecordCategory::Action) => action::deserialize(kind, variant),
        Some(JournalRecordCategory::WaitAsk) => wait_ask::deserialize(kind, variant),
        Some(JournalRecordCategory::Policy) => policy::deserialize(kind, variant),
        Some(JournalRecordCategory::Compat) => compat::deserialize(kind, variant),
        None => Err(de::Error::custom(
            "non-journal record kind tag used for JournalEvent payload",
        )),
    }
}

impl<'de> Deserialize<'de> for JournalEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("JournalEvent", JOURNAL_EVENT_VARIANTS, JournalEventVisitor)
    }
}

pub(crate) fn decode_journal_event_payload_for_envelope<'payload>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
) -> Result<(JournalEvent, &'payload [u8]), JournalError> {
    if envelope.record_kind == RecordKind::RunContinuation.id() {
        return Err(JournalError::InvalidEvent);
    }
    let (event, remainder) = postcard::take_from_bytes::<JournalEvent>(payload)
        .map_err(JournalError::PostcardDecodeFailed)?;
    crate::codec::validate_journal_event_record_kind(envelope, &event)?;
    Ok((event, remainder))
}
