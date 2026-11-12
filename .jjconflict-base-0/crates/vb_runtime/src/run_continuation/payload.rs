use super::{ContinuationExpected, RunContinuationError};
use crate::admission::RunAdmission;
use crate::primitives::collect::CollectStatesCheckpointV1;
use serde::{Deserialize, Serialize};
use vb_core::action::ActionContract;
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{StepIdx, WorkflowDigest};

pub(super) const VERSION: u16 = 1;
const DOMAIN: &str = "velvet-ballistics/run-continuation/v1";

/// Two supported quiescent cuts, identified from actual compiled edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationCut {
    /// Terminal CollectNext completed; CollectFinish has not run.
    BeforeCollectFinish,
    /// CollectFinish completed; the compiled exit has not run.
    AfterCollectFinish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Boundary {
    pub(super) cut: ContinuationCut,
    pub(super) control: StepIdx,
    pub(super) finish: StepIdx,
    pub(super) exit: StepIdx,
}

/// Complete versioned preimage; it deliberately excludes its own digest.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Body {
    pub(super) version: u16,
    pub(super) expected: ContinuationExpected,
    pub(super) workflow: WorkflowDigest,
    pub(super) input_digest: WorkflowDigest,
    pub(super) boundary: Boundary,
    pub(super) frame: RunFrameCheckpointV1,
    pub(super) value_store: ValueStoreCheckpointV1,
    pub(super) action_attempts: Box<[u16]>,
    pub(super) admission: RunAdmission,
    pub(super) collect_states: CollectStatesCheckpointV1,
    pub(super) action_contracts: Box<[ActionContract]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Wire {
    pub(super) body: Body,
    pub(super) digest: WorkflowDigest,
}

impl Body {
    pub(super) fn capture(
        state: &crate::shard::types::RunState,
        authority: &super::ContinuationAuthority,
        admission: RunAdmission,
        frame: RunFrameCheckpointV1,
        boundary: Boundary,
    ) -> Self {
        Self {
            version: VERSION,
            expected: authority.expected,
            workflow: authority.workflow.digest(),
            input_digest: authority.input_digest,
            boundary,
            frame,
            value_store: state.store.checkpoint_v1(),
            action_attempts: state.action_attempts.clone(),
            admission,
            collect_states: state.collect_states.checkpoint_v1(),
            action_contracts: state.action_contracts.clone(),
        }
    }

    pub(super) fn digest(&self) -> Result<WorkflowDigest, RunContinuationError> {
        let bytes = postcard::to_allocvec(self)?;
        Ok(WorkflowDigest::from_bytes(blake3::derive_key(
            DOMAIN, &bytes,
        )))
    }
}

impl Wire {
    pub(super) fn validate_integrity(&self) -> Result<(), RunContinuationError> {
        if self.body.version != VERSION {
            return Err(RunContinuationError::UnsupportedVersion {
                version: self.body.version,
            });
        }
        if self.body.digest()? != self.digest {
            return Err(RunContinuationError::Digest);
        }
        Ok(())
    }
}
