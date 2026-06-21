//! Record staging methods for [`super::JournalWriteBatch`].

use super::{BatchState, JournalWriteBatch};
use crate::{
    codec::encode_record,
    constants::{
        MAGIC_BLOB, MAGIC_INDEX_RECORD, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
        MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    keys::{blob_key, run_header_key, run_snapshot_key, workflow_source_key},
    records::{BlobRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord},
    recovery::RunSnapshot,
};

impl<'j> JournalWriteBatch<'j> {
    /// Inserts a workflow source record into the batch.
    pub fn put_workflow_source(
        &mut self,
        record: &WorkflowSourceRecord,
    ) -> Result<(), JournalError> {
        if let Err(e) =
            crate::journal::verify_content_digest(&record.source, &record.digest.as_bytes())
        {
            self.state = BatchState::Aborted {
                reason: "workflow_source_digest_mismatch",
            };
            return Err(e);
        }
        let key = match workflow_source_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted {
                    reason: "workflow_source_key_failed",
                };
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
                self.state = BatchState::Aborted {
                    reason: "workflow_source_encode_failed",
                };
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.workflow_source, key, value);
        Ok(())
    }

    /// Inserts a run header record into the batch.
    pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = match run_header_key(record.run) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted { reason: "batch_aborted" };
                return Err(e);
            }
        };
        let value = match encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            record,
            MAX_RUN_HEADER_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted { reason: "batch_aborted" };
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.run_header, key, value);
        Ok(())
    }

    /// Inserts a run snapshot record into the batch.
    pub fn put_snapshot(&mut self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = match run_snapshot_key(snapshot.run, snapshot.seq) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted { reason: "batch_aborted" };
                return Err(e);
            }
        };
        let value = match encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted { reason: "batch_aborted" };
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.run_snapshot, key, value);
        Ok(())
    }

    /// Inserts a blob record into the batch.
    pub fn put_blob(&mut self, record: &BlobRecord) -> Result<(), JournalError> {
        if let Err(e) = crate::journal::verify_content_digest(&record.bytes, &record.digest) {
            self.state = BatchState::Aborted {
                reason: "blob_digest_mismatch",
            };
            return Err(e);
        }
        let key = match blob_key(record.digest) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted {
                    reason: "blob_key_failed",
                };
                return Err(e);
            }
        };
        let value = match encode_record(MAGIC_BLOB, RecordKind::Blob, 0, record, MAX_BLOB_BYTES) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted {
                    reason: "blob_encode_failed",
                };
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.blob, key, value);
        Ok(())
    }
}
