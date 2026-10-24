use super::super::payload::Boundary;
use super::super::{ContinuationAuthority, ContinuationExpected};
use crate::admission::RunAdmission;
use crate::primitives::collect::CollectStatesCheckpointV1;
use serde::Serialize;
use vb_core::WorkflowDigest;
use vb_core::action::ActionContract;
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::value_store::ValueStoreCheckpointViewV1;

/// Exact same field order/schema as payload::Body; only ownership differs.
#[derive(Serialize)]
pub(super) struct BodyRef<'a> {
    version: u16,
    expected: ContinuationExpected,
    workflow: WorkflowDigest,
    input_digest: WorkflowDigest,
    boundary: Boundary,
    frame: &'a RunFrameCheckpointV1,
    value_store: ValueStoreCheckpointViewV1<'a>,
    action_attempts: &'a [u16],
    admission: &'a RunAdmission,
    collect_states: &'a CollectStatesCheckpointV1,
    action_contracts: &'a [ActionContract],
}

impl<'a> BodyRef<'a> {
    pub(super) fn new(
        state: &'a crate::shard::types::RunState,
        authority: &ContinuationAuthority,
        frame: &'a RunFrameCheckpointV1,
        boundary: Boundary,
        collect: &'a CollectStatesCheckpointV1,
    ) -> Result<Self, super::TransportFailure> {
        let admission = state
            .admission
            .as_ref()
            .ok_or(super::super::RunContinuationError::Admission)?;
        Ok(Self {
            version: super::super::payload::VERSION,
            expected: authority.expected,
            workflow: state.workflow.digest(),
            input_digest: authority.input_digest,
            boundary,
            frame,
            value_store: state.store.checkpoint_view_v1(),
            action_attempts: &state.action_attempts,
            admission,
            collect_states: collect,
            action_contracts: &state.action_contracts,
        })
    }
}
