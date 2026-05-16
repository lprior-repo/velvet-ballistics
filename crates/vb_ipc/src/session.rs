//! Session management for the IPC server.

use vb_core::ids::WorkflowDigest;
use vb_core::workflow::CompiledWorkflow;

/// Workflow resolution failed before runtime submission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkflowResolutionError {
    /// No resolver is wired into this IPC surface.
    #[error("workflow resolution required")]
    Required,
    /// The requested workflow digest is unknown.
    #[error("workflow not found")]
    NotFound,
    /// Resolver rejected the compiled workflow artifact.
    #[error("workflow artifact invalid")]
    InvalidArtifact,
}

/// Resolves compiled workflows for IPC submit commands.
pub trait WorkflowResolver {
    /// Returns the compiled workflow for an already-validated digest.
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError>;
}
