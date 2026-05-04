//! Fjall-backed journal implementation.
//!
//! Provides the main storage interface for workflow artifacts,
//! run metadata, journal events, snapshots, and blobs.

use std::path::Path;
use std::sync::Mutex;

use serde::de::DeserializeOwned;

use crate::{
    batch::JournalWriteBatch,
    codec::{decode_record, encode_record},
    constants::{
        KEYSPACE_BLOB, KEYSPACE_COMPILED_IR, KEYSPACE_INDEX_ACTION, KEYSPACE_INDEX_STATUS,
        KEYSPACE_INDEX_WORKFLOW, KEYSPACE_RUN_EVENT, KEYSPACE_RUN_HEADER, KEYSPACE_RUN_SNAPSHOT,
        KEYSPACE_WORKFLOW_SOURCE, MAGIC_COMPILED_ARTIFACT, MAGIC_JOURNAL_EVENT,
        MAGIC_WORKFLOW_SOURCE, MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    events::JournalEvent,
    process_lock::ProcessLock,
    keys::{
        compiled_ir_key,
        run_event_key, workflow_source_key,
    },

    records::{CompiledIrRecord, RecordKind, WorkflowSourceRecord},
    types::{EventSeq, FjallConfig, KeyspaceProfile},
};

use crate::keys::run_prefix_key;
use fjall::Readable;

/// Verifies that content bytes hash to the expected digest.
/// Used at admission time to prevent digest forgery.
pub(crate) fn verify_content_digest(content: &[u8], expected: &[u8]) -> Result<(), JournalError> {
    let computed = blake3::hash(content);
    if computed.as_bytes() == expected {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}

/// Fjall-backed append journal.
pub struct FjallJournal {
    pub(crate) database: fjall::Database,
    pub(crate) workflow_source: fjall::Keyspace,
    pub(crate) compiled_ir: fjall::Keyspace,
    pub(crate) run_header: fjall::Keyspace,
    pub(crate) events: fjall::Keyspace,
    pub(crate) run_snapshot: fjall::Keyspace,
    pub(crate) blob: fjall::Keyspace,
    pub(crate) index_status: fjall::Keyspace,
    pub(crate) index_workflow: fjall::Keyspace,
    pub(crate) index_action: fjall::Keyspace,
    #[allow(dead_code)]
    write_lock: Mutex<()>,
    _process_lock: ProcessLock,
}

impl FjallJournal {
    /// Opens or creates the journal at `path`.
    pub fn open(path: impl AsRef<Path>, config: Option<FjallConfig>) -> Result<Self, JournalError> {
        let config = config.unwrap_or_default();
        let path_ref = path.as_ref();
        let database = fjall::Database::builder(path_ref)
            .cache_size(config.cache_size_bytes)
            .open()?;

        let workflow_source = database.keyspace(KEYSPACE_WORKFLOW_SOURCE, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let compiled_ir = database.keyspace(KEYSPACE_COMPILED_IR, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let run_header = database.keyspace(KEYSPACE_RUN_HEADER, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let events = database.keyspace(KEYSPACE_RUN_EVENT, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let run_snapshot = database.keyspace(KEYSPACE_RUN_SNAPSHOT, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let blob = database.keyspace(KEYSPACE_BLOB, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Blob)
        })?;
        let index_status = database.keyspace(KEYSPACE_INDEX_STATUS, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_workflow = database.keyspace(KEYSPACE_INDEX_WORKFLOW, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_action = database.keyspace(KEYSPACE_INDEX_ACTION, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let _process_lock = ProcessLock::acquire(path_ref)?;
        Ok(Self {
            database,
            workflow_source,
            compiled_ir,
            run_header,
            events,
            run_snapshot,
            blob,
            index_status,
            index_workflow,
            index_action,
            write_lock: Mutex::new(()),
            _process_lock,
        })
    }

    /// Returns all declared keyspace names after a successful open.
    #[must_use]
    pub const fn declared_keyspaces() -> [&'static str; 9] {
        [
            KEYSPACE_WORKFLOW_SOURCE,
            KEYSPACE_COMPILED_IR,
            KEYSPACE_RUN_HEADER,
            KEYSPACE_RUN_EVENT,
            KEYSPACE_RUN_SNAPSHOT,
            KEYSPACE_BLOB,
            KEYSPACE_INDEX_STATUS,
            KEYSPACE_INDEX_WORKFLOW,
            KEYSPACE_INDEX_ACTION,
        ]
    }

    /// Stores immutable workflow source bytes by digest.
    ///
    /// The source bytes are verified against the claimed digest before storage.
    pub fn put_workflow_source(&self, record: &WorkflowSourceRecord) -> Result<(), JournalError> {
        verify_content_digest(&record.source, &record.digest.as_bytes())?;
        let key = workflow_source_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )?;
        self.workflow_source.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads workflow source bytes by digest.
    pub fn workflow_source(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<Option<WorkflowSourceRecord>, JournalError> {
        let key = workflow_source_key(digest.as_bytes())?;
        self.decode_optional(
            &self.workflow_source,
            key.as_slice(),
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
    }

    /// Stores compiled IR bytes by digest.
    pub fn put_compiled_ir(&self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        let key = compiled_ir_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            record,
            MAX_COMPILED_IR_BYTES,
        )?;
        self.compiled_ir.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads compiled IR bytes by digest.
    pub fn compiled_ir(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<Option<CompiledIrRecord>, JournalError> {
        let key = compiled_ir_key(digest.as_bytes())?;
        self.decode_optional(
            &self.compiled_ir,
            key.as_slice(),
            MAGIC_COMPILED_ARTIFACT,
            MAX_COMPILED_IR_BYTES,
        )
    }

    /// Appends one event without forcing a durability barrier.
    pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unpersisted(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unpersisted(event)?;
        self.persist_strict()
    }

    /// Appends multiple events with a single strict durability barrier.
    pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> {
        for event in events {
            self.append_unpersisted(event)?;
        }
        if !events.is_empty() {
            self.persist_strict()?;
        }
        Ok(())
    }

    pub(crate) fn append_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.events.contains_key(key)? {
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        self.events.insert(key.to_vec(), value)?;
        Ok(())
    }

    pub(crate) fn append_queued_unpersisted(
        &self,
        event: &JournalEvent,
    ) -> Result<(), JournalError> {
        match self.append_unpersisted(event) {
            Ok(()) => Ok(()),
            Err(JournalError::DuplicateEvent { run, seq }) => {
                let key = run_event_key(run, seq)?;
                let Some(value) = self.events.get(key)? else {
                    return Err(JournalError::DuplicateEvent { run, seq });
                };
                let (_, existing) = decode_record::<JournalEvent>(
                    value.as_ref(),
                    MAGIC_JOURNAL_EVENT,
                    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                )?;
                if existing == *event {
                    Ok(())
                } else {
                    Err(JournalError::DuplicateEvent { run, seq })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Forces a strict durability barrier.
    pub fn persist_strict(&self) -> Result<(), JournalError> {
        self.database.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }

    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: vb_core::RunId) -> Result<Vec<JournalEvent>, JournalError> {
        let start_seq = self
            .latest_snapshot_seq(run)
            .ok()
            .flatten()
            .unwrap_or(EventSeq::new(0));
        self.events_for_run_from(run, start_seq)
    }

    /// Returns events for a run starting from a given sequence, with validation.
    fn events_for_run_from(
        &self,
        run: vb_core::RunId,
        start_seq: EventSeq,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
        let mut expected = start_seq;
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, run_prefix_key(run)?) {
            let value = item.value()?;
            let (_, event) = decode_record(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;
            crate::codec::validate_replayed_event(run, expected, &event)?;
            expected = crate::codec::next_seq(expected)?;
            replay.push(event);
        }

        Ok(replay)
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn decode_optional<T: DeserializeOwned>(
        &self,
        keyspace: &fjall::Keyspace,
        key: &[u8],
        magic: u32,
        max_payload_len: u32,
    ) -> Result<Option<T>, JournalError> {
        let Some(value) = keyspace.get(key)? else {
            return Ok(None);
        };
        let (_, record) = decode_record(value.as_ref(), magic, max_payload_len)?;
        Ok(Some(record))
    }

    /// Creates a new atomic cross-keyspace write batch.
    pub fn batch(&self) -> JournalWriteBatch<'_> {
        JournalWriteBatch::new(self)
    }
}

impl Drop for FjallJournal {
    fn drop(&mut self) {
        if let Err(e) = self.database.persist(fjall::PersistMode::SyncAll) {
            let _ = e;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::*;
    use crate::{
        BlobRecord, EventSeq, JournalEvent, RunHeaderRecord,
        WorkflowSourceRecord, CompiledIrRecord,
        constants::*,
        recovery::RunSnapshot,
    };
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_event(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
        }
    }

    // =========================================================================
    // Write/read round-trip tests
    // =========================================================================

    #[test]
    fn workflow_source_roundtrip() {
        let (_temp, journal) = temp_journal();
        let source = b"workflow: hello_world".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source: source.clone(),
        };
        journal.put_workflow_source(&record).expect("put should succeed");
        let loaded = journal.workflow_source(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("workflow source should be found");
        };
        assert_eq!(found.source, source);
        assert_eq!(found.digest, digest);
    }

    #[test]
    fn workflow_source_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let result = journal.workflow_source(missing).expect("lookup should succeed");
        assert_eq!(result, None, "missing digest should return None");
    }

    #[test]
    fn compiled_ir_roundtrip() {
        let (_temp, journal) = temp_journal();
        let ir = b"compiled-artifact-bytes".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord { digest, ir: ir.clone() };
        journal.put_compiled_ir(&record).expect("put should succeed");
        let loaded = journal.compiled_ir(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("compiled IR should be found");
        };
        assert_eq!(found.ir, ir);
    }

    #[test]
    fn compiled_ir_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0x00; DIGEST_BYTES]);
        let result = journal.compiled_ir(missing).expect("lookup should succeed");
        assert_eq!(result, None, "missing digest should return None");
    }

    #[test]
    fn run_header_roundtrip() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(12345);
        let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(7),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: 1700000000000,
        };
        journal.put_run_header(&record).expect("put should succeed");
        let loaded = journal.run_header(run).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("run header should be found");
        };
        assert_eq!(found.run, run);
        assert_eq!(found.workflow_id, WorkflowId::new(7));
        assert_eq!(found.status, 2);
        assert_eq!(found.accepted_at_ms, 1700000000000);
    }

    #[test]
    fn run_header_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let result = journal.run_header(RunId::new(999)).expect("lookup should succeed");
        assert_eq!(result, None, "missing run should return None");
    }

    #[test]
    fn snapshot_roundtrip() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(55);
        let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(10),
            workflow,
            slots: vec![0u8, 1u8],
            taint: vec![0u8, 0u8],
        };
        journal.put_snapshot(&snapshot).expect("put should succeed");
        let loaded = journal.snapshot(run, EventSeq::new(10)).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("snapshot should be found");
        };
        assert_eq!(found.run, run);
        assert_eq!(found.seq, EventSeq::new(10));
        assert_eq!(found.slots.len(), 2);
        assert_eq!(found.workflow, workflow);
    }

    #[test]
    fn blob_roundtrip() {
        let (_temp, journal) = temp_journal();
        let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord { digest, bytes: payload.clone() };
        journal.put_blob(&record).expect("put should succeed");
        let loaded = journal.blob(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("blob should be found");
        };
        assert_eq!(found.bytes, payload);
    }

    #[test]
    fn blob_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let result = journal.blob([0; DIGEST_BYTES]).expect("lookup should succeed");
        assert_eq!(result, None, "missing blob should return None");
    }

    #[test]
    fn blob_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3];
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
        let record = BlobRecord { digest: wrong_digest, bytes: payload };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "digest mismatch must be rejected, got {:?}",
            result
        );
    }

    // =========================================================================
    // Keyspace isolation — different run IDs don't interfere
    // =========================================================================

    #[test]
    fn events_for_run_isolates_different_runs() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        // Write events for run A
        let events_a = [
            make_event(run_a, 0),
            make_step_started(run_a, 1, 0),
        ];
        journal.append_strict_batch(&events_a).expect("batch A should succeed");

        // Write events for run B
        let events_b = [
            make_event(run_b, 0),
            make_step_started(run_b, 1, 0),
            make_step_started(run_b, 2, 1),
        ];
        journal.append_strict_batch(&events_b).expect("batch B should succeed");

        // Replay run A: should only get run A events
        let replayed_a = journal.events_for_run(run_a).expect("replay A should succeed");
        assert_eq!(replayed_a.len(), 2, "run A should have exactly 2 events");
        for event in &replayed_a {
            assert_eq!(event.run_id(), run_a, "replayed event must belong to run A");
        }

        // Replay run B: should only get run B events
        let replayed_b = journal.events_for_run(run_b).expect("replay B should succeed");
        assert_eq!(replayed_b.len(), 3, "run B should have exactly 3 events");
        for event in &replayed_b {
            assert_eq!(event.run_id(), run_b, "replayed event must belong to run B");
        }
    }

    #[test]
    fn run_headers_isolate_different_runs() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(10);
        let run_b = RunId::new(20);
        let digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

        journal.put_run_header(&RunHeaderRecord {
            run: run_a,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        }).expect("put A should succeed");

        journal.put_run_header(&RunHeaderRecord {
            run: run_b,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: 200,
        }).expect("put B should succeed");

        let header_a = journal.run_header(run_a).expect("get A should succeed").expect("A present");
        let header_b = journal.run_header(run_b).expect("get B should succeed").expect("B present");

        assert_eq!(header_a.workflow_id, WorkflowId::new(1));
        assert_eq!(header_a.status, 1);
        assert_eq!(header_b.workflow_id, WorkflowId::new(2));
        assert_eq!(header_b.status, 2);
    }

    #[test]
    fn snapshots_isolate_different_runs() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(50);
        let run_b = RunId::new(60);
        let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

        journal.put_snapshot(&RunSnapshot {
            run: run_a,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![0u8],
            taint: vec![],
        }).expect("put A should succeed");

        journal.put_snapshot(&RunSnapshot {
            run: run_b,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![1u8, 2u8, 3u8],
            taint: vec![0u8],
        }).expect("put B should succeed");

        let snap_a = journal.snapshot(run_a, EventSeq::new(1)).expect("get A").expect("present");
        let snap_b = journal.snapshot(run_b, EventSeq::new(1)).expect("get B").expect("present");

        assert_eq!(snap_a.run, run_a);
        assert_eq!(snap_a.slots.len(), 1);
        assert_eq!(snap_b.run, run_b);
        assert_eq!(snap_b.slots.len(), 3);
    }

    // =========================================================================
    // Sequential event ordering
    // =========================================================================

    #[test]
    fn events_for_run_returns_events_in_sequence_order() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);

        let events: Vec<JournalEvent> = (0..5).map(|i| {
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i),
                step: StepIdx::new(i as u16),
            }
        }).collect();
        journal.append_strict_batch(&events).expect("batch should succeed");

        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 5);
        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(event.seq().get(), i as u64, "event at index {} should have seq {}", i, i);
        }
    }

    #[test]
    fn events_for_run_rejects_sequence_gap() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(400);

        // Write seq 0 and seq 2 (gap at seq 1)
        let e0 = make_event(run, 0);
        let e2 = make_event(run, 2);
        journal.append_unpersisted(&e0).expect("append 0");
        journal.append_unpersisted(&e2).expect("append 2");

        let result = journal.events_for_run(run);
        assert!(
            matches!(result, Err(JournalError::SequenceGap { .. })),
            "sequence gap must be detected during replay, got {:?}",
            result
        );
    }

    #[test]
    fn events_for_run_returns_empty_for_unknown_run() {
        let (_temp, journal) = temp_journal();
        let result = journal.events_for_run(RunId::new(99999)).expect("replay should succeed");
        assert_eq!(result.len(), 0, "unknown run should have zero events");
    }

    // =========================================================================
    // Duplicate event rejection
    // =========================================================================

    #[test]
    fn append_strict_rejects_duplicate_event() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(500);
        let event = make_event(run, 0);

        journal.append_strict(&event).expect("first append should succeed");
        let result = journal.append_strict(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate event must be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn append_journaled_succeeds_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(600);
        let event = make_event(run, 0);

        journal.append_journaled(&event).expect("append_journaled should succeed");
        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0], event);
    }

    #[test]
    fn append_strict_batch_writes_all_events() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(700);
        let events = [
            make_event(run, 0),
            make_step_started(run, 1, 0),
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
            },
        ];
        journal.append_strict_batch(&events).expect("batch should succeed");
        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0], events[0]);
        assert_eq!(replayed[1], events[1]);
        assert_eq!(replayed[2], events[2]);
    }

    #[test]
    fn append_strict_batch_on_empty_is_ok() {
        let (_temp, journal) = temp_journal();
        let result = journal.append_strict_batch(&[]);
        assert!(result.is_ok(), "empty batch should succeed");
    }

    // =========================================================================
    // Declared keyspaces
    // =========================================================================

    #[test]
    fn declared_keyspaces_count_matches_opened_keyspaces() {
        let declared = FjallJournal::declared_keyspaces();
        assert_eq!(declared.len(), 9, "there should be 9 declared keyspaces");
        let (_temp, _journal) = temp_journal();
        // If we got here, all keyspaces opened successfully
    }

    // =========================================================================
    // Verify content digest
    // =========================================================================

    #[test]
    fn verify_content_digest_accepts_valid() {
        let content = b"some bytes";
        let hash = blake3::hash(content);
        let result = verify_content_digest(content, hash.as_bytes());
        assert!(result.is_ok(), "valid content digest should pass");
    }

    #[test]
    fn verify_content_digest_rejects_mismatch() {
        let content = b"some bytes";
        let wrong = blake3::hash(b"other bytes");
        let result = verify_content_digest(content, wrong.as_bytes());
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "mismatched digest should fail"
        );
    }

    // =========================================================================
    // Multiple event types round-trip through journal
    // =========================================================================

    #[test]
    fn all_event_variant_roundtrip_through_journal() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(800);
        let digest = WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]);

        let events = [
            JournalEvent::RunAccepted { run, seq: EventSeq::new(0), workflow: digest },
            JournalEvent::StepStarted { run, seq: EventSeq::new(1), step: StepIdx::new(0) },
            JournalEvent::ActionScheduled { run, seq: EventSeq::new(2), step: StepIdx::new(0), action: vb_core::ActionId::new(1) },
            JournalEvent::SlotWrittenEvent { run, seq: EventSeq::new(3), slot: SlotIdx::new(0), value: None },
            JournalEvent::ActionCompletedEvent { run, seq: EventSeq::new(4), step: StepIdx::new(0), action: vb_core::ActionId::new(1) },
            JournalEvent::ActionFailedEvent { run, seq: EventSeq::new(5), step: StepIdx::new(1), action: vb_core::ActionId::new(2) },
            JournalEvent::WaitScheduledEvent { run, seq: EventSeq::new(6), step: StepIdx::new(1) },
            JournalEvent::AskScheduledEvent { run, seq: EventSeq::new(7), step: StepIdx::new(2) },
            JournalEvent::AskAnsweredEvent { run, seq: EventSeq::new(8), step: StepIdx::new(2) },
            JournalEvent::RetryScheduledEvent { run, seq: EventSeq::new(9), step: StepIdx::new(1) },
            JournalEvent::RunCancelled { run, seq: EventSeq::new(10) },
        ];

        journal.append_strict_batch(&events).expect("batch should succeed");
        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), events.len());
        for (i, (original, replayed_event)) in events.iter().zip(replayed.iter()).enumerate() {
            assert_eq!(original, replayed_event, "event at index {} mismatch", i);
        }
    }

    // =========================================================================
    // put_workflow_source with valid and invalid digests
    // =========================================================================

    #[test]
    fn workflow_source_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let source = b"workflow: real content".to_vec();
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let record = WorkflowSourceRecord {
            digest: wrong_digest,
            source,
        };
        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "wrong digest must be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn workflow_source_accepts_large_valid_payload() {
        let (_temp, journal) = temp_journal();
        // Build a source that is near but under the max (use 64 KiB)
        let source = vec![0x41u8; 65536];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source: source.clone(),
        };
        journal.put_workflow_source(&record).expect("put should succeed for large valid payload");
        let loaded = journal.workflow_source(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("large workflow source should be found");
        };
        assert_eq!(found.source.len(), source.len());
        assert_eq!(found.source, source);
    }

    #[test]
    fn workflow_source_stores_multiple_distinct_digests() {
        let (_temp, journal) = temp_journal();
        let source_a = b"workflow: a".to_vec();
        let source_b = b"workflow: b".to_vec();
        let digest_a = WorkflowDigest::from_bytes(blake3::hash(&source_a).into());
        let digest_b = WorkflowDigest::from_bytes(blake3::hash(&source_b).into());
        let record_a = WorkflowSourceRecord { digest: digest_a, source: source_a.clone() };
        let record_b = WorkflowSourceRecord { digest: digest_b, source: source_b.clone() };
        journal.put_workflow_source(&record_a).expect("put A should succeed");
        journal.put_workflow_source(&record_b).expect("put B should succeed");
        let loaded_a = journal.workflow_source(digest_a).expect("get A should succeed").expect("A present");
        let loaded_b = journal.workflow_source(digest_b).expect("get B should succeed").expect("B present");
        assert_eq!(loaded_a.source, source_a);
        assert_eq!(loaded_b.source, source_b);
    }

    // =========================================================================
    // put_snapshot with various sequence numbers
    // =========================================================================

    #[test]
    fn snapshot_multiple_sequences_same_run() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let workflow = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);
        for seq_val in 0u64..5 {
            let snapshot = RunSnapshot {
                run,
                seq: EventSeq::new(seq_val),
                workflow,
                slots: vec![seq_val as u8],
                taint: vec![],
            };
            journal.put_snapshot(&snapshot).expect("put should succeed");
        }
        // Each snapshot should be retrievable independently
        for seq_val in 0u64..5 {
            let loaded = journal.snapshot(run, EventSeq::new(seq_val))
                .expect("get should succeed")
                .expect("should be present");
            assert_eq!(loaded.seq, EventSeq::new(seq_val));
            assert_eq!(loaded.slots, vec![seq_val as u8]);
        }
    }

    #[test]
    fn snapshot_returns_none_for_missing_sequence() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(101);
        let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
        // Write seq 0 and seq 5
        let snap0 = RunSnapshot { run, seq: EventSeq::new(0), workflow, slots: vec![], taint: vec![] };
        let snap5 = RunSnapshot { run, seq: EventSeq::new(5), workflow, slots: vec![5u8], taint: vec![] };
        journal.put_snapshot(&snap0).expect("put 0");
        journal.put_snapshot(&snap5).expect("put 5");
        // seq 3 should be missing
        let result = journal.snapshot(run, EventSeq::new(3)).expect("get should succeed");
        assert_eq!(result, None, "missing snapshot seq should return None");
        // but seq 0 and 5 are present
        assert!(journal.snapshot(run, EventSeq::new(0)).expect("get 0").is_some());
        assert!(journal.snapshot(run, EventSeq::new(5)).expect("get 5").is_some());
    }

    #[test]
    fn snapshot_preserves_large_slots_and_taint() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(102);
        let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let slots = vec![0xAB_u8; 4096];
        let taint = vec![0x01_u8; 4096];
        let snapshot = RunSnapshot { run, seq: EventSeq::new(0), workflow, slots: slots.clone(), taint: taint.clone() };
        journal.put_snapshot(&snapshot).expect("put should succeed");
        let loaded = journal.snapshot(run, EventSeq::new(0)).expect("get should succeed").expect("present");
        assert_eq!(loaded.slots, slots);
        assert_eq!(loaded.taint, taint);
    }

    // =========================================================================
    // get_run_header / put_run_header extended round-trips
    // =========================================================================

    #[test]
    fn run_header_overwrite_updates_status() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(999);
        let digest = WorkflowDigest::from_bytes([0x01; DIGEST_BYTES]);
        let original = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        };
        journal.put_run_header(&original).expect("put original");
        let updated = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 3,
            accepted_at_ms: 100,
        };
        journal.put_run_header(&updated).expect("put updated");
        let loaded = journal.run_header(run).expect("get should succeed").expect("present");
        assert_eq!(loaded.status, 3, "status should be updated to 3");
        assert_eq!(loaded.run, run);
    }

    #[test]
    fn run_headers_returns_all_headers_in_order() {
        let (_temp, journal) = temp_journal();
        let digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
        // Insert in non-sorted order (run IDs 30, 10, 20)
        for run_id in [30u64, 10u64, 20u64] {
            let record = RunHeaderRecord {
                run: RunId::new(run_id),
                workflow_id: WorkflowId::new(run_id as u32),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: run_id,
            };
            journal.put_run_header(&record).expect("put should succeed");
        }
        let headers = journal.run_headers().expect("run_headers should succeed");
        assert_eq!(headers.len(), 3, "should have 3 headers");
        // run_headers are returned in key order (big-endian run IDs)
        let run_ids: Vec<u64> = headers.iter().map(|h| h.run.get()).collect();
        let mut sorted = run_ids.clone();
        sorted.sort();
        assert_eq!(run_ids, sorted, "headers should be in key order");
    }

    // =========================================================================
    // Event append and sequential read edge cases
    // =========================================================================

    #[test]
    fn append_queued_unpersisted_allows_idempotent_duplicate() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1100);
        let event = make_event(run, 0);
        journal.append_queued_unpersisted(&event).expect("first append should succeed");
        let result = journal.append_queued_unpersisted(&event);
        assert!(result.is_ok(), "idempotent duplicate of same event should succeed, got {:?}", result);
    }

    #[test]
    fn append_queued_unpersisted_rejects_different_duplicate() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1101);
        let event_a = make_event(run, 0);
        let mut event_b = make_event(run, 0);
        // Change the workflow digest so event_b differs
        if let JournalEvent::RunAccepted { ref mut workflow, .. } = event_b {
            *workflow = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        }
        journal.append_queued_unpersisted(&event_a).expect("first append should succeed");
        let result = journal.append_queued_unpersisted(&event_b);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "different event at same run/seq must be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn events_for_run_with_many_events() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1200);
        let count: u64 = 100;
        let events: Vec<JournalEvent> = (0..count).map(|i| {
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i),
                step: StepIdx::new(i as u16),
            }
        }).collect();
        journal.append_strict_batch(&events).expect("batch should succeed");
        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), count as usize);
        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(event.seq().get(), i as u64);
            assert_eq!(event.run_id(), run);
        }
    }

    // =========================================================================
    // Status index construction
    // =========================================================================

    #[test]
    fn status_index_stores_and_scans_markers() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(2000);
        let run_b = RunId::new(2001);
        // Insert status markers for different states
        journal.put_status_index(1, 1000, run_a).expect("status A");
        journal.put_status_index(2, 2000, run_a).expect("status B");
        journal.put_status_index(1, 3000, run_b).expect("status C");
        // Scan the entire keyspace
        let mut count = 0usize;
        for item in journal.index_status.iter() {
            let _ = item.key();
            count = count.saturating_add(1);
        }
        assert_eq!(count, 3, "should have 3 status index markers");
    }

    // =========================================================================
    // Workflow and action index construction
    // =========================================================================

    #[test]
    fn workflow_index_stores_markers() {
        let (_temp, journal) = temp_journal();
        let wf1 = WorkflowId::new(1);
        let wf2 = WorkflowId::new(2);
        let run_a = RunId::new(3000);
        let run_b = RunId::new(3001);
        journal.put_workflow_index(wf1, run_a).expect("wf idx A");
        journal.put_workflow_index(wf1, run_b).expect("wf idx B");
        journal.put_workflow_index(wf2, run_a).expect("wf idx C");
        let mut count = 0usize;
        for item in journal.index_workflow.iter() {
            let _ = item.key();
            count = count.saturating_add(1);
        }
        assert_eq!(count, 3, "should have 3 workflow index markers");
    }

    #[test]
    fn action_index_stores_markers() {
        let (_temp, journal) = temp_journal();
        let action1 = vb_core::ActionId::new(10);
        let action2 = vb_core::ActionId::new(20);
        let run = RunId::new(4000);
        let step_a = StepIdx::new(1);
        let step_b = StepIdx::new(2);
        journal.put_action_index(action1, run, step_a).expect("action idx A");
        journal.put_action_index(action1, run, step_b).expect("action idx B");
        journal.put_action_index(action2, run, step_a).expect("action idx C");
        let mut count = 0usize;
        for item in journal.index_action.iter() {
            let _ = item.key();
            count = count.saturating_add(1);
        }
        assert_eq!(count, 3, "should have 3 action index markers");
    }

    // =========================================================================
    // Cross-keyspace batch operations
    // =========================================================================

    #[test]
    fn batch_commits_across_multiple_keyspaces() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(5000);
        let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

        let source = b"batch workflow".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let workflow_record = WorkflowSourceRecord { digest: source_digest, source };

        let ir = b"batch ir".to_vec();
        let ir_digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let ir_record = CompiledIrRecord { digest: ir_digest, ir };

        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(42),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 9000,
        };

        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: source_digest,
        };

        let payload = vec![0xBB];
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let blob_record = BlobRecord { digest: blob_digest, bytes: payload };

        {
            let mut batch = journal.batch();
            batch.put_workflow_source(&workflow_record).expect("batch workflow source");
            batch.put_compiled_ir(&ir_record).expect("batch compiled ir");
            batch.put_run_header(&header).expect("batch run header");
            batch.append_event(&event).expect("batch event");
            batch.put_blob(&blob_record).expect("batch blob");
            batch.put_status_index(1, 100, run).expect("batch status idx");
            batch.put_workflow_index(WorkflowId::new(42), run).expect("batch workflow idx");
            batch.put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0)).expect("batch action idx");
            assert_eq!(batch.len(), 8, "batch should contain 8 operations");
            assert!(!batch.is_empty(), "batch should not be empty");
            batch.commit().expect("batch commit should succeed");
        }

        // Verify all keyspaces have the data
        assert!(journal.workflow_source(source_digest).expect("get ws").is_some());
        assert!(journal.compiled_ir(ir_digest).expect("get ir").is_some());
        assert!(journal.run_header(run).expect("get header").is_some());
        let replayed = journal.events_for_run(run).expect("get events");
        assert_eq!(replayed.len(), 1);
        assert!(journal.blob(blob_digest).expect("get blob").is_some());
    }

    #[test]
    fn batch_rejects_workflow_source_with_wrong_digest() {
        let (_temp, journal) = temp_journal();
        let source = b"real content".to_vec();
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let record = WorkflowSourceRecord { digest: wrong_digest, source };
        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch should reject digest mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn batch_rejects_blob_with_wrong_digest() {
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3];
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
        let record = BlobRecord { digest: wrong_digest, bytes: payload };
        let mut batch = journal.batch();
        let result = batch.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch should reject blob digest mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn batch_empty_commit_succeeds() {
        let (_temp, journal) = temp_journal();
        let batch = journal.batch();
        assert!(batch.is_empty(), "new batch should be empty");
        assert_eq!(batch.len(), 0, "new batch length should be 0");
        batch.commit().expect("empty batch commit should succeed");
    }

    #[test]
    fn batch_with_strict_durability() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(5001);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            status: 0,
            accepted_at_ms: 1,
        };
        let batch = journal.batch();
        let mut batch = batch.strict();
        batch.put_run_header(&header).expect("batch put should succeed");
        batch.commit().expect("strict batch commit should succeed");
        assert!(journal.run_header(run).expect("get").is_some());
    }

    // =========================================================================
    // Missing key lookups return correct error / None
    // =========================================================================

    #[test]
    fn compiled_ir_returns_none_for_unwritten_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);
        let result = journal.compiled_ir(missing).expect("lookup should succeed");
        assert_eq!(result, None, "missing compiled IR should return None");
    }

    #[test]
    fn snapshot_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let result = journal.snapshot(RunId::new(99999), EventSeq::new(0)).expect("lookup should succeed");
        assert_eq!(result, None, "missing snapshot should return None");
    }

    #[test]
    fn blob_returns_none_for_unwritten_digest() {
        let (_temp, journal) = temp_journal();
        let result = journal.blob([0x99; DIGEST_BYTES]).expect("lookup should succeed");
        assert_eq!(result, None, "missing blob should return None");
    }

    // =========================================================================
    // compiled_ir with digest verification
    // =========================================================================

    #[test]
    fn compiled_ir_roundtrip_large_payload() {
        let (_temp, journal) = temp_journal();
        let ir = vec![0x42u8; 65536];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord { digest, ir: ir.clone() };
        journal.put_compiled_ir(&record).expect("put should succeed");
        let loaded = journal.compiled_ir(digest).expect("get should succeed").expect("present");
        assert_eq!(loaded.ir, ir);
    }

    // =========================================================================
    // persist_strict succeeds on idle journal
    // =========================================================================

    #[test]
    fn persist_strict_succeeds_without_prior_writes() {
        let (_temp, journal) = temp_journal();
        journal.persist_strict().expect("persist_strict on idle journal should succeed");
    }
}
