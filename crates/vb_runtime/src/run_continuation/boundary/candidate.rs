use super::*;

pub(super) fn candidate(
    frame: &RunFrameCheckpointV1,
    control: StepIdx,
    authority: &ContinuationAuthority,
) -> Result<Option<Boundary>> {
    let Some((finish, exit)) = candidate_finish_exit(control, authority)? else {
        return Ok(None);
    };
    let Some(cut) = candidate_cut(frame, finish, exit) else {
        return Ok(None);
    };
    Ok(Some(Boundary {
        cut,
        control,
        finish,
        exit,
    }))
}

fn candidate_finish_exit(
    control: StepIdx,
    authority: &ContinuationAuthority,
) -> Result<Option<(StepIdx, StepIdx)>> {
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
    let exit = node
        .routing
        .next
        .step()
        .ok_or(RunContinuationError::BoundaryState)?;
    Ok(Some((finish, exit)))
}

pub(super) fn candidate_cut(
    frame: &RunFrameCheckpointV1,
    finish: StepIdx,
    exit: StepIdx,
) -> Option<ContinuationCut> {
    if frame.pc == finish {
        Some(ContinuationCut::BeforeCollectFinish)
    } else if frame.pc == exit {
        Some(ContinuationCut::AfterCollectFinish)
    } else {
        None
    }
}
