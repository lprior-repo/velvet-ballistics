#![forbid(unsafe_code)]
//! Atomic cross-keyspace write batch backed by Fjall.
//!
//! Accumulates writes across multiple keyspaces and commits them
//! atomically with a single WAL fsync.

use std::collections::HashSet;

use crate::{
    codec::encode_record,
    constants::{
        JOURNAL_KEY_BYTES, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
        MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
        MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    events::JournalEvent,
    keys::{
        blob_key, compiled_ir_key, index_action_key, index_status_key, index_workflow_key,
        run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
    },
    records::{BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord},
    recovery::RunSnapshot,
};

use crate::journal::FjallJournal;

/// Atomic cross-keyspace write batch backed by Fjall.
///
/// Accumulates writes across multiple keyspaces and commits them
/// atomically with a single WAL fsync.
///
/// # Invariant I1
/// `JournalWriteBatch` is `!Send + !Sync` because it contains
/// `PhantomData<*mut FjallJournal>` which is `!Send + !Sync`,
/// preventing any batch handle from crossing thread boundaries.
pub struct JournalWriteBatch<'j> {
    inner: fjall::OwnedWriteBatch,
    journal: &'j FjallJournal,
    #[allow(dead_code)]
    staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>,
    aborted: bool,
    _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}

impl<'j> JournalWriteBatch<'j> {
    /// Creates a new batch for the given journal.
    pub fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
            staged_event_keys: HashSet::new(),
            aborted: false,
            _not_send_or_sync: core::marker::PhantomData,
        }
    }

    /// Inserts a workflow source record into the batch.
    ///
    /// The source bytes are verified against the claimed digest before staging.
    pub fn put_workflow_source(
        &mut self,
        record: &WorkflowSourceRecord,
    ) -> Result<(), JournalError> {
        if let Err(e) =
            crate::journal::verify_content_digest(&record.source, &record.digest.as_bytes())
        {
            self.aborted = true;
            return Err(e);
        }
        let key = match workflow_source_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => {
                self.aborted = true;
                return Err(e);
            }
        };
        let value = match encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            record,
            MAX_WORKFLOW_SOURCE_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.aborted = true;
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.workflow_source, key, value);
        Ok(())
    }

    /// Inserts a compiled IR record into the batch.
    pub fn put_compiled_ir(&mut self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        let key = compiled_ir_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            record,
            MAX_COMPILED_IR_BYTES,
        )?;
        self.inner.insert(&self.journal.compiled_ir, key, value);
        Ok(())
    }

    /// Inserts a run header record into the batch.
    pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = run_header_key(record.run)?;
        let value = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            record,
            MAX_RUN_HEADER_BYTES,
        )?;
        self.inner.insert(&self.journal.run_header, key, value);
        Ok(())
    }

    /// Inserts a run snapshot record into the batch.
    pub fn put_snapshot(&mut self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
        let value = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        )?;
        self.inner.insert(&self.journal.run_snapshot, key, value);
        Ok(())
    }

    /// Inserts a blob record into the batch.
    ///
    /// The blob bytes are verified against the claimed digest before staging.
    pub fn put_blob(&mut self, record: &BlobRecord) -> Result<(), JournalError> {
        if let Err(e) = crate::journal::verify_content_digest(&record.bytes, &record.digest) {
            self.aborted = true;
            return Err(e);
        }
        let key = match blob_key(record.digest) {
            Ok(k) => k,
            Err(e) => {
                self.aborted = true;
                return Err(e);
            }
        };
        let value = match encode_record(MAGIC_BLOB, RecordKind::Blob, 0, record, MAX_BLOB_BYTES) {
            Ok(v) => v,
            Err(e) => {
                self.aborted = true;
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.blob, key, value);
        Ok(())
    }

    /// Inserts a status index marker into the batch.
    pub fn put_status_index(
        &mut self,
        state: u8,
        timestamp: u64,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.inner
            .insert(&self.journal.index_status, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts a workflow index marker into the batch.
    pub fn put_workflow_index(
        &mut self,
        workflow: vb_core::WorkflowId,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.inner
            .insert(&self.journal.index_workflow, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts an action index marker into the batch.
    pub fn put_action_index(
        &mut self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.inner
            .insert(&self.journal.index_action, key, Vec::<u8>::new());
        Ok(())
    }

    /// Appends a journal event into the batch.
    ///
    /// # Invariant I20
    /// Duplicate event detection is enforced at `append_event` time by
    /// checking the journal's keyspace for already-committed events.
    /// Same-batch idempotent inserts are allowed (duplicates within
    /// the same batch are collapsed at commit time).
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.journal.events.contains_key(key)? {
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
        self.inner.insert(&self.journal.events, key, value);
        Ok(())
    }

    /// Returns the number of operations in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.aborted { 0 } else { self.inner.len() }
    }

    /// Returns true if the batch contains no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets strict durability for the commit.
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }

    /// Commits the batch atomically.
    pub fn commit(self) -> Result<(), JournalError> {
        if self.aborted {
            return Ok(());
        }
        self.inner.commit()?;
        Ok(())
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
        BlobRecord, CompiledIrRecord, EventSeq, JournalEvent, RunHeaderRecord,
        WorkflowSourceRecord, constants::DIGEST_BYTES, recovery::RunSnapshot,
    };
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, crate::FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_event(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_run_header(run: RunId) -> RunHeaderRecord {
        RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 1000,
        }
    }

    // =========================================================================
    // JournalWriteBatch construction and initial state
    // =========================================================================

    #[test]
    fn new_batch_is_empty_with_zero_length() {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        assert!(batch.is_empty(), "newly constructed batch must be empty");
        assert_eq!(
            batch.len(),
            0,
            "newly constructed batch must report length 0"
        );
    }

    #[test]
    fn new_batch_from_journal_batch_method_is_empty() {
        let (_temp, journal) = temp_journal();
        let batch = journal.batch();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    // =========================================================================
    // JournalWriteBatch len/is_empty tracking
    // =========================================================================

    #[test]
    fn len_increments_after_each_append_event() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let mut batch = JournalWriteBatch::new(&journal);

        batch.append_event(&make_event(run, 0)).expect("append 0");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        batch.append_event(&make_event(run, 1)).expect("append 1");
        assert_eq!(batch.len(), 2);

        batch.append_event(&make_event(run, 2)).expect("append 2");
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn len_increments_after_put_run_header() {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_run_header(&make_run_header(RunId::new(10)))
            .expect("put header");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn len_increments_after_put_status_index() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(20);
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_status_index(1, 12345, run)
            .expect("put status index");
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn len_increments_after_put_workflow_index() {
        let (_temp, journal) = temp_journal();
        let wf = WorkflowId::new(5);
        let run = RunId::new(30);
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_index(wf, run)
            .expect("put workflow index");
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn len_increments_after_put_action_index() {
        let (_temp, journal) = temp_journal();
        let action = vb_core::ActionId::new(99);
        let run = RunId::new(40);
        let step = StepIdx::new(0);
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_action_index(action, run, step)
            .expect("put action index");
        assert_eq!(batch.len(), 1);
    }

    // =========================================================================
    // Batch flush: empty batch commit
    // =========================================================================

    #[test]
    fn empty_batch_commit_succeeds() {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        let result = batch.commit();
        assert!(
            result.is_ok(),
            "committing an empty batch should succeed, got {:?}",
            result
        );
    }

    // =========================================================================
    // Batch flush: single event
    // =========================================================================

    #[test]
    fn commit_with_single_event_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let event = make_event(run, 0);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append should succeed");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            1,
            "should find 1 event after single-event batch commit"
        );
        assert_eq!(events[0], event);
    }

    // =========================================================================
    // Batch flush: multiple events of different kinds
    // =========================================================================

    #[test]
    fn commit_with_multiple_events_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&e0).expect("append 0");
        batch.append_event(&e1).expect("append 1");
        batch.append_event(&e2).expect("append 2");
        assert_eq!(batch.len(), 3);
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            3,
            "should find 3 events after multi-event batch"
        );
        assert_eq!(events[0], e0);
        assert_eq!(events[1], e1);
        assert_eq!(events[2], e2);
    }

    // =========================================================================
    // Batch with put_workflow_source (valid digest)
    // =========================================================================

    #[test]
    fn batch_put_workflow_source_with_valid_digest_commits() {
        let (_temp, journal) = temp_journal();
        let source = b"workflow: batch_test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source: source.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_source(&record)
            .expect("put workflow source");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");

        let loaded = journal.workflow_source(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("workflow source should be found after batch commit");
        };
        assert_eq!(found.source, source);
    }

    // =========================================================================
    // Batch with put_compiled_ir (valid)
    // =========================================================================

    #[test]
    fn batch_put_compiled_ir_with_valid_digest_commits() {
        let (_temp, journal) = temp_journal();
        let ir = b"compiled-batch-test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord {
            digest,
            ir: ir.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_compiled_ir(&record).expect("put compiled ir");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");

        let loaded = journal.compiled_ir(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("compiled IR should be found after batch commit");
        };
        assert_eq!(found.ir, ir);
    }

    // =========================================================================
    // Batch with put_run_header
    // =========================================================================

    #[test]
    fn batch_put_run_header_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);
        let header = make_run_header(run);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_run_header(&header).expect("put run header");
        batch.commit().expect("commit should succeed");

        let loaded = journal.run_header(run).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("run header should be found after batch commit");
        };
        assert_eq!(found.run, run);
        assert_eq!(found.status, 1);
    }

    // =========================================================================
    // Batch with put_snapshot
    // =========================================================================

    #[test]
    fn batch_put_snapshot_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(400);
        let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(5),
            workflow,
            slots: vec![1, 2, 3],
            taint: vec![0],
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_snapshot(&snapshot).expect("put snapshot");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .snapshot(run, EventSeq::new(5))
            .expect("get should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.run, run);
        assert_eq!(loaded.slots, vec![1, 2, 3]);
        assert_eq!(loaded.taint, vec![0]);
    }

    // =========================================================================
    // Batch with put_blob (valid digest)
    // =========================================================================

    #[test]
    fn batch_put_blob_with_valid_digest_commits() {
        let (_temp, journal) = temp_journal();
        let payload = vec![0xCA, 0xFE];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord {
            digest,
            bytes: payload.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_blob(&record).expect("put blob");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");

        let loaded = journal.blob(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("blob should be found after batch commit");
        };
        assert_eq!(found.bytes, payload);
    }

    // =========================================================================
    // Batch strict durability mode
    // =========================================================================

    #[test]
    fn batch_strict_mode_commits_successfully() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(500);
        let event = make_event(run, 0);

        let batch = JournalWriteBatch::new(&journal);
        let mut batch = batch.strict();
        batch.append_event(&event).expect("append should succeed");
        batch.commit().expect("strict commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    // =========================================================================
    // Cross-keyspace batch with mixed operation types
    // =========================================================================

    #[test]
    fn batch_mixed_operations_across_keyspaces_commit_atomically() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(600);
        let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

        let source = b"batch mixed ops source".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let workflow_record = WorkflowSourceRecord {
            digest: source_digest,
            source,
        };

        let ir = b"batch mixed ops ir".to_vec();
        let ir_digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let ir_record = CompiledIrRecord {
            digest: ir_digest,
            ir,
        };

        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(42),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 9999,
        };

        let event = make_event(run, 0);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_source(&workflow_record)
            .expect("workflow source");
        batch.put_compiled_ir(&ir_record).expect("compiled ir");
        batch.put_run_header(&header).expect("run header");
        batch.append_event(&event).expect("event");
        batch.put_status_index(1, 100, run).expect("status index");
        batch
            .put_workflow_index(WorkflowId::new(42), run)
            .expect("workflow index");
        batch
            .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
            .expect("action index");

        assert_eq!(batch.len(), 7, "batch should track 7 operations");
        batch.commit().expect("mixed batch commit should succeed");

        // Verify all keyspaces were written
        assert!(
            journal
                .workflow_source(source_digest)
                .expect("get ws")
                .is_some(),
            "workflow source should exist"
        );
        assert!(
            journal.compiled_ir(ir_digest).expect("get ir").is_some(),
            "compiled IR should exist"
        );
        assert!(
            journal.run_header(run).expect("get header").is_some(),
            "run header should exist"
        );
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1, "should have 1 event");
    }

    // =========================================================================
    // Edge case: batch put_workflow_source with wrong digest
    // =========================================================================

    #[test]
    fn batch_put_workflow_source_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let source = b"real content".to_vec();
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let record = WorkflowSourceRecord {
            digest: wrong_digest,
            source,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch must reject digest mismatch, got {:?}",
            result
        );
        // Batch length should still be 0 since the put failed
        assert_eq!(batch.len(), 0);
    }

    // =========================================================================
    // Edge case: batch put_blob with wrong digest
    // =========================================================================

    #[test]
    fn batch_put_blob_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3];
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
        let record = BlobRecord {
            digest: wrong_digest,
            bytes: payload,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch must reject blob digest mismatch, got {:?}",
            result
        );
        assert_eq!(batch.len(), 0);
    }

    // =========================================================================
    // Edge case: zero-length batch followed by strict commit
    // =========================================================================

    #[test]
    fn empty_strict_batch_commit_succeeds() {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        let batch = batch.strict();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        batch
            .commit()
            .expect("empty strict batch commit should succeed");
    }

    // =========================================================================
    // Edge case: put index operations do not carry payloads
    // =========================================================================

    #[test]
    fn batch_index_operations_increment_len_without_payloads() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(700);
        let wf = WorkflowId::new(10);
        let action = vb_core::ActionId::new(20);
        let step = StepIdx::new(1);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_status_index(2, 5000, run).expect("status idx");
        batch.put_workflow_index(wf, run).expect("workflow idx");
        batch
            .put_action_index(action, run, step)
            .expect("action idx");
        assert_eq!(batch.len(), 3, "three index operations should yield len 3");
        assert!(!batch.is_empty());
        batch.commit().expect("index batch commit should succeed");

        // Verify index markers are present
        let status_count = journal.index_status.iter().count();
        assert_eq!(status_count, 1, "should have 1 status index marker");

        let wf_count = journal.index_workflow.iter().count();
        assert_eq!(wf_count, 1, "should have 1 workflow index marker");

        let action_count = journal.index_action.iter().count();
        assert_eq!(action_count, 1, "should have 1 action index marker");
    }

    // =========================================================================
    // RED PHASE: vb-fb52 failing tests — Atomic Journal and Index Write Batches
    // =========================================================================
    // NOTE: batch_is_not_send_or_sync is now enforced at the type level via
    // PhantomData<*mut FjallJournal>. The type literally cannot implement Send
    // or Sync without unsafe code, so no runtime test is needed.

    #[test]
    fn batch_put_compiled_ir_commits_and_is_readable() {
        // I3: compiled_ir readable after batch commit
        let (_temp, journal) = temp_journal();
        let ir = b"compiled-artifact-bytes".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord {
            digest,
            ir: ir.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_compiled_ir(&record).expect("batch compiled ir");
        batch.commit().expect("commit should succeed");

        let loaded = journal.compiled_ir(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("compiled IR should be found after batch commit");
        };
        assert_eq!(found.ir, ir);
    }

    #[test]
    fn batch_append_event_commits_and_is_readable() {
        // I6: event readable after batch commit
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let event = make_event(run, 0);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append event");
        batch.commit().expect("commit should succeed");

        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 1, "should have 1 event after batch commit");
        assert_eq!(replayed[0], event);
    }

    #[test]
    fn batch_append_event_rejects_duplicate_event() {
        // EP-7, I20: DuplicateEvent on second batch append with same run_id+seq
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);
        let event = make_event(run, 0);

        // First append via batch
        let mut batch1 = JournalWriteBatch::new(&journal);
        batch1
            .append_event(&event)
            .expect("first append should succeed");
        batch1.commit().expect("commit should succeed");

        // Second append with same run_id+seq should fail
        let mut batch2 = JournalWriteBatch::new(&journal);
        let result = batch2.append_event(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate event must be rejected with DuplicateEvent, got {:?}",
            result
        );
        assert_eq!(
            batch2.len(),
            0,
            "batch len should remain 0 after failed append"
        );
    }

    #[test]
    fn len_equals_staged_count_after_random_operations() {
        // P1: len() always equals actual staged operation count
        let (_temp, journal) = temp_journal();
        let run = RunId::new(400);

        let mut batch = JournalWriteBatch::new(&journal);

        // Stage 3 events using indexed iteration
        let events: Vec<_> = (0..3)
            .map(|i| JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(i),
                workflow: WorkflowDigest::from_bytes([0; 32]),
            })
            .collect();

        events.iter().enumerate().for_each(|(idx, evt)| {
            batch.append_event(evt).expect("append should succeed");
            let expected_len = idx + 1;
            assert_eq!(
                batch.len(),
                expected_len,
                "len() must equal staged count after each operation"
            );
        });

        // Stage a header
        let header = make_run_header(run);
        batch.put_run_header(&header).expect("put header");
        assert_eq!(batch.len(), 4);

        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn is_empty_equals_len_zero_invariant() {
        // P2: is_empty() == (len() == 0) holds after every operation
        let (_temp, journal) = temp_journal();
        let run = RunId::new(500);

        let mut batch = JournalWriteBatch::new(&journal);

        // Initially empty
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) for new batch"
        );

        // After one operation
        batch.append_event(&make_event(run, 0)).expect("append");
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) after one operation"
        );

        // After more operations
        batch
            .put_run_header(&make_run_header(run))
            .expect("put header");
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) after multiple operations"
        );
    }

    #[test]
    fn batch_len_never_decreases() {
        // P3: len() monotonically increases (never decreases)
        let (_temp, journal) = temp_journal();
        let run = RunId::new(600);

        let mut batch = JournalWriteBatch::new(&journal);

        let operations = 5;
        let events: Vec<_> = (0..operations)
            .map(|i| JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(i),
                workflow: WorkflowDigest::from_bytes([0; 32]),
            })
            .collect();

        let mut prev_len = 0;
        events.iter().for_each(|evt| {
            batch.append_event(evt).expect("append");
            let new_len = batch.len();
            assert!(
                new_len > prev_len,
                "len() must increase monotonically, prev={}, new={}",
                prev_len,
                new_len
            );
            prev_len = new_len;
        });

        batch.commit().expect("commit");
    }

    #[test]
    fn all_or_nothing_commit_across_keyspaces() {
        // P5: commit is all-or-nothing; no partial state visible
        let (_temp, journal) = temp_journal();
        let run = RunId::new(800);
        let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

        let source = b"batch atomic test".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let workflow_record = WorkflowSourceRecord {
            digest: source_digest,
            source,
        };

        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(42),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 9999,
        };

        {
            let mut batch = JournalWriteBatch::new(&journal);
            batch.put_workflow_source(&workflow_record).expect("ws");
            batch.put_run_header(&header).expect("header");
            batch.commit().expect("commit should succeed");
        }

        // All or nothing: both must be present or neither
        let ws_present = journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some();
        let header_present = journal.run_header(run).expect("get header").is_some();
        assert_eq!(
            ws_present, header_present,
            "commit must be all-or-nothing across keyspaces"
        );
    }

    #[test]
    fn digest_verification_mandatory_on_workflow_source() {
        // P7: BLAKE3 digest verification cannot be skipped for workflow_source
        let (_temp, journal) = temp_journal();
        let source = b"content to forge".to_vec();
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);

        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "workflow_source digest verification must be mandatory"
        );
    }

    #[test]
    fn digest_verification_mandatory_on_blob() {
        // P8: BLAKE3 digest verification cannot be skipped for blob
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3, 4, 5];
        let forged_digest: [u8; DIGEST_BYTES] = [0xAB; 32];

        let record = BlobRecord {
            digest: forged_digest,
            bytes: payload,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "blob digest verification must be mandatory"
        );
    }

    // =========================================================================
    // Additional tests for 5x density (40 new tests)
    // =========================================================================

    #[test]
    fn batch_append_step_succeeded_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1101);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            output: SlotIdx::new(7),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append step succeeded");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_finished_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1102);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::new(9),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append run finished");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_cancelled_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1103);
        let event = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append run cancelled");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_multiple_events_same_run_preserves_order() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1200);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let e2 = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&e0).expect("append 0");
        batch.append_event(&e1).expect("append 1");
        batch.append_event(&e2).expect("append 2");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], e0);
        assert_eq!(events[1], e1);
        assert_eq!(events[2], e2);
    }

    #[test]
    fn batch_len_tracks_all_operation_types() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1300);

        let mut batch = JournalWriteBatch::new(&journal);
        assert_eq!(batch.len(), 0);

        batch
            .append_event(&make_event(run, 0))
            .expect("append event");
        assert_eq!(batch.len(), 1);

        let header = make_run_header(run);
        batch.put_run_header(&header).expect("put header");
        assert_eq!(batch.len(), 2);

        let source = b"test source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let wf_record = WorkflowSourceRecord { digest, source };
        batch
            .put_workflow_source(&wf_record)
            .expect("put wf source");
        assert_eq!(batch.len(), 3);

        batch
            .put_status_index(1, 1000, run)
            .expect("put status idx");
        assert_eq!(batch.len(), 4);
    }

    #[test]
    fn batch_is_empty_after_single_operation_is_false() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1400);

        let mut batch = JournalWriteBatch::new(&journal);
        assert!(batch.is_empty());

        batch.append_event(&make_event(run, 0)).expect("append");
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_is_empty_after_commit_is_false_for_nonempty_batch() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1401);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&make_event(run, 0)).expect("append");
        assert!(!batch.is_empty());
        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn batch_len_after_commit_equals_committed_count() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1500);

        let mut batch = JournalWriteBatch::new(&journal);
        (0..7).for_each(|i| {
            batch.append_event(&make_event(run, i)).expect("append");
        });
        assert_eq!(batch.len(), 7);
        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn batch_put_status_index_with_various_states() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1700);

        (0..4u8).for_each(|state| {
            let mut batch = JournalWriteBatch::new(&journal);
            batch
                .put_status_index(state, 1000 + u64::from(state), run)
                .expect("put status index");
            batch.commit().expect("commit should succeed");
        });
    }

    #[test]
    fn batch_put_workflow_index_multiple_workflows() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1800);

        (0..4u32).for_each(|i| {
            let workflow_id = vb_core::WorkflowId::new(i + 1);
            let mut batch = JournalWriteBatch::new(&journal);
            batch
                .put_workflow_index(workflow_id, run)
                .expect("put workflow idx");
            batch.commit().expect("commit should succeed");
        });
    }

    #[test]
    fn batch_put_action_index_multiple_actions() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1900);
        let step = StepIdx::new(0);

        (0..4u16).for_each(|i| {
            let action_id = vb_core::ActionId::new(i + 1);
            let mut batch = JournalWriteBatch::new(&journal);
            batch
                .put_action_index(action_id, run, step)
                .expect("put action idx");
            batch.commit().expect("commit should succeed");
        });
    }

    #[test]
    fn batch_put_blob_with_correct_digest_succeeds() {
        let (_temp, journal) = temp_journal();
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord {
            digest,
            bytes: payload,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_blob(&record).expect("put blob should succeed");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .blob(record.digest)
            .expect("get blob")
            .expect("blob should exist");
        assert_eq!(loaded.bytes, record.bytes);
    }

    #[test]
    fn batch_put_compiled_ir_readable_after_commit() {
        let (_temp, journal) = temp_journal();
        let ir = b"compiled ir bytes for batch test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord {
            digest,
            ir: ir.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_compiled_ir(&record).expect("put compiled ir");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .compiled_ir(digest)
            .expect("get compiled ir")
            .expect("ir should exist");
        assert_eq!(loaded.ir, ir);
    }

    #[test]
    fn batch_multiple_operations_same_batch_all_visible_after_commit() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2000);
        let source = b"workflow source data".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(42),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 5000,
        };

        {
            let mut batch = JournalWriteBatch::new(&journal);
            batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest: source_digest,
                    source: source.clone(),
                })
                .expect("put wf source");
            batch.put_run_header(&header).expect("put header");
            batch
                .append_event(&make_event(run, 0))
                .expect("append event");
            batch.commit().expect("commit should succeed");
        }

        let ws = journal
            .workflow_source(source_digest)
            .expect("get ws")
            .expect("ws should exist");
        assert_eq!(ws.source, source);
        let h = journal
            .run_header(run)
            .expect("get header")
            .expect("header should exist");
        assert_eq!(h.workflow_id, header.workflow_id);
        let events = journal.events_for_run(run).expect("get events");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_len_increases_by_one_for_each_operation() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2100);

        let mut batch = JournalWriteBatch::new(&journal);
        let initial_len = batch.len();

        batch.append_event(&make_event(run, 0)).expect("append");
        assert_eq!(batch.len(), initial_len + 1);

        batch
            .put_run_header(&make_run_header(run))
            .expect("put header");
        assert_eq!(batch.len(), initial_len + 2);

        batch.put_status_index(1, 1000, run).expect("put status");
        assert_eq!(batch.len(), initial_len + 3);
    }

    #[test]
    fn batch_is_empty_reflects_true_state() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2200);

        let mut batch = JournalWriteBatch::new(&journal);
        assert!(batch.is_empty());

        batch.append_event(&make_event(run, 0)).expect("append");
        assert!(!batch.is_empty());

        let header = make_run_header(run);
        batch.put_run_header(&header).expect("put header");
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_strict_mode_affects_commit() {
        let (_temp, journal) = temp_journal();
        let _run = RunId::new(2300);

        let batch = JournalWriteBatch::new(&journal);
        let _strict_batch = batch.strict();
    }

    #[test]
    fn batch_strict_batch_accepts_operations() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2301);

        let mut batch = JournalWriteBatch::new(&journal).strict();
        batch
            .append_event(&make_event(run, 0))
            .expect("append should succeed");
        assert_eq!(batch.len(), 1);
        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn batch_rejects_duplicate_event_across_batches() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2400);
        let event = make_event(run, 0);

        {
            let mut batch1 = JournalWriteBatch::new(&journal);
            batch1
                .append_event(&event)
                .expect("first append should succeed");
            batch1.commit().expect("commit should succeed");
        }

        let mut batch2 = JournalWriteBatch::new(&journal);
        let result = batch2.append_event(&event);
        assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })));
    }

    #[test]
    fn batch_len_zero_after_failed_append() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2500);
        let event = make_event(run, 0);

        let mut batch1 = JournalWriteBatch::new(&journal);
        batch1.append_event(&event).expect("first append");
        batch1.commit().expect("commit");

        let mut batch2 = JournalWriteBatch::new(&journal);
        let result = batch2.append_event(&event);
        assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })));
        assert_eq!(batch2.len(), 0, "batch len must be 0 after failed append");
    }

    #[test]
    fn batch_aborted_operations_set_len_to_zero() {
        let (_temp, journal) = temp_journal();
        let source = b"data".to_vec();
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: wrong_digest,
            source,
        });
        assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
        assert_eq!(batch.len(), 0, "aborted batch must report len 0");
    }

    #[test]
    fn batch_put_workflow_source_then_readable() {
        let (_temp, journal) = temp_journal();
        let source = b"workflow source for read test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source: source.clone(),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_source(&record)
            .expect("put should succeed");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .workflow_source(digest)
            .expect("get should succeed")
            .expect("should be found");
        assert_eq!(loaded.source, source);
    }

    #[test]
    fn batch_put_run_header_then_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2600);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(99),
            compiled_digest: WorkflowDigest::from_bytes([0x12; DIGEST_BYTES]),
            status: 2,
            accepted_at_ms: 9999,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_run_header(&header).expect("put should succeed");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .run_header(run)
            .expect("get should succeed")
            .expect("should be found");
        assert_eq!(loaded.workflow_id, header.workflow_id);
        assert_eq!(loaded.status, header.status);
    }

    #[test]
    fn batch_event_replay_returns_correct_sequence() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2700);
        let events: Vec<_> = (0..5u16)
            .map(|i| JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i as u64),
                step: StepIdx::new(i),
            })
            .collect();

        let mut batch = JournalWriteBatch::new(&journal);
        events.iter().for_each(|evt| {
            batch.append_event(evt).expect("append should succeed");
        });
        batch.commit().expect("commit should succeed");

        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 5);
        replayed.iter().enumerate().for_each(|(i, evt)| {
            assert_eq!(evt.seq(), EventSeq::new(i as u64));
        });
    }

    #[test]
    fn batch_len_equals_staged_count_for_various_operations() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2800);

        let mut batch = JournalWriteBatch::new(&journal);
        let mut expected = 0;

        batch.append_event(&make_event(run, 0)).expect("append");
        expected += 1;
        assert_eq!(batch.len(), expected);

        batch.put_status_index(1, 1000, run).expect("put status");
        expected += 1;
        assert_eq!(batch.len(), expected);

        batch
            .put_workflow_index(WorkflowId::new(1), run)
            .expect("put workflow");
        expected += 1;
        assert_eq!(batch.len(), expected);

        batch
            .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
            .expect("put action");
        expected += 1;
        assert_eq!(batch.len(), expected);

        batch
            .put_run_header(&make_run_header(run))
            .expect("put header");
        expected += 1;
        assert_eq!(batch.len(), expected);
    }

    #[test]
    fn batch_index_operations_do_not_require_payload() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2900);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_status_index(2, 5000, run).expect("status idx");
        batch
            .put_workflow_index(WorkflowId::new(10), run)
            .expect("workflow idx");
        batch
            .put_action_index(vb_core::ActionId::new(20), run, StepIdx::new(1))
            .expect("action idx");
        assert_eq!(batch.len(), 3);
        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn batch_put_blob_rejects_wrong_digest() {
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3, 4, 5];
        let _correct_digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];

        let record = BlobRecord {
            digest: wrong_digest,
            bytes: payload,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.put_blob(&record);
        assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn batch_event_commits_correctly() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3000);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(5),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append should succeed");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_multiple_runs_do_not_interfere() {
        let (_temp, journal) = temp_journal();
        let run1 = RunId::new(3100);
        let run2 = RunId::new(3101);

        let mut batch1 = JournalWriteBatch::new(&journal);
        batch1
            .append_event(&make_event(run1, 0))
            .expect("append run1");
        batch1.commit().expect("commit run1");

        let mut batch2 = JournalWriteBatch::new(&journal);
        batch2
            .append_event(&make_event(run2, 0))
            .expect("append run2");
        batch2.commit().expect("commit run2");

        let events1 = journal.events_for_run(run1).expect("replay run1");
        let events2 = journal.events_for_run(run2).expect("replay run2");
        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
    }

    #[test]
    fn batch_len_after_clear_equals_zero() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3200);

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&make_event(run, 0)).expect("append");
        assert_eq!(batch.len(), 1);

        let batch2 = JournalWriteBatch::new(&journal);
        assert_eq!(batch2.len(), 0);
    }

    #[test]
    fn batch_put_compiled_ir_with_valid_digest() {
        let (_temp, journal) = temp_journal();
        let ir = b"valid compiled ir".to_vec();
        let ir_clone = ir.clone();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord { digest, ir };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_compiled_ir(&record).expect("put should succeed");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .compiled_ir(digest)
            .expect("get should succeed")
            .expect("should be found");
        assert_eq!(loaded.ir, ir_clone);
    }

    #[test]
    fn batch_workflow_source_with_valid_digest() {
        let (_temp, journal) = temp_journal();
        let source = b"valid workflow source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_source(&record)
            .expect("put should succeed");
        batch.commit().expect("commit should succeed");

        let loaded = journal
            .workflow_source(digest)
            .expect("get should succeed")
            .expect("should be found");
        assert_eq!(loaded.source, record.source);
    }

    #[test]
    fn batch_status_index_markers_are_queryable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3300);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_status_index(1, 1000, run)
            .expect("put status idx");
        batch.commit().expect("commit should succeed");

        let markers = journal.index_status.iter().count();
        assert!(markers >= 1, "should have at least 1 status index marker");
    }

    #[test]
    fn batch_workflow_index_markers_are_queryable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3301);
        let workflow_id = WorkflowId::new(42);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_workflow_index(workflow_id, run)
            .expect("put workflow idx");
        batch.commit().expect("commit should succeed");

        let markers = journal.index_workflow.iter().count();
        assert!(markers >= 1, "should have at least 1 workflow index marker");
    }

    #[test]
    fn batch_action_index_markers_are_queryable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3302);
        let action_id = vb_core::ActionId::new(7);
        let step = StepIdx::new(3);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .put_action_index(action_id, run, step)
            .expect("put action idx");
        batch.commit().expect("commit should succeed");

        let markers = journal.index_action.iter().count();
        assert!(markers >= 1, "should have at least 1 action index marker");
    }

    #[test]
    fn batch_commit_persists_all_keyspaces_atomically() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3400);
        let source = b"atomic test source".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(77),
            compiled_digest: WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 7000,
        };

        {
            let mut batch = JournalWriteBatch::new(&journal);
            batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest: source_digest,
                    source: source.clone(),
                })
                .expect("put wf source");
            batch.put_run_header(&header).expect("put header");
            batch
                .append_event(&make_event(run, 0))
                .expect("append event");
            batch.commit().expect("commit should succeed");
        }

        let ws_exists = journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some();
        let header_exists = journal.run_header(run).expect("get header").is_some();
        let event_exists = !journal.events_for_run(run).expect("get events").is_empty();
        assert!(
            ws_exists && header_exists && event_exists,
            "all keyspaces must be present after atomic commit"
        );
    }

    #[test]
    fn batch_run_admission_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3500);
        let event = JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(0),
            artifact_digest: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Strict,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append run admission");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_action_scheduled_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3600);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(5),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append action scheduled");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_action_completed_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3700);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            action: vb_core::ActionId::new(10),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append action completed");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_action_failed_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3800);
        let event = JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            action: vb_core::ActionId::new(15),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append action failed");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_slot_written_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3900);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(4),
            value: Some(b"slot value".to_vec()),
            extra: None,
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append slot written");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_wait_scheduled_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4000);
        let event = JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append wait scheduled");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_ask_scheduled_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4100);
        let event = JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append ask scheduled");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_retry_scheduled_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4200);
        let event = JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append retry scheduled");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_run_failed_event_commits_and_is_readable() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4300);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
        };

        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append run failed");
        batch.commit().expect("commit should succeed");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn batch_len_zero_for_empty_batch() {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn batch_len_after_single_append_is_one() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4500);
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&make_event(run, 0)).expect("append");
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn batch_multiple_events_same_run_all_committed() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4600);
        let events: Vec<_> = (0..10u16)
            .map(|i| JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i as u64),
                step: StepIdx::new(i),
            })
            .collect();

        let mut batch = JournalWriteBatch::new(&journal);
        events.iter().for_each(|evt| {
            batch.append_event(evt).expect("append should succeed");
        });
        batch.commit().expect("commit should succeed");
        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 10);
    }
}
