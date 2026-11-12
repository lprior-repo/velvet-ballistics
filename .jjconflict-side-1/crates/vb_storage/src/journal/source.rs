use crate::{
    codec::{decode_record, encode_record},
    constants::{
        DIGEST_KEY_BYTES, MAGIC_COMPILED_ARTIFACT, MAGIC_WORKFLOW_SOURCE, MAX_COMPILED_IR_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES, PREFIX_COMPILED_IR,
    },
    error::JournalError,
    journal::FjallJournal,
    journal::admission::{verify_compiled_ir_record_digest, verify_content_digest},
    keys::{compiled_ir_key, decode_storage_key, workflow_source_key},
    records::{CompiledIrRecord, RecordKind, WorkflowSourceRecord},
    types::StorageKey,
};

const MAX_COMPILED_IR_SOURCE_DIGEST_SCAN_RECORDS: usize = 65_536;

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
        self.decode_optional_with(
            &self.workflow_source,
            key.as_slice(),
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
            |_envelope, record| validate_workflow_source_read(digest, record),
        )
    }

    /// Stores compiled IR bytes by digest.
    ///
    /// The IR bytes are verified against the claimed digest before storage so
    /// a forged `CompiledIrRecord { digest, ir }` cannot be persisted under
    /// the digest key (master §18 invariant 8: digest↔content binding).
    pub fn put_compiled_ir(&self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        verify_compiled_ir_record_digest(record)?;
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
        // SA-009: test-only hook forces the readback to report the artifact
        // as missing even when the underlying LSM still holds it.
        #[cfg(test)]
        if self.consume_compiled_ir_readback_failure_for_test() {
            return Ok(None);
        }
        let key = compiled_ir_key(digest.as_bytes())?;
        self.decode_optional_with(
            &self.compiled_ir,
            key.as_slice(),
            MAGIC_COMPILED_ARTIFACT,
            MAX_COMPILED_IR_BYTES,
            |_envelope, record| validate_compiled_ir_read(digest, record),
        )
    }

    /// Loads an accepted-artifact compiled IR record by the workflow/source digest.
    ///
    /// Compiled IR records are content-addressed by their artifact digest. Runtime
    /// events and run headers carry the workflow digest, so operator/IPC cold paths
    /// need a bounded fallback that resolves an accepted artifact whose
    /// `source_digest` matches that workflow digest without weakening the primary
    /// digest↔payload binding. Direct raw `WorkflowParts` records are ignored by
    /// this method because they do not carry a source-digest envelope.
    pub fn compiled_ir_for_source_digest(
        &self,
        source_digest: vb_core::WorkflowDigest,
    ) -> Result<Option<CompiledIrRecord>, JournalError> {
        let mut scanned = 0usize;
        for guard in self.compiled_ir.iter() {
            if scanned >= MAX_COMPILED_IR_SOURCE_DIGEST_SCAN_RECORDS {
                return Ok(None);
            }
            scanned = scanned.saturating_add(1);

            let (key, value) = guard.into_inner()?;
            let key_digest = compiled_ir_digest_from_key(key.as_ref())?;
            let (_, record) = decode_record::<CompiledIrRecord>(
                value.as_ref(),
                MAGIC_COMPILED_ARTIFACT,
                MAX_COMPILED_IR_BYTES,
            )?;
            validate_compiled_ir_read(key_digest, &record)?;
            let artifact = match postcard::from_bytes::<crate::admission::AcceptedArtifact>(
                record.ir.as_slice(),
            ) {
                Ok(artifact) => artifact,
                Err(_) => continue,
            };
            if artifact.source_digest == source_digest {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }
}

fn validate_workflow_source_read(
    requested_digest: vb_core::WorkflowDigest,
    record: &WorkflowSourceRecord,
) -> Result<(), JournalError> {
    let expected = requested_digest.as_bytes();
    if record.digest.as_bytes() != expected {
        return Err(JournalError::PayloadDigestMismatch);
    }
    verify_content_digest(record.source.as_slice(), &expected)
}

fn validate_compiled_ir_read(
    requested_digest: vb_core::WorkflowDigest,
    record: &CompiledIrRecord,
) -> Result<(), JournalError> {
    if record.digest.as_bytes() != requested_digest.as_bytes() {
        return Err(JournalError::PayloadDigestMismatch);
    }
    verify_compiled_ir_record_digest(record)
}

fn compiled_ir_digest_from_key(key: &[u8]) -> Result<vb_core::WorkflowDigest, JournalError> {
    match decode_storage_key(key) {
        Ok(StorageKey::CompiledIr { digest }) => Ok(vb_core::WorkflowDigest::from_bytes(digest)),
        Ok(_) | Err(_) => Err(JournalError::MalformedKeyspaceRow {
            prefix: PREFIX_COMPILED_IR,
            expected_len: DIGEST_KEY_BYTES,
            actual_len: key.len(),
        }),
    }
}
