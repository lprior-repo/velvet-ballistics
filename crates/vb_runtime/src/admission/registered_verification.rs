use super::{AcceptedArtifactStore, ArtifactEnvelopeError};
use vb_core::{CompiledWorkflow, RuntimePolicy};
use vb_storage::AcceptedArtifact;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StoredArtifactVerificationError {
    #[error(transparent)]
    Storage(#[from] vb_storage::JournalError),
    #[error(transparent)]
    Executor(#[from] vb_storage::executor::ExecutorError),
}

pub(crate) fn verify_stored_artifact(
    store: &dyn AcceptedArtifactStore,
    artifact: &AcceptedArtifact,
    policy: Option<RuntimePolicy>,
) -> Result<CompiledWorkflow, ArtifactEnvelopeError> {
    verify_stored_artifact_detailed(store, artifact, policy)
        .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)
}

pub(crate) fn verify_stored_artifact_detailed(
    store: &dyn AcceptedArtifactStore,
    artifact: &AcceptedArtifact,
    policy: Option<RuntimePolicy>,
) -> Result<CompiledWorkflow, StoredArtifactVerificationError> {
    if let vb_core::artifact::ArtifactEvidence::Packed(packed) = &artifact.verification.evidence {
        let catalog = store
            .executor_catalog()
            .ok_or(vb_storage::executor::ExecutorError::CatalogRequired)?;
        return catalog
            .verify_artifact(
                artifact,
                policy.map_or(packed.base.admission_policy, |policy| policy),
            )
            .map_err(Into::into);
    }
    match policy {
        Some(policy) => {
            vb_storage::admission::verify_accepted_artifact_for_policy(artifact, policy)
        }
        None => vb_storage::admission::verify_accepted_artifact(artifact),
    }
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admission::MissingAcceptedArtifactStore;
    use vb_core::action_pack::PackedArtifactEvidence;
    use vb_core::artifact::ArtifactEvidence;

    fn artifact() -> Result<AcceptedArtifact, String> {
        crate::admission::tests::evidence_test_support::accepted_artifact_with_caps(Box::new([]))
    }

    #[test]
    fn malformed_ir_preserves_decoder_error_only_for_recovery() -> Result<(), String> {
        let mut artifact = artifact()?;
        artifact.ir.clear();
        artifact.digest = vb_storage::admission::accepted_artifact_digest(&artifact)
            .map_err(|error| error.to_string())?;
        artifact.verification.digest = artifact.digest;
        let store = MissingAcceptedArtifactStore;
        let detailed =
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Strict));
        assert_eq!(
            detailed.as_ref().err().map(|error| error.to_string()),
            Some(
                vb_storage::JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeUnexpectedEnd,
                )
                .to_string()
            )
        );
        assert!(matches!(
            detailed,
            Err(StoredArtifactVerificationError::Storage(
                vb_storage::JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeUnexpectedEnd
                )
            ))
        ));
        assert_eq!(
            verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Strict)).map(|_| ()),
            Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)
        );
        Ok(())
    }

    #[test]
    fn checksum_failure_keeps_storage_cause_and_public_classification() -> Result<(), String> {
        let mut artifact = artifact()?;
        artifact.ir.push(0);
        let store = MissingAcceptedArtifactStore;
        assert!(matches!(
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Strict)),
            Err(StoredArtifactVerificationError::Storage(
                vb_storage::JournalError::ArtifactChecksumMismatch
            ))
        ));
        assert_eq!(
            verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Strict)).map(|_| ()),
            Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)
        );
        Ok(())
    }

    #[test]
    fn policy_failure_keeps_storage_cause_and_public_classification() -> Result<(), String> {
        let artifact = artifact()?;
        let store = MissingAcceptedArtifactStore;
        assert!(matches!(
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Journaled)),
            Err(StoredArtifactVerificationError::Storage(
                vb_storage::JournalError::ArtifactMalformed
            ))
        ));
        assert_eq!(
            verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Journaled)).map(|_| ()),
            Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)
        );
        Ok(())
    }

    #[test]
    fn packed_evidence_without_catalog_remains_rejected() -> Result<(), String> {
        let mut artifact = artifact()?;
        let ArtifactEvidence::Verified(base) = artifact.verification.evidence.clone() else {
            return Err(String::from("expected verified fixture evidence"));
        };
        artifact.verification.evidence =
            ArtifactEvidence::Packed(Box::new(PackedArtifactEvidence {
                base: *base,
                packs: Box::new([]),
            }));
        let store = MissingAcceptedArtifactStore;
        assert!(matches!(
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Strict)),
            Err(StoredArtifactVerificationError::Executor(
                vb_storage::executor::ExecutorError::CatalogRequired
            ))
        ));
        assert_eq!(
            verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Strict)).map(|_| ()),
            Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)
        );
        Ok(())
    }

    #[test]
    fn valid_verified_artifact_keeps_both_success_paths() -> Result<(), String> {
        let artifact = artifact()?;
        let store = MissingAcceptedArtifactStore;
        let detailed =
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Strict))
                .map_err(|error| error.to_string())?;
        let public = verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Strict))
            .map_err(|error| error.to_string())?;
        assert_eq!(detailed.digest(), artifact.source_digest);
        assert_eq!(public, detailed);
        Ok(())
    }
}
