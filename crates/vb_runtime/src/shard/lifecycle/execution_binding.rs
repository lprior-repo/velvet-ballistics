//! Bind the exact execution tuple to the independently checked accepted artifact.
use crate::admission::{AcceptedArtifactStore, ArtifactEnvelopeError};
use crate::{RuntimeError, RuntimeResult};
use vb_core::{ActionContract, CompiledWorkflow, RuntimePolicy};

pub(super) fn validate(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> RuntimeResult<()> {
    match policy {
        RuntimePolicy::Relaxed => return Ok(()),
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {}
        _ => {
            return Err(RuntimeError::AdmissionArtifactInvalid {
                digest: workflow.digest(),
            });
        }
    }
    let digest = workflow.digest();
    let artifact = store
        .load_accepted_artifact(digest)
        .map_err(|error| match error {
            ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                RuntimeError::AdmissionArtifactNotFound { digest }
            }
            ArtifactEnvelopeError::ArtifactDigestMismatch { requested, found } => {
                RuntimeError::AdmissionArtifactDigestMismatch { requested, found }
            }
            _ => RuntimeError::AdmissionArtifactInvalid { digest },
        })?;
    vb_storage::admission::verify_accepted_artifact_for_policy(&artifact, policy)
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid { digest })?;
    if artifact.source_digest != digest {
        return Err(RuntimeError::AdmissionArtifactDigestMismatch {
            requested: digest,
            found: artifact.source_digest,
        });
    }
    let payload = postcard::to_allocvec(&workflow.to_parts())
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid { digest })?;
    if payload != artifact.ir || contracts != artifact.action_contracts.as_ref() {
        return Err(RuntimeError::AdmissionArtifactInvalid { digest });
    }
    Ok(())
}
