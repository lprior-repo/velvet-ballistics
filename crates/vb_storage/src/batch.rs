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
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BATCH_COUNT,
        MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
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
    // DEAD_FIELD: confirmed unused — initialized but never read or modified after construction
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
        state: crate::types::IndexStatusState,
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
            self.aborted = true;
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        if self.inner.len() >= MAX_BATCH_COUNT {
            return Err(JournalError::QueueFull);
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

mod tests;
