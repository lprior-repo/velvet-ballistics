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
        // Per-action attempt counters are captured verbatim: the CLI retry advances
        // the run attempt (checked), not the per-step action attempt. Advancing the
        // action attempt here would drive the re-executed ticket past the action's
        // max-attempts capacity, so the failed step's action re-executes as its first
        // action attempt in the new run attempt.
        let action_attempts = state.action_attempts.clone();
        if action_attempts.get(retry_index).is_none() {
            return Err(RunCheckpointV1Error::ActionAttemptCountMismatch);
        }
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
            action_attempts,
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
