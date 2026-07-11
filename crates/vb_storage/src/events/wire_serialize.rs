#![forbid(unsafe_code)]

#[path = "wire_serialize_action.rs"]
mod action;
#[path = "wire_serialize_run.rs"]
mod run;
#[path = "wire_serialize_step.rs"]
mod step;
#[path = "wire_serialize_wait_ask.rs"]
mod wait_ask;

use super::super::JournalEvent;
use super::category::{JournalRecordCategory, journal_record_category};
use serde::{Serialize, Serializer};

fn serialize_record_kind_variant<S, T>(
    serializer: S,
    tag: u16,
    variant: &'static str,
    payload: &T,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + ?Sized,
{
    serializer.serialize_newtype_variant("JournalEvent", u32::from(tag), variant, payload)
}

fn serialize_routing_error<S>() -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Err(serde::ser::Error::custom(
        "JournalEvent variant routed to an unsupported record-kind serializer",
    ))
}

impl Serialize for JournalEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_by_category(self, serializer)
    }
}

fn serialize_by_category<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match journal_record_category(event.record_kind()) {
        Some(JournalRecordCategory::Run) => run::serialize(event, serializer),
        Some(JournalRecordCategory::Step) => step::serialize(event, serializer),
        Some(JournalRecordCategory::Action) => action::serialize(event, serializer),
        Some(JournalRecordCategory::WaitAsk) => wait_ask::serialize(event, serializer),
        None => serialize_routing_error::<S>(),
    }
}
