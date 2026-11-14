//! Admission evidence required to resume a recovered run.

use vb_core::CompiledWorkflow;
use vb_storage::admission::AcceptedArtifact;

use crate::admission::RunAdmission;
use crate::recovery::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, validate_recovered_admission_evidence,
};
use crate::run_checkpoint::RunCheckpointV1;
use crate::shard::types::RecoveredAdmissionKind;
use crate::{RuntimeError, RuntimeResult};

/// Phase A: Validate the command's recovered admission evidence.
///
/// Historical admissions must carry the full current evidence. An Unverified
/// admission has no durable authority to resume and is rejected before the
/// recovery caller can build state or perform effects.
pub(crate) fn validate_recovered_admission<'a>(
    kind: &'a RecoveredAdmissionKind,
    context: RecoveredAdmissionContext,
    artifact: &AcceptedArtifact,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<&'a RunAdmission> {
    let historical = historical_admission(kind)?;
    validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
        context,
        admission: historical,
        artifact,
        workflow,
    })?;
    Ok(historical)
}

fn historical_admission(kind: &RecoveredAdmissionKind) -> RuntimeResult<&RunAdmission> {
    match kind {
        RecoveredAdmissionKind::Historical(admission) => Ok(admission),
        RecoveredAdmissionKind::Unverified { .. } => Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_unverified"),
        }),
    }
}

/// Validate nested checkpoint identities and admission before recovery mutates the shard.
pub(crate) fn validate_retry_checkpoint(
    checkpoint: Option<&RunCheckpointV1>,
    context: RecoveredAdmissionContext,
    artifact: &AcceptedArtifact,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    if let Some(checkpoint) = checkpoint {
        checkpoint
            .validate_against(workflow, artifact.digest, &artifact.action_contracts)
            .map_err(|error| RuntimeError::Recovery {
                error: format!("retry checkpoint validation failed: {error}"),
            })?;
        if let Some(admission) = checkpoint.admission.as_ref() {
            validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
                context,
                admission,
                artifact,
                workflow,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::historical_admission;
    use crate::RuntimeError;
    use crate::shard::types::RecoveredAdmissionKind;
    use vb_core::{RunId, WorkflowDigest};

    #[test]
    fn unverified_recovery_has_a_typed_nonresumable_reason() {
        let kind = RecoveredAdmissionKind::Unverified {
            run_id: RunId::new(1),
            artifact_digest: WorkflowDigest::from_bytes([0; 32]),
        };
        assert!(matches!(
            historical_admission(&kind),
            Err(RuntimeError::RecoveryCannotResume { reason })
                if reason == "admission_unverified"
        ));
    }
}
