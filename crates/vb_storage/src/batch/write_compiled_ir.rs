//! Test-only compiled-IR staging for [`super::JournalWriteBatch`].

use super::{BatchState, JournalWriteBatch};
use crate::{codec::encode_record, error::JournalError, records::RecordKind};

impl<'j> JournalWriteBatch<'j> {
    /// Inserts a compiled IR record into the batch for storage-boundary tests.
    #[cfg(test)]
    pub(crate) fn put_compiled_ir(
        &mut self,
        record: &crate::records::CompiledIrRecord,
    ) -> Result<(), JournalError> {
        if let Err(e) = crate::admission::validate_compiled_ir_record(record) {
            self.state = BatchState::Aborted;
            return Err(e);
        }

        let artifact = match crate::admission::decode_accepted_artifact_envelope(&record.ir) {
            Ok(a) => a,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        let h_pending = crate::admission::compute_artifact_metadata_hash(&artifact);

        if let Some(&h_staged) = self.staged_ir_hashes.get(&record.digest) {
            if h_pending != h_staged {
                self.state = BatchState::Aborted;
                return Err(JournalError::MetadataMutation {
                    digest: record.digest,
                });
            }
        }

        let key = match crate::keys::compiled_ir_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        if let Ok(Some(existing)) = self.journal.compiled_ir(record.digest) {
            self.reject_metadata_mismatch(
                record.digest,
                h_pending,
                &existing.ir,
                existing.metadata_hash,
            )?;
        }

        self.staged_ir_hashes.insert(record.digest, h_pending);
        let mut record_with_hash = record.clone();
        record_with_hash.metadata_hash = Some(h_pending);
        let value = match encode_record(
            crate::constants::MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record_with_hash,
            crate::constants::MAX_COMPILED_IR_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };

        self.inner.insert(&self.journal.compiled_ir, key, value);
        Ok(())
    }

    #[cfg(test)]
    fn reject_metadata_mismatch(
        &mut self,
        digest: vb_core::WorkflowDigest,
        h_pending: [u8; 32],
        existing_ir: &[u8],
        existing_hash: Option<[u8; 32]>,
    ) -> Result<(), JournalError> {
        let h_existing = match existing_hash {
            Some(hash) => hash,
            None => {
                let existing_artifact =
                    crate::admission::decode_accepted_artifact_envelope(existing_ir)?;
                crate::admission::compute_artifact_metadata_hash(&existing_artifact)
            }
        };
        if h_pending == h_existing {
            Ok(())
        } else {
            self.state = BatchState::Aborted;
            Err(JournalError::MetadataMutation { digest })
        }
    }
}
