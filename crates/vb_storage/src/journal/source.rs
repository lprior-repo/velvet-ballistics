use crate::{
    admission::{
        compute_artifact_metadata_hash, decode_accepted_artifact_envelope,
        validate_compiled_ir_record,
    },
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
    ///
    /// SECURITY: This is pub(crate) to restrict access to admission path only.
    /// External callers MUST use `submit_artifact` or `admit_compiled_artifact`
    /// which properly bind all artifact metadata (warnings, capabilities, seq).
    ///
    /// # Metadata Hash Protection
    ///
    /// To prevent same-digest metadata mutation attacks, this function validates
    /// that any subsequent write to an existing digest has the same metadata hash
    /// as the existing record. The metadata hash is computed from artifact fields
    /// that should not change after admission: `source_digest`, `policy_digest`,
    /// inner `ir`, `verification` flags, `accepted_at_seq`, and
    /// `required_capabilities`.
    pub(crate) fn put_compiled_ir(&self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        validate_compiled_ir_record(record)?;
        let h_pending = Self::compute_pending_metadata_hash(record)?;
        let key = compiled_ir_key(record.digest.as_bytes())?;
        let existing = self.load_existing_compiled_ir(&key)?;
        self.validate_metadata_hash_is_consistent(record.digest, h_pending, existing.as_ref())?;
        let record_to_store = self.build_stored_record(record, h_pending);
        self.insert_compiled_ir_record(&key, &record_to_store)
    }

    /// Computes the metadata hash for a pending record.
    fn compute_pending_metadata_hash(record: &CompiledIrRecord) -> Result<[u8; 32], JournalError> {
        let artifact = decode_accepted_artifact_envelope(&record.ir)?;
        Ok(compute_artifact_metadata_hash(&artifact))
    }

    /// Loads the existing compiled IR record, if any.
    fn load_existing_compiled_ir(
        &self,
        key: &[u8],
    ) -> Result<Option<crate::records::CompiledIrRecord>, JournalError> {
        self.decode_optional(
            &self.compiled_ir,
            key,
            MAGIC_COMPILED_ARTIFACT,
            MAX_COMPILED_IR_BYTES,
        )
    }

    /// Validates that the pending metadata hash is consistent with any existing record.
    fn validate_metadata_hash_is_consistent(
        &self,
        digest: vb_core::WorkflowDigest,
        h_pending: [u8; 32],
        existing: Option<&crate::records::CompiledIrRecord>,
    ) -> Result<(), JournalError> {
        let Some(existing_record) = existing else {
            return Ok(());
        };
        match existing_record.metadata_hash {
            Some(h_existing) => {
                if h_pending != h_existing {
                    return Err(JournalError::MetadataMutation { digest });
                }
            }
            None => {
                let existing_artifact = decode_accepted_artifact_envelope(&existing_record.ir)?;
                let h_existing = compute_artifact_metadata_hash(&existing_artifact);
                if h_pending != h_existing {
                    return Err(JournalError::MetadataMutation { digest });
                }
            }
        }
        Ok(())
    }

    /// Builds the record to store, attaching the computed metadata hash.
    fn build_stored_record(&self, record: &CompiledIrRecord, h: [u8; 32]) -> CompiledIrRecord {
        let mut r = record.clone();
        r.metadata_hash = Some(h);
        r
    }

    /// Encodes and inserts the compiled IR record.
    fn insert_compiled_ir_record(
        &self,
        key: &[u8],
        record: &CompiledIrRecord,
    ) -> Result<(), JournalError> {
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
        match self.decode_optional(
            &self.compiled_ir,
            key.as_slice(),
            MAGIC_COMPILED_ARTIFACT,
            MAX_COMPILED_IR_BYTES,
        )? {
            Some(record) => {
                validate_compiled_ir_record(&record)?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }
}
