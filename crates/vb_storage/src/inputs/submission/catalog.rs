use super::{RunSubmission, SubmissionInputs};
use crate::{
    FjallJournal, JournalError, OwnedRunInputsRecordV1, RunHeaderRecord, WorkflowSourceRecord,
};
use vb_core::{AggregateResourceBudget, CapabilitySet, RuntimePolicy};

/// Transient trusted inputs for one packed registered-artifact submission.
pub struct ExecutorCatalogSubmissionContext<'a> {
    catalog: &'a crate::executor::ExecutorCatalog,
    granted_capabilities: CapabilitySet,
}

impl<'a> ExecutorCatalogSubmissionContext<'a> {
    /// Binds the actual registered catalog to caller-supplied capabilities.
    pub fn new(
        catalog: &'a crate::executor::ExecutorCatalog,
        granted_capabilities: CapabilitySet,
    ) -> Self {
        Self {
            catalog,
            granted_capabilities,
        }
    }
}

impl RunSubmission {
    /// Prepares owned inputs with a real executor catalog for registered artifacts.
    pub fn owned_with_executor_catalog(
        source: WorkflowSourceRecord,
        artifact: crate::AcceptedArtifact,
        inputs: OwnedRunInputsRecordV1,
        header: RunHeaderRecord,
        context: ExecutorCatalogSubmissionContext<'_>,
    ) -> Result<Self, JournalError> {
        let budget_origin_millis = header.accepted_at_ms;
        let workflow = context
            .catalog
            .verify_artifact(&artifact, artifact_policy(&artifact))
            .map_err(|_| JournalError::ArtifactMalformed)?;
        let budget = AggregateResourceBudget::from_workflow(&workflow)
            .map_err(|_| JournalError::ArtifactMalformed)?;
        let submission = Self {
            source,
            artifact,
            inputs: SubmissionInputs::Owned(inputs),
            header,
            budget: Some(budget),
            budget_origin_millis: Some(budget_origin_millis),
            granted_capabilities: context.granted_capabilities,
        };
        submission.validate_binding_with_executor_catalog(context.catalog)?;
        submission.validate_catalog_capabilities()?;
        Ok(submission)

    }
    fn validate_binding_with_executor_catalog(
        &self,
        catalog: &crate::executor::ExecutorCatalog,
    ) -> Result<(), JournalError> {
        self.validate_identities()?;
        crate::journal::verify_content_digest(&self.source.source, &self.source.digest.as_bytes())?;
        let workflow = catalog
            .verify_artifact(&self.artifact, artifact_policy(&self.artifact))
            .map_err(|_| JournalError::ArtifactMalformed)?;
        self.validate_workflow_binding(&workflow)
    }

    pub(super) fn validate_policy_with_executor_catalog(
        &self,
        policy: RuntimePolicy,
        catalog: &crate::executor::ExecutorCatalog,
    ) -> Result<(), JournalError> {
        self.validate_policy_requirements(policy)?;
        self.validate_catalog_capabilities()?;
        catalog
            .verify_artifact(&self.artifact, policy)
            .map_err(|_| JournalError::ArtifactMalformed)?;
        Ok(())
    }

    fn validate_catalog_capabilities(&self) -> Result<(), JournalError> {
        let required = &self.artifact.required_capabilities;
        let all_required_granted = required
            .iter()
            .all(|capability| self.granted_capabilities.grants(capability));
        if !all_required_granted || self.granted_capabilities.len() != required.len() {
            return Err(JournalError::CapabilityDenied);
        }
        Ok(())
    }
}

impl FjallJournal {
    /// Commits a catalog-verified packed artifact and owned inputs atomically.
    pub fn commit_run_submission_with_executor_catalog(
        &self,
        submission: &RunSubmission,
        policy: RuntimePolicy,
        catalog: &crate::executor::ExecutorCatalog,
    ) -> Result<(), JournalError> {
        submission.validate_policy_with_executor_catalog(policy, catalog)?;
        self.commit_run_submission_validated(submission, policy, Some(catalog))
    }
}

fn artifact_policy(artifact: &crate::AcceptedArtifact) -> RuntimePolicy {
    match &artifact.verification.evidence {
        vb_core::artifact::ArtifactEvidence::Packed(packed) => packed.base.admission_policy,
        vb_core::artifact::ArtifactEvidence::Verified(evidence) => evidence.admission_policy,
        vb_core::artifact::ArtifactEvidence::Unverified => RuntimePolicy::Strict,
    }
}
