use super::{AcceptedArtifactStore, ArtifactEnvelopeError};
use vb_core::{CompiledWorkflow, RuntimePolicy};
use vb_storage::AcceptedArtifact;

pub(crate) fn verify_stored_artifact(
    store: &dyn AcceptedArtifactStore, artifact: &AcceptedArtifact, policy: Option<RuntimePolicy>,
) -> Result<CompiledWorkflow, ArtifactEnvelopeError> {
    if let vb_core::artifact::ArtifactEvidence::Packed(packed) = &artifact.verification.evidence {
        let catalog = store.executor_catalog().ok_or(ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
        return catalog.verify_artifact(artifact, policy.map_or(packed.base.admission_policy, |policy| policy))
            .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid);
    }
    match policy {
        Some(policy) => vb_storage::admission::verify_accepted_artifact_for_policy(artifact, policy),
        None => vb_storage::admission::verify_accepted_artifact(artifact),
    }.map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)
}
