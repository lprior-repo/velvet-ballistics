//! Recovery admission validation and construction phases.
//!
//! Separates recovery admission into distinct phases that complete before
//! any sequence or slot mutation:
//! - Phase A: Validate the command's recovered admission evidence
//! - Phase B: Validate checkpoint policy pin if present
//! - Phase C: Build unverified admission for legacy recovery

use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::{CapabilitySet, CompiledWorkflow};
use vb_storage::RunPolicyPin;
use vb_storage::admission::AcceptedArtifact;

use crate::admission::RunAdmission;
use crate::recovery::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, validate_recovered_admission_evidence,
    validate_recovered_policy_pin,
};
use crate::run_checkpoint::RunCheckpointV1;
use crate::shard::types::RecoveredAdmissionKind;
use crate::{RuntimeError, RuntimeResult};

/// Phase A: Validate the command's recovered admission evidence.
///
/// For Historical admissions, validates the full recovered admission evidence
/// using the existing RecoveredAdmissionContext/Evidence types.
///
/// For Unverified admissions, validates only run identity and artifact
/// digest matching. Policy pin cannot be validated without historical
/// admission, which is the correct fail-closed behavior for legacy
/// raw recovery.
pub(crate) fn validate_recovered_admission(
    kind: &RecoveredAdmissionKind,
    context: RecoveredAdmissionContext,
    artifact: &AcceptedArtifact,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    match kind {
        RecoveredAdmissionKind::Historical(historical) => {
            validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
                context,
                admission: historical,
                artifact,
                workflow,
            })
        }
        RecoveredAdmissionKind::Unverified {
            run_id,
            artifact_digest: cmd_digest,
        } => validate_unverified_identity(context.run, *run_id, artifact.digest, *cmd_digest),
    }
}

fn validate_unverified_identity(
    run: RunId,
    admission_run_id: RunId,
    expected_artifact_digest: WorkflowDigest,
    admission_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    if admission_run_id != run {
        return Err(RuntimeError::Recovery {
            error: format!(
                "admission run identity mismatch: expected {run:?}, got {admission_run_id:?}"
            ),
        });
    }
    if admission_digest != expected_artifact_digest {
        return Err(RuntimeError::Recovery {
            error: format!(
                "admission artifact digest mismatch: expected {expected_artifact_digest:?}, got {admission_digest:?}"
            ),
        });
    }
    Ok(())
}

/// Phase B: Validate checkpoint policy pin if a retry checkpoint carries admission.
///
/// When a retry checkpoint carries an admission, its policy pin must match
/// the loaded artifact's policy before any state or journal mutation. This
/// applies to both Historical and Unverified command paths because the
/// public new(...).with_retry_checkpoint(...) API can attach a checkpoint
/// to an Unverified command without Runtime prefix validation.
pub(crate) fn validate_checkpoint_pin(
    checkpoint: Option<&RunCheckpointV1>,
    artifact: &AcceptedArtifact,
) -> RuntimeResult<()> {
    if let Some(checkpoint) = checkpoint
        && let Some(admission) = checkpoint.admission.as_ref()
    {
        validate_recovered_policy_pin(admission, artifact)?;
    }
    Ok(())
}

/// Phase C: Build unverified admission for legacy recovery.
///
/// For Unverified kind recovery, build admission using the artifact's
/// required capabilities and current policy, then force the Unverified pin
/// so recovery never promotes to Pinned.
pub(crate) fn build_unverified_admission(
    shard: &crate::shard::Shard,
    run: RunId,
    artifact_digest: WorkflowDigest,
    artifact: &AcceptedArtifact,
) -> RuntimeResult<Option<RunAdmission>> {
    let caps = CapabilitySet::from_grants(artifact.required_capabilities.clone());
    match shard.build_admission(run, artifact_digest, caps)? {
        Some(adm) => Ok(Some(adm.with_policy_pin(RunPolicyPin::Unverified))),
        None => Ok(None),
    }
}
