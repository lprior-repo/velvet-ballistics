#![forbid(unsafe_code)]
use crate::codec::encode_record;
use crate::constants::{
    MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_SNAPSHOT,
    MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES, MAX_RUN_HEADER_BYTES,
    MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
};
use crate::error::JournalError;
use crate::keys::{
    blob_key, compiled_ir_key, index_action_key, index_status_key, index_workflow_key,
    run_header_key, run_snapshot_key, workflow_source_key,
};
use crate::records::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord};
use crate::recovery::RunSnapshot;
use vb_core::{ActionId, StepIdx, WorkflowId};
use super::types::JournalWriteBatch;
impl<'j> JournalWriteBatch<'j> {
    pub fn put_workflow_source(&mut self, record: &WorkflowSourceRecord) -> Result<(), JournalError> {
        if let Err(e) = crate::journal::verify_content_digest(&record.source, &record.digest.as_bytes()) {
            self.aborted = true;
            return Err(e);
        }
        let key = match workflow_source_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        let value = match encode_record(
            MAGIC_WORKFLOW_SOURCE, crate::records::RecordKind::WorkflowSource, 0, record, MAX_WORKFLOW_SOURCE_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        self.inner.insert(&self.journal.workflow_source, key, value);
        Ok(())
    }
    pub fn put_compiled_ir(&mut self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        let key = match compiled_ir_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        let value = match encode_record(
            MAGIC_COMPILED_ARTIFACT, crate::records::RecordKind::CompiledIr, 0, record, MAX_COMPILED_IR_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        self.inner.insert(&self.journal.compiled_ir, key, value);
        Ok(())
    }
    pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = match run_header_key(record.run) {
            Ok(k) => k,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        let value = match encode_record(
            MAGIC_INDEX_RECORD, crate::records::RecordKind::RunHeader, record.run.get(), record, MAX_RUN_HEADER_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        self.inner.insert(&self.journal.run_header, key, value);
        Ok(())
    }
    pub fn put_snapshot(&mut self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = match run_snapshot_key(snapshot.run, snapshot.seq) {
            Ok(k) => k,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        let value = match encode_record(
            MAGIC_SNAPSHOT, crate::records::RecordKind::Snapshot, snapshot.seq.get(), snapshot, MAX_SNAPSHOT_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        self.inner.insert(&self.journal.run_snapshot, key, value);
        Ok(())
    }
    pub fn put_blob(&mut self, record: &BlobRecord) -> Result<(), JournalError> {
        if let Err(e) = crate::journal::verify_content_digest(&record.bytes, &record.digest) {
            self.aborted = true;
            return Err(e);
        }
        let key = match blob_key(record.digest) {
            Ok(k) => k,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        let value = match encode_record(
            MAGIC_BLOB, crate::records::RecordKind::Blob, 0, record, MAX_BLOB_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => { self.aborted = true; return Err(e); }
        };
        self.inner.insert(&self.journal.blob, key, value);
        Ok(())
    }
    pub fn put_status_index(&mut self, state: crate::types::IndexStatusState, timestamp: u64, run: vb_core::RunId) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.inner.insert(&self.journal.index_status, key, Vec::<u8>::new());
        Ok(())
    }
    pub fn put_workflow_index(&mut self, workflow: WorkflowId, run: vb_core::RunId) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.inner.insert(&self.journal.index_workflow, key, Vec::<u8>::new());
        Ok(())
    }
    pub fn put_action_index(&mut self, action: ActionId, run: vb_core::RunId, step: StepIdx) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.inner.insert(&self.journal.index_action, key, Vec::<u8>::new());
        Ok(())
    }
}
