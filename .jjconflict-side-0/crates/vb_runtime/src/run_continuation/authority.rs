use super::RunContinuationError;
use crate::admission::RunAdmission;
use crate::recovery::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, hydrate_run_admission_from_events,
    validate_recovered_admission_evidence,
};
use core::num::NonZeroU16;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{AcceptedArtifact, EventSeq, JournalEvent, OwnedRunInputsRecordV1};

/// Independently selected identities; callers must prove durable prefix provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContinuationExpected {
    /// Run selected from the authoritative committed prefix.
    pub run: RunId,
    /// Independently accepted artifact identity.
    pub artifact_digest: WorkflowDigest,
    /// Original authored source identity.
    pub source_digest: WorkflowDigest,
    /// Independently required runtime policy.
    pub policy: RuntimePolicy,
    /// Real durable attempt; this component does not derive its provenance.
    pub run_attempt: NonZeroU16,
    /// Exact prior durable sequence; this component does not select the prefix.
    pub prior_seq: EventSeq,
}

/// Verified component content authority, distinct from public activation authority.
pub struct ContinuationAuthority {
    pub(super) expected: ContinuationExpected,
    pub(super) workflow: CompiledWorkflow,
    pub(super) original: OwnedRootBundle,
    pub(super) input_digest: WorkflowDigest,
    pub(super) admission: RunAdmission,
    pub(super) contracts: Box<[vb_core::action::ActionContract]>,
}

impl ContinuationAuthority {
    /// Binds a validated retained record to its actual durable admission and artifact.
    ///
    /// Use the validated Fjall owned reader and actual committed receipt. Matching
    /// caller content alone does not prove prefix, sequence or attempt provenance.
    pub fn from_owned_record(
        record: &OwnedRunInputsRecordV1,
        durable_admission: &JournalEvent,
        artifact: &AcceptedArtifact,
        expected: ContinuationExpected,
    ) -> Result<Self, RunContinuationError> {
        validate_identities(record, artifact, expected)?;
        let workflow =
            vb_storage::admission::verify_accepted_artifact_for_policy(artifact, expected.policy)?;
        record.validate_admission(durable_admission)?;
        let original = record.restore(&workflow)?;
        let admission = hydrate_run_admission_from_events(std::slice::from_ref(durable_admission))
            .ok_or(RunContinuationError::Admission)?;
        validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
            context: RecoveredAdmissionContext {
                run: expected.run,
                expected_workflow: workflow.digest(),
                runtime_policy: expected.policy,
            },
            admission: &admission,
            artifact,
            workflow: &workflow,
        })?;
        Ok(Self {
            expected,
            workflow,
            original,
            input_digest: record.input_digest(),
            admission,
            contracts: artifact.action_contracts.clone(),
        })
    }
}

fn validate_identities(
    record: &OwnedRunInputsRecordV1,
    artifact: &AcceptedArtifact,
    expected: ContinuationExpected,
) -> Result<(), RunContinuationError> {
    if record.run() != expected.run
        || record.artifact_digest() != expected.artifact_digest
        || artifact.digest != expected.artifact_digest
        || record.source_digest() != expected.source_digest
    {
        return Err(RunContinuationError::Identity);
    }
    Ok(())
}
