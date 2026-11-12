use super::{
    ActionContract, CheckpointStepState, CollectStates, CompiledWorkflow, RunCheckpointV1,
    RunCheckpointV1Error, RunFrame, ValueStore, WorkflowDigest, validate_admission,
};

impl RunCheckpointV1 {
    pub(crate) fn validate_structure_against(
        &self,
        workflow: &CompiledWorkflow,
        artifact_digest: WorkflowDigest,
        accepted_action_contracts: &[ActionContract],
    ) -> Result<(), RunCheckpointV1Error> {
        self.validate_identity(workflow, artifact_digest)?;
        self.validate_dimensions(workflow)?;
        self.validate_retry_state()?;
        self.validate_contracts(accepted_action_contracts)?;
        self.validate_nested()
    }

    fn validate_identity(
        &self,
        workflow: &CompiledWorkflow,
        artifact_digest: WorkflowDigest,
    ) -> Result<(), RunCheckpointV1Error> {
        if self.version != 3 {
            return Err(RunCheckpointV1Error::UnsupportedVersion);
        }
        if self.workflow_digest != workflow.digest() {
            return Err(RunCheckpointV1Error::WorkflowIdentityMismatch);
        }
        if self.artifact_digest != artifact_digest {
            return Err(RunCheckpointV1Error::ArtifactIdentityMismatch);
        }
        if self.frame.run_id != self.run_id {
            return Err(RunCheckpointV1Error::RunIdentityMismatch);
        }
        Ok(())
    }

    fn validate_dimensions(&self, workflow: &CompiledWorkflow) -> Result<(), RunCheckpointV1Error> {
        let step_count = u16::try_from(self.frame.states.len())
            .map_err(|_| RunCheckpointV1Error::FrameDimensionsMismatch)?;
        let slot_count = u16::try_from(self.frame.slots.len())
            .map_err(|_| RunCheckpointV1Error::FrameDimensionsMismatch)?;
        if step_count != workflow.node_count() || slot_count != workflow.slot_count() {
            return Err(RunCheckpointV1Error::FrameDimensionsMismatch);
        }
        Ok(())
    }

    fn validate_retry_state(&self) -> Result<(), RunCheckpointV1Error> {
        let retry_index = usize::from(self.retry_step.get());
        if self.frame.states.get(retry_index) != Some(&CheckpointStepState::Pending)
            || self.frame.pc != self.retry_step
        {
            return Err(RunCheckpointV1Error::RetryStepMismatch);
        }
        if self.action_attempts.len() != self.frame.states.len() {
            return Err(RunCheckpointV1Error::ActionAttemptCountMismatch);
        }
        Ok(())
    }

    fn validate_contracts(
        &self,
        accepted_action_contracts: &[ActionContract],
    ) -> Result<(), RunCheckpointV1Error> {
        let admission = self
            .admission
            .as_ref()
            .ok_or(RunCheckpointV1Error::MissingAdmissionAuthority)?;
        validate_admission(admission, self.run_id, self.artifact_digest)?;
        if self.action_contracts.as_ref() != accepted_action_contracts {
            return Err(RunCheckpointV1Error::ActionContractsMismatch);
        }
        Ok(())
    }

    fn validate_nested(&self) -> Result<(), RunCheckpointV1Error> {
        RunFrame::from_checkpoint_v1(self.frame.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        ValueStore::from_checkpoint_v1(self.value_store.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        CollectStates::from_checkpoint_v1(self.collect_states.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        Ok(())
    }
}
