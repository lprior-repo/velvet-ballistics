use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::journal::FjallJournal;
use crate::queue::JournalWriterQueue;
use crate::records::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord};
use crate::recovery::{self, ActionReplayTracker, RunSnapshot};
use crate::types::JournalWriterFlushReport;

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
    digest: crate::BlobDigest,
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
