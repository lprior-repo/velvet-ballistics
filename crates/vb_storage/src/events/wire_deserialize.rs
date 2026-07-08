#![forbid(unsafe_code)]

#[path = "wire_deserialize_action.rs"]
mod action;
#[path = "wire_deserialize_run.rs"]
mod run;
#[path = "wire_deserialize_step.rs"]
mod step;
#[path = "wire_deserialize_wait_ask.rs"]
mod wait_ask;

use super::super::JournalEvent;
use super::category::{JournalRecordCategory, journal_record_category};
use super::legacy::decode_legacy_journal_event_payload;
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
        Some(JournalRecordCategory::Run) => run::deserialize(kind, variant),
        Some(JournalRecordCategory::Step) => step::deserialize(kind, variant),
        Some(JournalRecordCategory::Action) => action::deserialize(kind, variant),
        Some(JournalRecordCategory::WaitAsk) => wait_ask::deserialize(kind, variant),
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
    match postcard::take_from_bytes::<JournalEvent>(payload) {
        Ok((event, remainder)) if event.record_kind().id() == envelope.record_kind => {
            Ok((event, remainder))
        }
        Ok((event, _)) => decode_legacy_or_payload_mismatch(envelope, payload, event),
        Err(error) => decode_legacy_or_decode_error(envelope, payload, error),
    }
}

fn decode_legacy_or_payload_mismatch<'payload>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
    stable_event: JournalEvent,
) -> Result<(JournalEvent, &'payload [u8]), JournalError> {
    match decode_legacy_journal_event_payload(envelope, payload) {
        Ok(decoded) => Ok(decoded),
        Err(_) => Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: envelope.record_kind,
            payload_kind: stable_event.record_kind().id(),
        }),
    }
}

fn decode_legacy_or_decode_error<'payload>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
    stable_error: postcard::Error,
) -> Result<(JournalEvent, &'payload [u8]), JournalError> {
    match decode_legacy_journal_event_payload(envelope, payload) {
        Ok(decoded) => Ok(decoded),
        Err(_) => Err(JournalError::PostcardDecodeFailed(stable_error)),
    }
}
