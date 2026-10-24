use super::super::{ContinuationAuthority, RunContinuationError};
use super::{BoundedContinuationPayload, ContinuationTransportLimits, TransportFailure};
use super::{
    encode,
    operation::{Operation, buffer},
    reference::BodyRef,
};
use crate::primitives::collect::CollectStatesCheckpointV1;
use crate::shard::types::RunState;
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::value_store::transport::CheckpointTransportError as Error;

pub(super) fn capture(
    state: &RunState,
    authority: &ContinuationAuthority,
    limits: ContinuationTransportLimits,
    op: &mut Operation,
) -> Result<BoundedContinuationPayload, TransportFailure> {
    let frame = frame(state, authority, op)?;
    super::super::validation::quiescent(&frame)?;
    let boundary = super::super::boundary::locate(&frame, authority)?;
    let collect = CollectStatesCheckpointV1 {
        entries: Vec::new(),
        lineages: Vec::new(),
    };
    let body = BodyRef::new(state, authority, &frame, boundary, &collect)?;
    let bytes = encode_body(&body, limits.component)?;
    let validated = encode::decode(&bytes, authority, op)?;
    drop(validated);
    Ok(BoundedContinuationPayload {
        bytes,
        usage: op.budget.usage(),
    })
}

fn frame(
    state: &RunState,
    authority: &ContinuationAuthority,
    op: &mut Operation,
) -> Result<RunFrameCheckpointV1, TransportFailure> {
    if state.workflow != authority.workflow {
        return Err(RunContinuationError::WorkflowIdentity.into());
    }
    if state.run_attempt != authority.expected.run_attempt {
        return Err(RunContinuationError::AttemptIdentity.into());
    }
    if !state.collect_states.transport_tables_empty() {
        return Err(RunContinuationError::CollectState.into());
    }
    let nodes = usize::from(authority.workflow.node_count());
    let slots = usize::from(authority.workflow.slot_count());
    if usize::from(state.frame.step_count()) != nodes
        || usize::from(state.frame.slot_count()) != slots
    {
        return Err(RunContinuationError::Dimensions.into());
    }
    op.budget
        .work(nodes.checked_add(slots).ok_or(Error::Overflow)?)?;
    op.budget.reserve(nodes, 8)?;
    op.budget.reserve(slots, 64)?;
    Ok(state.frame.checkpoint_v1())
}

fn encode_body(body: &BodyRef<'_>, limit: usize) -> Result<Vec<u8>, TransportFailure> {
    let mut bytes = buffer(limit)?;
    let body_cap = limit.checked_sub(32).ok_or(Error::Overflow)?;
    let body_space = bytes.get_mut(..body_cap).ok_or(Error::Overflow)?;
    let encoded = postcard::to_slice(body, body_space)?;
    let body_len = encoded.len();
    let digest = blake3::derive_key("velvet-ballistics/run-continuation/v1", encoded);
    let end = body_len.checked_add(32).ok_or(Error::Overflow)?;
    bytes
        .get_mut(body_len..end)
        .ok_or(Error::Overflow)?
        .copy_from_slice(&digest);
    bytes.truncate(end);
    Ok(bytes)
}
