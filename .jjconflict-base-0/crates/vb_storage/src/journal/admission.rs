use crate::{
    admission::{AcceptedArtifact, accepted_artifact_digest},
    error::JournalError,
    records::CompiledIrRecord,
};

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

pub(crate) fn verify_compiled_ir_record_digest(
    record: &CompiledIrRecord,
) -> Result<(), JournalError> {
    if verify_content_digest(&record.ir, &record.digest.as_bytes()).is_ok() {
        return Ok(());
    }

    let artifact = postcard::from_bytes::<AcceptedArtifact>(&record.ir)
        .map_err(|_| JournalError::PayloadDigestMismatch)?;
    let digest = accepted_artifact_digest(&artifact)?;
    if digest == record.digest
        && artifact.digest == record.digest
        && artifact.verification.digest == record.digest
    {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}
