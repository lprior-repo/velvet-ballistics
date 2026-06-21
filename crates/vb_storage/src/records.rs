#![forbid(unsafe_code)]
//! Durable record types for storage.

mod entities;
mod kinds;
mod status;

pub use entities::{
    BlobRecord, CompiledIrRecord, RecoveryStampRecord, RunHeaderRecord, WorkflowSourceRecord,
};
pub use kinds::RecordKind;

pub use status::{
    KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
};


#[cfg(test)]
mod tests;
