#![forbid(unsafe_code)]
//! Direct submission admission preflight for the runtime façade.

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
        if !matches!(
            shard.policy,
            RuntimePolicy::Strict | RuntimePolicy::Journaled
        ) {
            return Ok(());
        }
        crate::admission::admit_artifact_run(
            shard.artifact_store.as_ref(),
            shard.policy,
            run,
            workflow.digest(),
            caps,
        )
        .map(|_| ())
        .map_err(|error| match error {
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
                digest: workflow.digest(),
            },
        })
    }
}
