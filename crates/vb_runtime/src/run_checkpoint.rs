#![forbid(unsafe_code)]
//! Versioned complete checkpoint payload for one retry boundary.

use core::num::NonZeroU16;

use serde::{Deserialize, Serialize};
use vb_core::action::ActionContract;
use vb_core::frame::{CheckpointStepState, RunFrame, RunFrameCheckpointV1};
use vb_core::ids::{RunId, StepIdx, WorkflowDigest};
use vb_core::value_store::{ValueStore, ValueStoreCheckpointV1};
use vb_core::workflow::CompiledWorkflow;

use crate::admission::RunAdmission;
use crate::primitives::collect::{CollectStates, CollectStatesCheckpointV1};
use crate::shard::types::RunState;

/// Complete v1 state needed to restore a run at a retry boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCheckpointV1 {
    /// Checkpoint format version.
    pub version: u16,
    /// Run whose state is captured.
    pub run_id: RunId,
    /// Accepted artifact identity used to load the immutable workflow.
    pub artifact_digest: WorkflowDigest,
    /// Compiled workflow identity checked on restore.
    pub workflow_digest: WorkflowDigest,
    /// Durable attempt represented by this checkpoint.
    pub run_attempt: NonZeroU16,
    /// Failed step the retry command is authorized to execute again.
    pub retry_step: StepIdx,
    /// Journal sequence immediately before the checkpoint is appended.
    pub prior_seq: u64,
    /// Full execution frame, including slots, taint, PC, counters, and every step state.
    pub frame: RunFrameCheckpointV1,
    /// Full cold value-store arenas and limits.
    pub value_store: ValueStoreCheckpointV1,
    /// Per-action-step attempt counters.
    pub action_attempts: Box<[u16]>,
    /// Admission authority captured with the state.
    pub admission: Option<RunAdmission>,
    /// Pagination state and page lineage captured with the state.
    pub collect_states: CollectStatesCheckpointV1,
    /// Validated action contracts bound to the accepted artifact.
    pub action_contracts: Box<[ActionContract]>,
}

/// Errors proving a checkpoint cannot authorize restoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RunCheckpointV1Error {
    /// The checkpoint version is not supported.
    #[error("unsupported runtime checkpoint version")]
    UnsupportedVersion,
    /// Frame run identity differs from the checkpoint identity.
    #[error("checkpoint frame belongs to a different run")]
    RunIdentityMismatch,
    /// Admission run identity differs from the checkpoint identity.
    #[error("checkpoint admission belongs to a different run")]
    AdmissionRunMismatch,
    /// Retry checkpoint lacks admission authority.
    #[error("retry checkpoint has no accepted admission authority")]
    MissingAdmissionAuthority,
    /// Admission artifact identity differs from the checkpoint identity.
    #[error("checkpoint admission has a different artifact digest")]
    ArtifactIdentityMismatch,
    /// Compiled workflow identity differs from checkpoint metadata.
    #[error("checkpoint workflow digest mismatch")]
    WorkflowIdentityMismatch,
    /// Frame dimensions do not match the accepted workflow.
    #[error("checkpoint frame dimensions differ from the workflow")]
    FrameDimensionsMismatch,
    /// Retry step is outside the workflow or not failed in the captured frame.
    #[error("checkpoint retry step is not the failed step")]
    RetryStepMismatch,
    /// Action attempt counters do not cover every frame step.
    #[error("checkpoint action attempt count differs from frame step count")]
    ActionAttemptCountMismatch,
    /// Checkpoint action contracts differ from the independently loaded artifact registry.
    #[error("checkpoint action contracts differ from accepted artifact")]
    ActionContractsMismatch,
    /// The per-step action attempt counter cannot be advanced.
    #[error("checkpoint action attempt counter overflow")]
    ActionAttemptOverflow,
    /// A nested state DTO failed validation.
    #[error("nested checkpoint validation failed")]
    InvalidNestedState,
}

impl RunCheckpointV1 {
    /// Captures every mutable run-state component after validating retry authority.
    pub fn capture(
        run_id: RunId,
        artifact_digest: WorkflowDigest,
        run_attempt: NonZeroU16,
        retry_step: StepIdx,
        prior_seq: u64,
        state: &RunState,
    ) -> Result<Self, RunCheckpointV1Error> {
        if state.frame.checkpoint_v1().run_id != run_id {
            return Err(RunCheckpointV1Error::RunIdentityMismatch);
        }
        if let Some(admission) = &state.admission {
            validate_admission(admission, run_id, artifact_digest)?;
        } else {
            return Err(RunCheckpointV1Error::MissingAdmissionAuthority);
        }
        let mut frame = state.frame.checkpoint_v1();
        let retry_index = usize::from(retry_step.get());
        if frame.states.get(retry_index) != Some(&CheckpointStepState::Failed) {
            return Err(RunCheckpointV1Error::RetryStepMismatch);
        }
        let mut action_attempts = state.action_attempts.to_vec();
        let action_attempt = action_attempts
            .get_mut(retry_index)
            .ok_or(RunCheckpointV1Error::ActionAttemptCountMismatch)?;
        *action_attempt = action_attempt
            .checked_add(1)
            .ok_or(RunCheckpointV1Error::ActionAttemptOverflow)?;
        // Checkpoints are retry-ready state: keep the audited failed event in the journal,
        // while resetting only the selected step and PC in the new checkpoint.
        let retry_state = frame
            .states
            .get_mut(retry_index)
            .ok_or(RunCheckpointV1Error::RetryStepMismatch)?;
        *retry_state = CheckpointStepState::Pending;
        frame.pc = retry_step;
        let checkpoint = Self {
            version: 1,
            run_id,
            artifact_digest,
            workflow_digest: state.workflow.digest(),
            run_attempt,
            retry_step,
            prior_seq,
            frame,
            value_store: state.store.checkpoint_v1(),
            action_attempts: action_attempts.into_boxed_slice(),
            admission: state.admission.clone(),
            collect_states: state.collect_states.checkpoint_v1(),
            action_contracts: state.action_contracts.clone(),
        };
        checkpoint.validate_against(&state.workflow, artifact_digest, &state.action_contracts)?;
        Ok(checkpoint)
    }

    /// Validates all identities and nested state against the accepted compiled workflow.
    pub fn validate_against(
        &self,
        workflow: &CompiledWorkflow,
        artifact_digest: WorkflowDigest,
        accepted_action_contracts: &[ActionContract],
    ) -> Result<(), RunCheckpointV1Error> {
        if self.version != 1 {
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
        let step_count = u16::try_from(self.frame.states.len())
            .map_err(|_| RunCheckpointV1Error::FrameDimensionsMismatch)?;
        let slot_count = u16::try_from(self.frame.slots.len())
            .map_err(|_| RunCheckpointV1Error::FrameDimensionsMismatch)?;
        if step_count != workflow.node_count() || slot_count != workflow.slot_count() {
            return Err(RunCheckpointV1Error::FrameDimensionsMismatch);
        }
        let retry_index = usize::from(self.retry_step.get());
        if self.frame.states.get(retry_index) != Some(&CheckpointStepState::Pending)
            || self.frame.pc != self.retry_step
        {
            return Err(RunCheckpointV1Error::RetryStepMismatch);
        }
        if self.action_attempts.len() != self.frame.states.len() {
            return Err(RunCheckpointV1Error::ActionAttemptCountMismatch);
        }
        if let Some(admission) = &self.admission {
            validate_admission(admission, self.run_id, self.artifact_digest)?;
        } else {
            return Err(RunCheckpointV1Error::MissingAdmissionAuthority);
        }
        if self.action_contracts.as_ref() != accepted_action_contracts {
            return Err(RunCheckpointV1Error::ActionContractsMismatch);
        }
        RunFrame::from_checkpoint_v1(self.frame.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        ValueStore::from_checkpoint_v1(self.value_store.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        CollectStates::from_checkpoint_v1(self.collect_states.clone())
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        Ok(())
    }

    /// Restores a validated checkpoint using the independently loaded accepted workflow.
    pub fn restore(
        self,
        workflow: CompiledWorkflow,
        artifact_digest: WorkflowDigest,
        accepted_action_contracts: &[ActionContract],
    ) -> Result<(RunState, NonZeroU16, StepIdx, u64), RunCheckpointV1Error> {
        self.validate_against(&workflow, artifact_digest, accepted_action_contracts)?;
        let frame = RunFrame::from_checkpoint_v1(self.frame)
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        let store = ValueStore::from_checkpoint_v1(self.value_store)
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        let collect_states = CollectStates::from_checkpoint_v1(self.collect_states)
            .map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
        let state = RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::ReconcileOnly,
            frame,
            run_attempt: self.run_attempt,
            workflow,
            store,
            action_attempts: self.action_attempts,
            admission: self.admission,
            collect_states,
            action_contracts: self.action_contracts,
        };
        Ok((state, self.run_attempt, self.retry_step, self.prior_seq))
    }
}

fn validate_admission(
    admission: &RunAdmission,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
) -> Result<(), RunCheckpointV1Error> {
    if admission.run_id() != run_id {
        return Err(RunCheckpointV1Error::AdmissionRunMismatch);
    }
    if admission.artifact_digest() != artifact_digest {
        return Err(RunCheckpointV1Error::ArtifactIdentityMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RunCheckpointV1, RunCheckpointV1Error};
    use crate::admission::RunAdmission;
    use crate::primitives::collect::CollectStates;
    use crate::shard::types::RunState;
    use core::num::NonZeroU16;
    use vb_core::capability::CapabilitySet;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::policy::RuntimePolicy;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    fn workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::from("retry-checkpoint"),
            digest: WorkflowDigest::from_bytes([3; 32]),
        semantic_source_identity: None,
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        })
        .ok()
    }

    fn failed_state(
        workflow: vb_core::workflow::CompiledWorkflow,
        run_id: RunId,
    ) -> Option<RunState> {
        let mut frame = RunFrame::new(run_id, StepIdx::ZERO, 1, 1).ok()?;
        frame.mark_running(StepIdx::ZERO).ok()?;
        frame.mark_failed(StepIdx::ZERO).ok()?;
        Some(RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
            frame,
            run_attempt: NonZeroU16::MIN,
            workflow,
            store: ValueStore::new(),
            action_attempts: Box::from([0]),
            admission: Some(RunAdmission::new(
                WorkflowDigest::from_bytes([7; 32]),
                run_id,
                CapabilitySet::empty(),
                RuntimePolicy::Strict,
            )),
            collect_states: CollectStates::new(),
            action_contracts: Box::new([]),
        })
    }

    #[test]
    fn capture_resets_only_retry_step_and_increments_its_action_attempt() -> Result<(), String> {
        let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
        let run_id = RunId::new(41);
        let state = failed_state(workflow, run_id)
            .ok_or_else(|| "failed state setup failed".to_string())?;
        let checkpoint = RunCheckpointV1::capture(
            run_id,
            WorkflowDigest::from_bytes([7; 32]),
            NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
            StepIdx::ZERO,
            9,
            &state,
        )
        .map_err(|error| format!("capture failed: {error}"))?;

        assert_eq!(
            checkpoint.frame.states,
            [vb_core::frame::CheckpointStepState::Pending]
        );
        assert_eq!(checkpoint.frame.pc, StepIdx::ZERO);
        assert_eq!(checkpoint.action_attempts.as_ref(), [1]);
        assert_eq!(
            state.frame.step_state(StepIdx::ZERO),
            Ok(vb_core::frame::StepState::Failed)
        );
        assert_eq!(checkpoint.prior_seq, 9);
        let bytes = postcard::to_allocvec(&checkpoint)
            .map_err(|error| format!("checkpoint encode failed: {error}"))?;
        let decoded: RunCheckpointV1 = postcard::from_bytes(&bytes)
            .map_err(|error| format!("checkpoint decode failed: {error}"))?;
        assert_eq!(decoded, checkpoint);
        let (restored, attempt, retry_step, prior_seq) = decoded
            .restore(
                state.workflow.clone(),
                WorkflowDigest::from_bytes([7; 32]),
                &state.action_contracts,
            )
            .map_err(|error| format!("checkpoint restore failed: {error}"))?;
        assert_eq!(attempt.get(), 2);
        assert_eq!(retry_step, StepIdx::ZERO);
        assert_eq!(prior_seq, 9);
        assert_eq!(
            restored.frame.step_state(StepIdx::ZERO),
            Ok(vb_core::frame::StepState::Pending)
        );
        assert_eq!(restored.action_attempts.as_ref(), [1]);
        Ok(())
    }

    #[test]
    fn restore_rejects_a_checkpoint_for_a_different_workflow() -> Result<(), String> {
        let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
        let run_id = RunId::new(42);
        let state = failed_state(workflow.clone(), run_id)
            .ok_or_else(|| "failed state setup failed".to_string())?;
        let mut checkpoint = RunCheckpointV1::capture(
            run_id,
            WorkflowDigest::from_bytes([7; 32]),
            NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
            StepIdx::ZERO,
            3,
            &state,
        )
        .map_err(|error| format!("capture failed: {error}"))?;
        checkpoint.workflow_digest = WorkflowDigest::from_bytes([9; 32]);
        assert_eq!(
            checkpoint.validate_against(
                &workflow,
                WorkflowDigest::from_bytes([7; 32]),
                &state.action_contracts,
            ),
            Err(RunCheckpointV1Error::WorkflowIdentityMismatch)
        );
        Ok(())
    }
}
