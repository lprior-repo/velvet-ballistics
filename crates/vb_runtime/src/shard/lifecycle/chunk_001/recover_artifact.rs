impl Shard {
    fn load_recovery_artifact(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> RuntimeResult<vb_storage::AcceptedArtifact> {
        self.artifact_store
            .load_accepted_artifact(digest)
            .map_err(|error| match error {
                crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                    RuntimeError::Recovery {
                        error: format!("artifact not found during recovery: {digest:?}"),
                    }
                }
                other => RuntimeError::Recovery {
                    error: format!("artifact load failed: {other}"),
                },
            })
    }
}
