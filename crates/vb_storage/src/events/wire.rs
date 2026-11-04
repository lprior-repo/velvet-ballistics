#![forbid(unsafe_code)]

#[path = "wire_category.rs"]
mod category;
#[path = "wire_compat.rs"]
mod compat;
#[path = "wire_deserialize.rs"]
mod deserialize;
#[path = "wire_serialize.rs"]
mod serialize;
#[path = "wire_tags.rs"]
mod tags;

pub use compat::classify_journal_envelope;
pub(crate) use compat::is_schema_one_shared_envelope_compatible;
pub(crate) use deserialize::decode_journal_event_payload_for_envelope;
