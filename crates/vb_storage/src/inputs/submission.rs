use super::RunInputsRecord;
use crate::{
    AcceptedArtifact, CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent,
    RunHeaderRecord, WorkflowSourceRecord,
};
use vb_core::RuntimePolicy;

/// Complete prepared admission. Private members prevent partial publication.
pub struct RunSubmission {
    source: WorkflowSourceRecord,
    artifact: AcceptedArtifact,
    inputs: RunInputsRecord,
    header: RunHeaderRecord,
}

impl RunSubmission {
    /// Binds retained inputs, source and header to the exact executable artifact.
    pub fn new(
        source: WorkflowSourceRecord,
        artifact: AcceptedArtifact,
        inputs: RunInputsRecord,
        header: RunHeaderRecord,
    ) -> Result<Self, JournalError> {
        let submission = Self {
            source,
            artifact,
            inputs,
            header,
        };
        submission.validate_binding()?;
        Ok(submission)
    }

    fn validate_binding(&self) -> Result<(), JournalError> {
        self.validate_identities()?;
        crate::journal::verify_content_digest(&self.source.source, &self.source.digest.as_bytes())?;
        let workflow = crate::admission::verify_accepted_artifact(&self.artifact)?;
        self.inputs.validate_workflow(&workflow)?;
        if crate::admission::compute_policy_digest(&workflow)? != self.artifact.policy_digest {
            return Err(JournalError::PayloadDigestMismatch);
        }
        Ok(())
    }

    fn validate_identities(&self) -> Result<(), JournalError> {
        let digest = crate::admission::accepted_artifact_digest(&self.artifact)?;
        if digest != self.artifact.digest
            || digest != self.artifact.verification.digest
            || digest != self.inputs.artifact_digest()
            || self.header.run != self.inputs.run()
            || self.header.compiled_digest != self.inputs.workflow_digest()
            || self.source.digest != self.inputs.source_digest()
            || self.artifact.source_digest != self.inputs.workflow_digest()
        {
            return Err(JournalError::PayloadDigestMismatch);
        }
        Ok(())
    }

    fn validate_policy(&self, policy: RuntimePolicy) -> Result<(), JournalError> {
        if !matches!(
            policy,
            RuntimePolicy::Relaxed | RuntimePolicy::Journaled | RuntimePolicy::Strict
        ) {
            return Err(JournalError::AdmissionRequired);
        }
        if !self.artifact.required_capabilities.is_empty() {
            return Err(JournalError::CapabilityDenied);
        }
        crate::admission::verify_accepted_artifact_for_policy(&self.artifact, policy)?;
        Ok(())
    }

    fn commit(&self, journal: &FjallJournal, policy: RuntimePolicy) -> Result<(), JournalError> {
        let mut batch = journal.batch();
        self.stage(&mut batch)?;
        self.stage_acceptance(&mut batch, policy)?;
        match policy {
            RuntimePolicy::Strict => batch.strict().commit(),
            _ => batch.commit(),
        }
    }

    fn stage(&self, batch: &mut crate::JournalWriteBatch<'_>) -> Result<(), JournalError> {
        batch.put_workflow_source(&self.source)?;
        let ir =
            postcard::to_allocvec(&self.artifact).map_err(JournalError::PostcardEncodeFailed)?;
        batch.put_compiled_ir(&CompiledIrRecord {
            digest: self.artifact.digest,
            ir,
        })?;
        batch.put_run_inputs(&self.inputs)?;
        batch.put_run_header(&self.header)?;
        batch.put_workflow_index(self.header.workflow_id, self.header.run)?;
        batch.put_status_index(
            crate::IndexStatusState::Submitted,
            self.header.accepted_at_ms,
            self.header.run,
        )
    }

    fn stage_acceptance(
        &self,
        batch: &mut crate::JournalWriteBatch<'_>,
        policy: RuntimePolicy,
    ) -> Result<(), JournalError> {
        if policy == RuntimePolicy::Relaxed {
            return Ok(());
        }
        batch.append_event(&self.accepted_event())?;
        batch.append_event(&self.admission_event(policy))
    }

    fn accepted_event(&self) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: self.header.run,
            seq: EventSeq::new(0),
            workflow: self.header.compiled_digest,
        }
    }

    fn admission_event(&self, policy: RuntimePolicy) -> JournalEvent {
        JournalEvent::RunAdmission {
            input_digest: crate::RunInputBinding::Digest(self.inputs.input_digest()),
            run: self.header.run,
            seq: EventSeq::new(1),
            artifact_digest: self.artifact.digest,
            policy_pin: crate::RunPolicyPin::Pinned(self.artifact.policy_digest),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy,
        }
    }
}

impl FjallJournal {
    /// Commits source, executable artifact, inputs, header and acceptance indexes atomically.
    /// Strict returns only after SyncAll. Journaled/Relaxed retain a data-loss window.
    pub fn commit_run_submission(
        &self,
        submission: &RunSubmission,
        policy: RuntimePolicy,
    ) -> Result<(), JournalError> {
        submission.validate_policy(policy)?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if self.run_header(submission.header.run)?.is_some()
            || self.run_inputs(submission.header.run)?.is_some()
            || !self.events_for_run(submission.header.run)?.is_empty()
        {
            return Err(JournalError::RunAlreadyExists);
        }
        submission.commit(self, policy)
    }
}
