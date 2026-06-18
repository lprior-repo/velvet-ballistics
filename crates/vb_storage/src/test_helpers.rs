#![forbid(unsafe_code)]
#![allow(unused_imports)]
//! Test helpers that re-export vb_storage internals for use in tests.
//!
//! After refactoring, the lib.rs exports changed but tests.rs still expects
//! the old structure with items available via `super::*`. This module provides
//! a single `use crate::test_helpers::*` import to restore test functionality.

use vb_core::{ActionId, DiagnosticCode, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

pub use crate::constants::{
    CURRENT_SCHEMA_VERSION, DIGEST_BYTES, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
    MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
    MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
    MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR,
    PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT,
    PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
    RECORD_HEADER_LEN,
};

pub use crate::error::JournalError;
pub use crate::events::JournalEvent;
pub use crate::types::{
    keyspace_options_for, EventSeq, KeyspaceProfile, StorageKey, StorageLimits,
};

pub use crate::records::{
    BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
};

pub use crate::keys::{
    blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key, index_workflow_key,
    journal_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
};

pub use crate::codec::{
    decode_record, decode_record_header, encode_record, encode_record_header, verify_digest_match,
};

pub use crate::queue::BatchBuilder;
pub use crate::queue::JournalWriterQueue;
pub use crate::recovery::{ActionReplayTracker, RunSnapshot};

pub use crate::FjallJournal;

pub use crate::open_store;

pub use crate::replay_journal;

pub use crate::flush_profile;

pub use crate::init_keyspaces;

// ---------------------------------------------------------------------------
// Free function wrappers for methods that tests expect as free functions
// ---------------------------------------------------------------------------

pub fn read_blob(
    journal: &FjallJournal,
    digest: [u8; DIGEST_BYTES],
) -> Result<Option<BlobRecord>, JournalError> {
    journal.blob(digest)
}

pub fn read_run_events(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Vec<JournalEvent>, JournalError> {
    journal.events_for_run(run)
}

pub fn append_journal_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    journal.append_journaled(event)
}

pub fn write_snapshot(journal: &FjallJournal, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
}
