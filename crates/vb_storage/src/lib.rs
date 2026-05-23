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
#[cfg(kani)]
pub mod kani_codec;

#[cfg(kani)]
pub mod kani_record_magic;

#[cfg(kani)]
pub mod kani_record_schema;

#[cfg(kani)]
pub mod kani_record_kind;

#[cfg(kani)]
pub mod kani_record_payload_len;

#[cfg(kani)]
pub mod kani_record_crc;

#[cfg(kani)]
pub mod kani_proof_flags_gap;

#[cfg(kani)]
pub mod kani_digest_checks_vb_2bzz;

#[cfg(kani)]
pub mod kani_recovery_hydrate;

#[cfg(kani)]
pub mod kani_admission;

pub mod keys;
pub mod process_lock;

// PO-010: register the deterministic replay proptest module for `cargo test --lib`
// evidence collection. This is test-only verification wiring and does not alter
// production runtime behavior.
#[cfg(test)]
#[path = "po010_proptests.rs"]
mod proptests;

pub mod queue;
pub mod records;
pub mod recovery;
pub mod security_tests;
pub mod snapshots;
pub mod tests;
pub mod trimming;
pub mod types;
pub mod vb_2bok_durability_gate_tests;

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
pub use recovery::{ActionReplayTracker, RunSnapshot};
pub use types::*;

// Journal
pub use journal::FjallJournal;
pub use journal::incident::{
    IncidentAnalysis, SideEffect, SideEffectCertainty, analyze_incident_events, build_repair_hints,
    derive_lifecycle_state_from_events, lifecycle_state_to_inspect_status,
};

// Batch
pub use batch::JournalWriteBatch;

// Queue
pub use queue::JournalWriterQueue;

// Types
pub use types::JournalWriterFlushReport;

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
