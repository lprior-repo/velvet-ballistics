//! Convenience wrapper functions and test helpers.
//!
//! Convenience functions provide short-hands over the `FjallJournal` API.
//! Test helpers (feature-gated behind `test` / `test-support`) produce
//! well-formed records for integration testing.

use crate::constants;
use crate::recovery;
use crate::{
    AcceptedArtifact, ActionReplayTracker, BlobRecord, CompiledIrRecord, EventSeq,
    FjallJournal, JournalError, JournalEvent, JournalWriterFlushReport, JournalWriterQueue,
    RunHeaderRecord, RunSnapshot, VerificationProof, WorkflowSourceRecord,
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

// NOTE: put_compiled_ir is pub(crate) for internal use only.
// External code must use submit_artifact / admit_compiled_artifact which
// perform proper validation and bind all artifact fields cryptographically.
// Direct writes bypass admission validation and could allow mutation of
// non-digest-bound fields (warnings, required_capabilities, accepted_at_seq).
//
// SECURITY: This function is provided for integration testing only.
// It allows tests to verify storage boundary validation with malformed data.
// Production code should always use submit_artifact.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn __put_compiled_ir_for_testing(
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

// ============================================================================
// Test helpers
// ============================================================================

pub(crate) fn accepted_compiled_ir_record_for_test(ir: Vec<u8>) -> CompiledIrRecord {
    let workflow = accepted_workflow_for_test(&ir);
    let digest = workflow.digest();
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0; constants::DIGEST_BYTES]);
    let artifact_ir = postcard::to_allocvec(&parts).expect("WorkflowParts should encode");
    let artifact = AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: crate::admission::compute_policy_digest(&workflow)
            .expect("policy digest should compute"),
        ir: artifact_ir,
        verification: VerificationProof::new(digest, 15, true),
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: Box::new([]),
    };
    let envelope = postcard::to_allocvec(&artifact).expect("AcceptedArtifact should encode");
    let metadata_hash = crate::admission::compute_artifact_metadata_hash(&artifact);
    CompiledIrRecord {
        digest,
        ir: envelope,
        metadata_hash: Some(metadata_hash),
    }
}

fn accepted_workflow_for_test(seed: &[u8]) -> vb_core::CompiledWorkflow {
    let mut parts = vb_core::WorkflowParts {
        name: Box::<str>::from("accepted_compiled_ir_record_for_test"),
        digest: vb_core::WorkflowDigest::from_bytes([0; constants::DIGEST_BYTES]),
        nodes: Box::new([
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(0),
                output: Some(vb_core::SlotIdx::new(0)),
                next: Some(vb_core::StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::I64(seed_value_for_test(seed))]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let digest_bytes = postcard::to_allocvec(&parts).expect("WorkflowParts should encode");
    parts.digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&digest_bytes).into());
    vb_core::CompiledWorkflow::try_from_parts(parts).expect("WorkflowParts should compile")
}

fn seed_value_for_test(seed: &[u8]) -> i64 {
    seed.iter().fold(0_i64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(i64::from(*byte))
    })
}

