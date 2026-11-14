use super::payload::{Body, Boundary, ContinuationCut};
use super::{ContinuationAuthority, RunContinuationError};
use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::workflow::NodeContinuation;
use vb_core::{CompiledNodeKind, SlotIdx, SlotValue, StepIdx, Taint};

#[path = "boundary/candidate.rs"]
mod candidate_cut;
use candidate_cut::candidate;

type Result<T> = core::result::Result<T, RunContinuationError>;

pub(super) fn locate(
    frame: &RunFrameCheckpointV1,
    authority: &ContinuationAuthority,
) -> Result<Boundary> {
    let mut found = None;
    for raw in 0..authority.workflow.node_count() {
        let Some(boundary) = candidate(frame, StepIdx::new(raw), authority)? else {
            continue;
        };
        if found.replace(boundary).is_some() {
            return Err(RunContinuationError::UnsupportedBoundary);
        }
    }
    found.ok_or(RunContinuationError::UnsupportedBoundary)
}

pub(super) fn validate(body: &Body, authority: &ContinuationAuthority) -> Result<()> {
    let boundary = body.boundary;
    let (source, collector, output) = shape(boundary, authority)?;
    let expected = authority
        .original
        .root(source)
        .ok_or(RunContinuationError::BoundaryState)?;
    if !matches!(expected.0, SlotValue::List(_))
        || source == collector
        || output == collector
        || output == source
    {
        return Err(RunContinuationError::BoundaryState);
    }
    validate_completed_boundary(&body.frame, boundary, authority, collector, expected)?;
    validate_cut_output(&body.frame, boundary, output, expected)
}

fn validate_completed_boundary(
    frame: &RunFrameCheckpointV1,
    boundary: Boundary,
    authority: &ContinuationAuthority,
    collector: SlotIdx,
    expected: (SlotValue, Taint),
) -> Result<()> {
    equal_slot(frame, collector, expected)?;
    completed_region(frame, boundary, authority)?;
    state(frame, boundary.control, CheckpointStepState::Succeeded)?;
    state(frame, boundary.exit, CheckpointStepState::Pending)
}

fn validate_cut_output(
    frame: &RunFrameCheckpointV1,
    boundary: Boundary,
    output: SlotIdx,
    expected: (SlotValue, Taint),
) -> Result<()> {
    match boundary.cut {
        ContinuationCut::BeforeCollectFinish => {
            state(frame, boundary.finish, CheckpointStepState::Pending)?;
            if frame.slots.get(usize::from(output.get())) != Some(&None) {
                return Err(RunContinuationError::BoundaryState);
            }
        }
        ContinuationCut::AfterCollectFinish => {
            state(frame, boundary.finish, CheckpointStepState::Succeeded)?;
            equal_slot(frame, output, expected)?;
        }
    }
    Ok(())
}

fn shape(
    boundary: Boundary,
    authority: &ContinuationAuthority,
) -> Result<(SlotIdx, SlotIdx, SlotIdx)> {
    let workflow = &authority.workflow;
    let region = workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let start = workflow
        .node(region.start())
        .ok_or(RunContinuationError::BoundaryState)?;
    let finish = workflow
        .node(boundary.finish)
        .ok_or(RunContinuationError::BoundaryState)?;
    let collector = start
        .output
        .slot()
        .ok_or(RunContinuationError::BoundaryState)?;
    let output = finish
        .output
        .slot()
        .ok_or(RunContinuationError::BoundaryState)?;
    let source = collect_source(boundary, authority)?;
    edges(boundary, authority, collector, output)?;
    Ok((source, collector, output))
}

fn collect_source(boundary: Boundary, authority: &ContinuationAuthority) -> Result<SlotIdx> {
    let workflow = &authority.workflow;
    let region = workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let start = workflow
        .node(region.start())
        .ok_or(RunContinuationError::BoundaryState)?;
    match start.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } if body == region.body() && done == region.done() => {
            source_bounds(authority, source, limit.get(), page_size)?;
            Ok(source)
        }
        _ => Err(RunContinuationError::BoundaryState),
    }
}

fn edges(
    boundary: Boundary,
    authority: &ContinuationAuthority,
    collector: SlotIdx,
    output: SlotIdx,
) -> Result<()> {
    validate_collect_next_kind(boundary, authority, collector)?;
    validate_finish_exit_kinds(boundary, authority, collector, output)?;
    validate_region_edges(boundary, authority)
}

fn validate_collect_next_kind(
    boundary: Boundary,
    authority: &ContinuationAuthority,
    collector: SlotIdx,
) -> Result<()> {
    let workflow = &authority.workflow;
    let region = workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let control = workflow
        .node(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    if !matches!(control.kind, CompiledNodeKind::CollectNext { collector_slot, body, done }
        if collector_slot == collector && body == region.body() && done == boundary.finish)
    {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}

fn validate_finish_exit_kinds(
    boundary: Boundary,
    authority: &ContinuationAuthority,
    collector: SlotIdx,
    output: SlotIdx,
) -> Result<()> {
    let workflow = &authority.workflow;
    let finish = workflow
        .node(boundary.finish)
        .ok_or(RunContinuationError::BoundaryState)?;
    let exit = workflow
        .node(boundary.exit)
        .ok_or(RunContinuationError::BoundaryState)?;
    if !matches!(finish.kind, CompiledNodeKind::CollectFinish { collector_slot } if collector_slot == collector)
        || !matches!(exit.kind, CompiledNodeKind::Finish { result } if result == output)
    {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}

fn validate_region_edges(boundary: Boundary, authority: &ContinuationAuthority) -> Result<()> {
    let workflow = &authority.workflow;
    let region = workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let finish = workflow
        .node(boundary.finish)
        .ok_or(RunContinuationError::BoundaryState)?;
    if region.control() != boundary.control
        || region.done() != boundary.finish
        || finish.routing.next != NodeContinuation::Step(boundary.exit)
    {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}

fn completed_region(
    frame: &RunFrameCheckpointV1,
    boundary: Boundary,
    authority: &ContinuationAuthority,
) -> Result<()> {
    let region = authority
        .workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    state(frame, region.start(), CheckpointStepState::Succeeded)?;
    for raw in region.body().get()..=region.control().get() {
        if !matches!(
            frame.states.get(usize::from(raw)),
            Some(CheckpointStepState::Succeeded | CheckpointStepState::Skipped)
        ) {
            return Err(RunContinuationError::BoundaryState);
        }
    }
    Ok(())
}

fn state(frame: &RunFrameCheckpointV1, step: StepIdx, expected: CheckpointStepState) -> Result<()> {
    if frame.states.get(usize::from(step.get())) != Some(&expected) {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}

fn equal_slot(
    frame: &RunFrameCheckpointV1,
    slot: SlotIdx,
    expected: (SlotValue, Taint),
) -> Result<()> {
    let index = usize::from(slot.get());
    if frame.slots.get(index) != Some(&Some(expected.0))
        || frame.taint.get(index) != Some(&expected.1)
    {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}

fn source_bounds(
    authority: &ContinuationAuthority,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
) -> Result<()> {
    let Some((SlotValue::List(id), _)) = authority.original.root(source) else {
        return Err(RunContinuationError::BoundaryState);
    };
    let index = usize::try_from(id.get()).map_err(|_| RunContinuationError::BoundaryState)?;
    let values = authority
        .original
        .checkpoint()
        .lists
        .get(index)
        .ok_or(RunContinuationError::BoundaryState)?;
    let len = u32::try_from(values.len()).map_err(|_| RunContinuationError::BoundaryState)?;
    if len == 0 || len > limit || page_size == 0 {
        return Err(RunContinuationError::BoundaryState);
    }
    Ok(())
}
