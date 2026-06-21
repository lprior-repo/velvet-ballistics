#![forbid(unsafe_code)]
//! Key encoding functions for Fjall keyspaces.
//!
//! Each key variant uses a specific binary format with a type prefix
//! followed by the payload fields in big-endian byte order.

mod decode;
pub mod encode;

// Re-export public API
pub use decode::{KeyPrefix, decode_storage_key, try_key_prefix};
pub use encode::{
    blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key, index_workflow_key,
    journal_key, recovery_stamp_key, run_event_key, run_header_key, run_seq_gap_key,
    run_snapshot_key, workflow_source_key,
};

pub(crate) use encode::run_prefix_key;

#[cfg(test)]
#[path = "keys/tests.rs"]
mod tests;
