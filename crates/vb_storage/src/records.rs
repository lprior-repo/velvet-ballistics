#![forbid(unsafe_code)]
//! Durable record types for storage.

mod kind;
mod status;
mod structs;

pub use kind::RecordKind;
pub use status::{
    KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
};
pub use structs::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord};
