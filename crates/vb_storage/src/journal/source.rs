use crate::{
    codec::encode_record,
    constants::{
        MAGIC_COMPILED_ARTIFACT, MAGIC_WORKFLOW_SOURCE, MAX_COMPILED_IR_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    journal::FjallJournal,
    journal::admission::verify_content_digest,
    keys::{compiled_ir_key, workflow_source_key},
    records::{CompiledIrRecord, RecordKind, WorkflowSourceRecord},
};

impl FjallJournal {
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
}
