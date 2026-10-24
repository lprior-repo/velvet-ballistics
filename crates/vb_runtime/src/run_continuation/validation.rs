use super::payload::Body;
use super::{ContinuationAuthority, RunContinuationError, boundary, ownership};
use crate::primitives::collect::CollectStates;
use vb_core::frame::{CheckpointStepState, RunFrame};
use vb_core::root_bundle::validate_runtime_store;

pub(super) fn validate(
    body: &Body,
    authority: &ContinuationAuthority,
) -> Result<(), RunContinuationError> {
    identities(body, authority)?;
    if body.admission != authority.admission {
        return Err(RunContinuationError::Admission);
    }
    if body.action_contracts != authority.contracts {
        return Err(RunContinuationError::Contracts);
    }
    dimensions(body, authority)?;
    let frame =
        RunFrame::from_checkpoint_v1(body.frame.clone()).map_err(RunContinuationError::Frame)?;
    quiescent(&body.frame)?;
    validate_runtime_store(&frame, &body.value_store, &authority.workflow)?;
    ownership::validate(body, authority)?;
    CollectStates::from_checkpoint_v1(body.collect_states.clone())
        .map_err(RunContinuationError::CollectCheckpoint)?;
    if !body.collect_states.entries.is_empty() || !body.collect_states.lineages.is_empty() {
        return Err(RunContinuationError::CollectState);
    }
    if boundary::locate(&body.frame, authority)? != body.boundary {
        return Err(RunContinuationError::BoundaryState);
    }
    boundary::validate(body, authority)
}

fn identities(body: &Body, authority: &ContinuationAuthority) -> Result<(), RunContinuationError> {
    if body.expected.run != authority.expected.run
        || body.frame.run_id != authority.expected.run
        || body.expected.artifact_digest != authority.expected.artifact_digest
        || body.expected.source_digest != authority.expected.source_digest
        || body.input_digest != authority.input_digest
        || body.expected.policy != authority.expected.policy
    {
        return Err(RunContinuationError::Identity);
    }
    if body.expected.run_attempt != authority.expected.run_attempt {
        return Err(RunContinuationError::AttemptIdentity);
    }
    if body.expected.prior_seq != authority.expected.prior_seq {
        return Err(RunContinuationError::SequenceIdentity);
    }
    if body.workflow != authority.workflow.digest()
        || !authority.original.matches_workflow(&authority.workflow)
    {
        return Err(RunContinuationError::WorkflowIdentity);
    }
    Ok(())
}

fn dimensions(body: &Body, authority: &ContinuationAuthority) -> Result<(), RunContinuationError> {
    let frame = &body.frame;
    if frame.states.len() != usize::from(authority.workflow.node_count())
        || frame.slots.len() != usize::from(authority.workflow.slot_count())
        || body.action_attempts.len() != frame.states.len()
    {
        return Err(RunContinuationError::Dimensions);
    }
    let budget = vb_core::budget::AggregateResourceBudget::from_workflow(&authority.workflow)
        .map_err(|error| RunContinuationError::Budget(Box::new(error)))?;
    let parallel =
        crate::engine::drive::actions::compute_max_parallel_in_flight(&authority.workflow)
            .map_err(|error| RunContinuationError::Engine(Box::new(error)))?;
    if frame.max_parallel_in_flight != parallel
        || frame.executed > u64::from(budget.max_steps_executable)
    {
        return Err(RunContinuationError::CounterBounds);
    }
    Ok(())
}

pub(super) fn quiescent(
    frame: &vb_core::frame::RunFrameCheckpointV1,
) -> Result<(), RunContinuationError> {
    if frame.parallel_in_flight != 0
        || frame.states.iter().any(|state| {
            !matches!(
                state,
                CheckpointStepState::Pending
                    | CheckpointStepState::Succeeded
                    | CheckpointStepState::Skipped
            )
        })
    {
        return Err(RunContinuationError::InFlight);
    }
    Ok(())
}
