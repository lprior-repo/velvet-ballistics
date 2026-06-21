#![forbid(unsafe_code)]
#![allow(unused_imports)]
//! Test helpers that re-export vb_storage internals for use in tests.
//!
//! After refactoring, the lib.rs exports changed but tests.rs still expects
//! the old structure with items available via `super::*`. This module provides
//! a single `use crate::test_helpers::*` import to restore test functionality.

use vb_core::{ActionId, DiagnosticCode, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

pub(crate) use crate::constants::{
    CURRENT_SCHEMA_VERSION, DIGEST_BYTES, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
    MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
    MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
    MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR,
    PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT,
    PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
    RECORD_HEADER_LEN,
};

pub(crate) use crate::error::JournalError;
pub(crate) use crate::events::JournalEvent;
pub(crate) use crate::types::{
    keyspace_options_for, EventSeq, KeyspaceProfile, StorageKey, StorageLimits,
};

pub(crate) use crate::records::{
    BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
};

pub(crate) use crate::keys::{
    blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key, index_workflow_key,
    journal_key, run_event_key, run_header_key, run_seq_gap_key, run_snapshot_key,
    workflow_source_key,
};

pub(crate) use crate::codec::{
    decode_record, decode_record_header, encode_record, encode_record_header, verify_digest_match,
};

pub(crate) use crate::queue::BatchBuilder;
pub(crate) use crate::queue::JournalWriterQueue;
pub(crate) use crate::recovery::{ActionReplayTracker, RunSnapshot};

pub(crate) use crate::FjallJournal;

pub(crate) use crate::open_store;

pub(crate) use crate::replay_journal;

pub(crate) use crate::flush_profile;

pub(crate) use crate::init_keyspaces;

// ---------------------------------------------------------------------------
// Free function wrappers for methods that tests expect as free functions
// ---------------------------------------------------------------------------

pub(crate) fn read_blob(
    journal: &FjallJournal,
    digest: [u8; DIGEST_BYTES],
) -> Result<Option<BlobRecord>, JournalError> {
    journal.blob(digest)
}

pub(crate) fn read_run_events(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Vec<JournalEvent>, JournalError> {
    journal.events_for_run(run)
}

pub(crate) fn append_journal_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    journal.append_journaled(event)
}

pub(crate) fn write_snapshot(journal: &FjallJournal, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
}

#[cfg(test)]
pub(crate) fn make_temp_journal_pair() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}
