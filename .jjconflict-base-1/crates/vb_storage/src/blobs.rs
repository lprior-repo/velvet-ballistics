#![forbid(unsafe_code)]
//! Blob storage operations.
//!
//! Provides storage and retrieval of large binary blobs.

use crate::{
    codec::encode_record,
    constants::{MAGIC_BLOB, MAX_BLOB_BYTES},
    error::JournalError,
    keys::blob_key,
    records::BlobRecord,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Stores a bounded blob by digest.
    ///
    /// The blob bytes are verified against the claimed digest before storage.
    pub fn put_blob(&self, record: &BlobRecord) -> Result<(), JournalError> {
        crate::journal::verify_content_digest(&record.bytes, &record.digest)?;
        let key = blob_key(record.digest)?;
        let value = encode_record(
            MAGIC_BLOB,
            crate::records::RecordKind::Blob,
            0,
            record,
            MAX_BLOB_BYTES,
        )?;
        self.blob.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads a bounded blob by digest.
    pub fn blob(
        &self,
        digest: [u8; crate::constants::DIGEST_BYTES],
    ) -> Result<Option<BlobRecord>, JournalError> {
        let key = blob_key(digest)?;
        self.decode_optional_with(
            &self.blob,
            key.as_slice(),
            MAGIC_BLOB,
            MAX_BLOB_BYTES,
            |_envelope, record| validate_blob_read(digest, record),
        )
    }

    /// Removes a blob by digest, enforcing retention boundary semantics.
    ///
    /// Returns `Ok(())` if the blob was successfully trimmed.
    /// Returns `Err(ArtifactNotFound)` if the blob does not exist.
    pub fn trim_blob(
        &self,
        digest: [u8; crate::constants::DIGEST_BYTES],
    ) -> Result<(), JournalError> {
        let key = blob_key(digest)?;
        let exists = self.blob.contains_key(key.as_slice())?;
        if !exists {
            return Err(JournalError::ArtifactNotFound {
                digest: vb_core::WorkflowDigest::from_bytes(digest),
            });
        }
        self.blob.remove(key.as_slice())?;
        Ok(())
    }
}

fn validate_blob_read(
    requested_digest: [u8; crate::constants::DIGEST_BYTES],
    record: &BlobRecord,
) -> Result<(), JournalError> {
    if record.digest != requested_digest {
        return Err(JournalError::PayloadDigestMismatch);
    }
    crate::journal::verify_content_digest(record.bytes.as_slice(), &requested_digest)
}
