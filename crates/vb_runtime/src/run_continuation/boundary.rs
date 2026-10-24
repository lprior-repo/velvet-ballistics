use super::payload::{Body, Boundary, ContinuationCut};
use super::{ContinuationAuthority, RunContinuationError};
use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::{CompiledNodeKind, SlotIdx, SlotValue, StepIdx, Taint};

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

fn candidate(
    frame: &RunFrameCheckpointV1,
    control: StepIdx,
    authority: &ContinuationAuthority,
) -> Result<Option<Boundary>> {
    let Some(region) = authority.workflow.loop_region(control) else {
        return Ok(None);
    };
    let finish = region.done();
    let node = authority
        .workflow
        .node(finish)
        .ok_or(RunContinuationError::BoundaryState)?;
    if !matches!(node.kind, CompiledNodeKind::CollectFinish { .. }) {
        return Ok(None);
    }
    let exit = node.next.ok_or(RunContinuationError::BoundaryState)?;
    let cut = if frame.pc == finish {
        ContinuationCut::BeforeCollectFinish
    } else if frame.pc == exit {
        ContinuationCut::AfterCollectFinish
    } else {
        return Ok(None);
    };
    Ok(Some(Boundary {
        cut,
        control,
        finish,
        exit,
    }))
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
    equal_slot(&body.frame, collector, expected)?;
    completed_region(&body.frame, boundary, authority)?;
    state(
        &body.frame,
        boundary.control,
        CheckpointStepState::Succeeded,
    )?;
    state(&body.frame, boundary.exit, CheckpointStepState::Pending)?;
    match boundary.cut {
        ContinuationCut::BeforeCollectFinish => {
            state(&body.frame, boundary.finish, CheckpointStepState::Pending)?;
            if body.frame.slots.get(usize::from(output.get())) != Some(&None) {
                return Err(RunContinuationError::BoundaryState);
            }
        }
        ContinuationCut::AfterCollectFinish => {
            state(&body.frame, boundary.finish, CheckpointStepState::Succeeded)?;
            equal_slot(&body.frame, output, expected)?;
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
    let collector = start.output.ok_or(RunContinuationError::BoundaryState)?;
    let output = finish.output.ok_or(RunContinuationError::BoundaryState)?;
    let source = match start.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } if body == region.body() && done == region.done() => {
            source_bounds(authority, source, limit, page_size)?;
            source
        }
        _ => return Err(RunContinuationError::BoundaryState),
    };
    edges(boundary, authority, collector, output)?;
    Ok((source, collector, output))
}

fn edges(
    boundary: Boundary,
    authority: &ContinuationAuthority,
    collector: SlotIdx,
    output: SlotIdx,
) -> Result<()> {
    let workflow = &authority.workflow;
    let region = workflow
        .loop_region(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let control = workflow
        .node(boundary.control)
        .ok_or(RunContinuationError::BoundaryState)?;
    let finish = workflow
        .node(boundary.finish)
        .ok_or(RunContinuationError::BoundaryState)?;
    let exit = workflow
        .node(boundary.exit)
        .ok_or(RunContinuationError::BoundaryState)?;
    if !matches!(control.kind, CompiledNodeKind::CollectNext { collector_slot, body, done }
        if collector_slot == collector && body == region.body() && done == boundary.finish)
        || !matches!(finish.kind, CompiledNodeKind::CollectFinish { collector_slot } if collector_slot == collector)
        || !matches!(exit.kind, CompiledNodeKind::Finish { result } if result == output)
        || region.control() != boundary.control
        || region.done() != boundary.finish
        || finish.next != Some(boundary.exit)
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
