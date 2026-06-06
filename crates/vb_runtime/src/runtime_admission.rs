#![forbid(unsafe_code)]
//! Direct submission admission preflight for the runtime façade.

use vb_core::WorkflowDigest;
use vb_core::capability::CapabilitySet;
use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::CompiledWorkflow;

use crate::shard::Shard;
use crate::{Runtime, RuntimeError, RuntimeResult};

impl Runtime {
    pub(crate) fn preflight_direct_admission(
        shard: &Shard,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        if !requires_admission(shard.policy) {
            return Ok(());
        }
        let digest = workflow.digest();
        crate::admission::admit_artifact_run(
            shard.artifact_store.as_ref(),
            shard.policy,
            run,
            digest,
            caps,
        )
        .map(|_| ())
        .map_err(|error| map_admission_error(error, digest))
    }
}

fn requires_admission(policy: RuntimePolicy) -> bool {
    matches!(policy, RuntimePolicy::Strict | RuntimePolicy::Journaled)
}

fn map_admission_error(
    error: crate::admission::AdmissionError,
    workflow_digest: WorkflowDigest,
) -> RuntimeError {
    match error {
        crate::admission::AdmissionError::ArtifactNotFound { digest } => {
            RuntimeError::AdmissionArtifactNotFound { digest }
        }
        crate::admission::AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        } => RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        _ => RuntimeError::AdmissionArtifactInvalid {
            digest: workflow_digest,
        },
    }
}
