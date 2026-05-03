#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

pub mod recovery;
pub mod constants;
pub mod types;
pub mod events;
pub mod records;
pub mod keys;
pub mod codec;
pub mod binary;
pub mod headers;
pub mod journal;
pub mod queue;
pub mod batch;
pub mod error;
pub mod admission;
pub mod blobs;
pub mod snapshots;
pub mod indexes;
pub mod artifacts;

// Re-export everything from modules that was public in the original lib.rs
pub use constants::*;
pub use types::*;
pub use events::JournalEvent;
pub use records::{RecordKind, WorkflowSourceRecord, CompiledIrRecord, RunHeaderRecord, BlobRecord};
pub use types::StorageKey;
pub use keys::{encode_key, workflow_source_key, compiled_ir_key, run_header_key, run_event_key, run_snapshot_key, blob_key, index_status_key, index_workflow_key, index_action_key};
pub use codec::{encode_record, decode_record, encode_record_header, decode_record_header, verify_digest_match};
pub use journal::FjallJournal;
pub use queue::{JournalWriterQueue, BatchBuilder};
pub use batch::JournalWriteBatch;
pub use error::JournalError;
pub use admission::{VerificationProof, AcceptedArtifact, submit_artifact, admit_compiled_artifact};

use std::path::Path;
use crate::recovery::{ActionReplayTracker, RecoveryHydration};

/// Opens the Fjall-backed storage engine.
pub fn open_store(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Initializes all declared keyspaces by opening the store.
pub fn init_keyspaces(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Appends one journal event without forcing a durability barrier.
pub fn append_journal_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    journal.append_journaled(event)
}

/// Stores immutable workflow source bytes by digest.
pub fn put_workflow_source(
    journal: &FjallJournal,
    record: &WorkflowSourceRecord,
) -> Result<(), JournalError> {
    journal.put_workflow_source(record)
}

/// Stores compiled IR bytes by digest.
pub fn put_compiled_ir(
    journal: &FjallJournal,
    record: &CompiledIrRecord,
) -> Result<(), JournalError> {
    journal.put_compiled_ir(record)
}

/// Stores run metadata by run id.
pub fn put_run_header(
    journal: &FjallJournal,
    record: &RunHeaderRecord,
) -> Result<(), JournalError> {
    journal.put_run_header(record)
}

/// Writes a compact run snapshot.
pub fn write_snapshot(journal: &FjallJournal, snapshot: &recovery::RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
}

/// Stores a bounded blob by digest.
pub fn put_blob(journal: &FjallJournal, record: &BlobRecord) -> Result<(), JournalError> {
    journal.put_blob(record)
}

/// Reads a stored blob by digest.
pub fn read_blob(
    journal: &FjallJournal,
    digest: [u8; DIGEST_BYTES],
) -> Result<Option<BlobRecord>, JournalError> {
    journal.blob(digest)
}

/// Reads one run's journal events in replay order.
pub fn read_run_events(
    journal: &FjallJournal,
    run: vb_core::RunId,
) -> Result<Vec<JournalEvent>, JournalError> {
    journal.events_for_run(run)
}

/// Replays one run's full journal through the recovery path.
pub fn replay_journal(
    journal: &FjallJournal,
    run: vb_core::RunId,
    tracker: &mut ActionReplayTracker,
) -> recovery::RecoveryResult<Vec<JournalEvent>> {
    recovery::recover_full_journal(journal, run, tracker)
}

/// Recovers summary hydration for every durable run without a terminal event.
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> recovery::RecoveryResult<Vec<RecoveryHydration>> {
    recovery::recover_all_incomplete_runs(journal)
}

/// Flushes one queued writer batch using each event's durability profile.
pub fn flush_profile(
    queue: &JournalWriterQueue,
    journal: &FjallJournal,
) -> Result<JournalWriterFlushReport, JournalError> {
    queue.flush_batch(journal)
}
