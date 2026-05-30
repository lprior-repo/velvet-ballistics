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
#[cfg(miri)]
pub mod codec_miri_tests;
pub mod constants;
pub mod error;
pub mod events;
pub mod headers;
pub mod indexes;
pub mod journal;
#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_codec;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_magic;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_schema;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_kind;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_payload_len;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_crc;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_digest_checks_vb_2bzz;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_hydrate_proofs;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_admission;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_postcard_envelope_wire;

#[cfg(all(kani, feature = "kani-typed-partitioned-ids"))]
pub mod kani_typed_partitioned_ids;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_decode_order;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_numeric_fields;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_payload_bounds;

#[cfg(kani)]
pub mod kani_journal_duplicate;

pub mod keys;
pub mod process_lock;

// PO-010: register the deterministic replay proptest module for `cargo test --lib`
// evidence collection. This is test-only verification wiring and does not alter
// production runtime behavior.
#[cfg(test)]
#[path = "po010_proptests.rs"]
mod proptests;

#[cfg(test)]
#[path = "proptest_storage.rs"]
mod proptest_storage;

#[cfg(test)]
#[path = "proptests.rs"]
mod proptest_integration;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;

pub mod queue;
pub mod records;
pub mod recovery;
pub mod security_tests;
pub mod slot_extra;
pub mod snapshots;
pub mod tests;
pub mod trimming;
pub mod types;
pub mod vb_2bok_durability_gate_tests;

// ============================================================================
// Re-exports from submodules
// ============================================================================

// Core types
pub use constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, DIGEST_BYTES, KEYSPACE_BLOB, KEYSPACE_COMPILED_IR,
    KEYSPACE_INDEX_ACTION, KEYSPACE_INDEX_STATUS, KEYSPACE_INDEX_WORKFLOW, KEYSPACE_RUN_EVENT,
    KEYSPACE_RUN_HEADER, KEYSPACE_RUN_SNAPSHOT, KEYSPACE_WORKFLOW_SOURCE, MAGIC_BLOB,
    MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT,
    MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BATCH_COUNT, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES,
    MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
    PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
    PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
pub use error::JournalError;
pub use events::{DurableActionOutcome, JournalEvent};
pub use records::{
    BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
};
pub use recovery::{ActionReplayTracker, RunSnapshot};
pub use slot_extra::{
    DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraEnvelope,
    SlotWrittenExtraError, decode_slot_written_extra, encode_slot_written_extra,
};
pub use types::{
    DurabilityProfile, EventSeq, FjallConfig, IndexStatusState, JournalBatchSize,
    JournalQueueCapacity, JournalWriterFlushReport, JournalWriterQueueProfileCounts,
    KeyspaceProfile, RecordEnvelope, RecordHeader, StorageKey, StorageLimits, keyspace_options_for,
};

// Journal
pub use journal::incident::{
    IncidentAnalysis, SideEffect, SideEffectCertainty, analyze_incident_events, build_repair_hints,
    derive_lifecycle_state_from_events, lifecycle_state_to_inspect_status,
};
pub use journal::{EventReplayLimit, FjallJournal};

// Batch
pub use batch::JournalWriteBatch;

// Queue
pub use queue::JournalWriterQueue;

// Trimming
pub use trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};

// Codec
pub use codec::{
    decode_record, decode_record_header, encode_record, encode_record_header, verify_digest_match,
};

// Admission
pub use admission::{
    AcceptedArtifact, VerificationProof, VerificationWarning, admit_compiled_artifact,
    submit_artifact, submit_artifact_with_contracts,
};

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
    expected_action_abi_digests: &[(vb_core::ActionId, vb_core::WorkflowDigest)],
    expected_policy_digests: &[(vb_core::StepIdx, vb_core::WorkflowDigest)],
) -> recovery::RecoveryResult<Vec<JournalEvent>> {
    recovery::recover_full_journal(
        journal,
        run,
        tracker,
        expected_action_abi_digests,
        expected_policy_digests,
    )
}

/// Flushes one queued writer batch using each event's durability profile.
pub fn flush_profile(
    queue: &JournalWriterQueue,
    journal: &FjallJournal,
) -> Result<JournalWriterFlushReport, JournalError> {
    queue.flush_batch(journal)
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
pub fn write_snapshot(journal: &FjallJournal, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
}

/// Stores a bounded blob by digest.
pub fn put_blob(journal: &FjallJournal, record: &BlobRecord) -> Result<(), JournalError> {
    journal.put_blob(record)
}

/// Reads a stored blob by digest.
pub fn read_blob(
    journal: &FjallJournal,
    digest: [u8; constants::DIGEST_BYTES],
) -> Result<Option<BlobRecord>, JournalError> {
    journal.blob(digest)
}

/// Replays one run's events in contiguous sequence order.
pub fn read_run_events(
    journal: &FjallJournal,
    run: vb_core::RunId,
) -> Result<Vec<JournalEvent>, JournalError> {
    journal.events_for_run(run)
}
