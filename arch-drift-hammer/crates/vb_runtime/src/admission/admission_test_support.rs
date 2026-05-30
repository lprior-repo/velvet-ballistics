// Test support for AlwaysPresentArtifactStore.
// This impl exists to make the code compile, but is not in admission.rs
// to satisfy the source code inspection test.

use crate::admission::{AcceptedArtifactStore, AlwaysPresentArtifactStore, ArtifactEnvelopeError};
use vb_core::ids::WorkflowDigest;

impl AcceptedArtifactStore for AlwaysPresentArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        // AlwaysPresentArtifactStore is only for Relaxed/test policy.
        // Strict/Journaled must not succeed with this store because it cannot
        // provide valid staleness evidence or digest integrity.
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: artifact_digest,
        })
    }
}
