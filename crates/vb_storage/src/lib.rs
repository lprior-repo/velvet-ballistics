#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

// ============================================================================
// Submodules
// ============================================================================

pub mod admission;
pub mod artifacts;
pub mod batch;
pub mod binary;
pub mod blobs;
pub mod codec;
pub mod constants;
pub mod error;
pub mod events;
pub mod headers;
pub mod indexes;
pub mod keys;
pub mod queue;
pub mod records;
pub mod recovery;
pub mod snapshots;
pub mod tests;
pub mod types;

// ============================================================================
// Re-exports from submodules
// ============================================================================

// Core types
pub use constants::*;
pub use error::JournalError;
pub use events::JournalEvent;
pub use records::{
    BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
};
pub use types::*;
pub use recovery::{ActionReplayTracker, RunSnapshot};

// Journal
pub use journal::FjallJournal;

// Batch
pub use batch::JournalWriteBatch;

// Queue
pub use queue::JournalWriterQueue;

// Codec
pub use codec::{
    decode_record, decode_record_header, encode_record, encode_record_header,
    verify_digest_match,
};

// Admission
pub use admission::{AcceptedArtifact, VerificationProof, admit_compiled_artifact, submit_artifact};

// ============================================================================
// Convenience wrapper functions
// ============================================================================

/// Opens the Fjall-backed storage engine.
pub fn open_store(path: impl AsRef<std::path::Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Initializes all declared keyspaces by opening the store.
pub fn init_keyspaces(path: impl AsRef<std::path::Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Replays one run's full journal through the recovery path.
pub fn replay_journal(
    journal: &FjallJournal,
    run: vb_core::RunId,
    tracker: &mut ActionReplayTracker,
) -> recovery::RecoveryResult<Vec<JournalEvent>> {
    recovery::recover_full_journal(journal, run, tracker)
}

/// Flushes one queued writer batch using each event's durability profile.
pub fn flush_profile(
    queue: &JournalWriterQueue,
    journal: &FjallJournal,
) -> Result<queue::JournalWriterFlushReport, JournalError> {
    queue.flush_batch(journal)
}
