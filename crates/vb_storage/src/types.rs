#![forbid(unsafe_code)]
//! Core storage types: configuration, profiles, and sequencing.

mod config;
mod index;
mod keys;
mod preview;
mod queue;
mod record;
mod seq;

// Re-exports from config
pub use config::{DurabilityProfile, FjallConfig, KeyspaceProfile, StorageLimits, keyspace_options_for};

// Re-exports from seq
pub use seq::EventSeq;

// Re-exports from queue
pub use queue::{
    JournalBatchSize, JournalQueueCapacity, JournalWriterFlushReport,
    JournalWriterQueueProfileCounts,
};

// Re-exports from record
pub use record::{RecordEnvelope, RecordHeader};

// Re-exports from index
pub use index::IndexStatusState;

// Re-exports from keys
pub use keys::StorageKey;

// Re-exports from preview
pub use preview::{DecodedPreview, PreviewConfig, PreviewPayload};
