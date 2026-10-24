use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_storage::{AcceptedArtifact, FjallJournal, JournalEvent};

pub(super) fn load(
    shard: &Shard,
    journal: &FjallJournal,
    admission: &JournalEvent,
) -> RuntimeResult<AcceptedArtifact> {
    let JournalEvent::RunAdmission {
        artifact_digest, ..
    } = admission
    else {
        return Err(super::identity::mismatch());
    };
    let record =
        journal
            .compiled_ir(*artifact_digest)?
            .ok_or(RuntimeError::AdmissionArtifactNotFound {
                digest: *artifact_digest,
            })?;
    let (artifact, rest): (AcceptedArtifact, _) = postcard::take_from_bytes(&record.ir)
        .map_err(vb_storage::JournalError::PostcardDecodeFailed)?;
    if !rest.is_empty() || artifact.digest != *artifact_digest {
        return Err(super::identity::mismatch());
    }
    let independent = shard
        .artifact_store
        .load_accepted_artifact(*artifact_digest)
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
            digest: *artifact_digest,
        })?;
    if independent != artifact {
        return Err(super::identity::mismatch());
    }
    Ok(artifact)
}
