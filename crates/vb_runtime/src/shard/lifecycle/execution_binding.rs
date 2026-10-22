//! Bind the exact execution tuple to the independently checked accepted artifact.
use crate::admission::{AcceptedArtifactStore, ArtifactEnvelopeError};
use crate::{RuntimeError, RuntimeResult};
use vb_core::{ActionContract, CompiledWorkflow, RuntimePolicy, WorkflowDigest};
use vb_storage::AcceptedArtifact;

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
    let artifact = load_artifact(store, digest)?;
    crate::admission::verify_stored_artifact(store, &artifact, Some(policy))
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid { digest })?;
    compare_execution_tuple(&artifact, workflow, contracts)
}

fn load_artifact(
    store: &dyn AcceptedArtifactStore,
    digest: WorkflowDigest,
) -> RuntimeResult<AcceptedArtifact> {
    store
        .load_accepted_artifact(digest)
        .map_err(|error| match error {
            ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                RuntimeError::AdmissionArtifactNotFound { digest }
            }
            ArtifactEnvelopeError::ArtifactDigestMismatch { requested, found } => {
                RuntimeError::AdmissionArtifactDigestMismatch { requested, found }
            }
            _ => RuntimeError::AdmissionArtifactInvalid { digest },
        })
}

fn compare_execution_tuple(
    artifact: &AcceptedArtifact,
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> RuntimeResult<()> {
    let digest = workflow.digest();
    if artifact.source_digest != digest {
        return Err(RuntimeError::AdmissionArtifactDigestMismatch {
            requested: digest,
            found: artifact.source_digest,
        });
    }
    compare_action_abi_digests(artifact, contracts)?;
    let payload = postcard::to_allocvec(&workflow.to_parts())
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid { digest })?;
    if payload != artifact.ir || contracts != artifact.action_contracts.as_ref() {
        return Err(RuntimeError::AdmissionArtifactInvalid { digest });
    }
    Ok(())
}

/// Compares submitter action ABI digests against the accepted artifact's
/// contracts before the full contract-equality comparison, so a divergent
/// submitter contract surfaces as a specific `AdmissionActionAbiDigestMismatch`
/// rather than a generic `AdmissionArtifactInvalid`.
fn compare_action_abi_digests(
    artifact: &AcceptedArtifact,
    contracts: &[ActionContract],
) -> RuntimeResult<()> {
    for contract in contracts {
        let Some(accepted) = artifact
            .action_contracts
            .iter()
            .find(|accepted| accepted.id == contract.id)
        else {
            continue;
        };
        let expected = crate::recovery::action_contract_abi_digest(accepted)?;
        let submitted = crate::recovery::action_contract_abi_digest(contract)?;
        if submitted != expected {
            return Err(RuntimeError::AdmissionActionAbiDigestMismatch {
                action: contract.id,
                expected,
                submitted,
            });
        }
    }
    Ok(())
}
