#![allow(unused_imports, dead_code)]

pub(crate) use crate::keys::{
    blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key, index_workflow_key,
    journal_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
};
pub(crate) use crate::queue::BatchBuilder;
pub(crate) use crate::recovery::{ActionReplayTracker, RunSnapshot};
pub(crate) use crate::{
    BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, DIGEST_BYTES, EventSeq, FjallJournal,
    IndexStatusState, JournalError, JournalEvent, JournalWriterQueue, KeyspaceProfile, MAGIC_BLOB,
    MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT,
    MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES,
    MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
    PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
    PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
    RecordKind, RunHeaderRecord, StorageKey, StorageLimits, WorkflowSourceRecord,
    append_journal_event, decode_record, decode_record_header, encode_record, encode_record_header,
    flush_profile, init_keyspaces, keyspace_options_for, open_store, put_blob, put_run_header,
    put_workflow_source, read_blob, read_run_events, replay_journal, verify_digest_match,
    write_snapshot,
};
pub(crate) use vb_core::{
    ActionId, CODE_REGISTRY, CapabilitySet, DiagnosticCode, RunId, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest, WorkflowId,
};

// --- Section 4: Journal Lifecycle BDD Tests ---

pub(crate) fn open_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal should open");
    (temp_dir, journal)
}

pub(crate) fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

// =========================================================================
// Section: Adversarial Record Header Decode Tests
// =========================================================================

pub(crate) fn encode_and_patch_field(
    event: &JournalEvent,
    kind: RecordKind,
    offset: usize,
    new_bytes: &[u8],
) -> Vec<u8> {
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    let end = offset.saturating_add(new_bytes.len());
    assert!(end <= 56, "patch must be within CRC-protected region");
    encoded
        .get_mut(offset..end)
        .expect("patch range valid")
        .copy_from_slice(new_bytes);
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;
    encoded
}
