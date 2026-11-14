use super::*;

pub(super) fn lane_finish(
    ctx: &mut NodeDispatchContext<'_>,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<LaneStep> {
    let Some(evidence) = ctx.evidence.as_deref_mut() else {
        return Err(invariant("concurrent lane evidence missing"));
    };
    let Some(drive_state) = ctx.drive_state.as_deref_mut() else {
        return Err(invariant("concurrent lane drive state missing"));
    };
    let lane_step = classify_lane_step(step.node, signal)?;
    finish_drive_step(
        ctx.plan,
        &mut *ctx.run,
        evidence,
        &*ctx.collect_states,
        step,
        signal,
        drive_state,
    )?;
    Ok(lane_step)
}

fn classify_lane_step(
    node: &CompiledNode,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<LaneStep> {
    match signal {
        RuntimeSignal::Continue => match &node.kind {
            CompiledNodeKind::ConcurrentLaneEnd { owner, result } => {
                Ok(LaneStep::End(*owner, *result))
            }
            _ => Ok(LaneStep::Continue),
        },
        RuntimeSignal::StepBudgetExhausted => Ok(LaneStep::Exhausted),
        _ => Err(invariant("concurrent lane must be a pure synchronous body")),
    }
}

pub(super) fn run_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    entry: StepIdx,
    bound: usize,
) -> RuntimeEngineResult<LaneOutcome> {
    let drive_state = ctx
        .drive_state
        .as_deref_mut()
        .ok_or_else(|| invariant("concurrent lane drive state missing"))?;
    let previous = drive_state.enter_concurrent_lane(owner);
    let result = run_lane_steps(ctx, owner, entry, bound);
    let drive_state = ctx
        .drive_state
        .as_deref_mut()
        .ok_or_else(|| invariant("concurrent lane drive state missing"))?;
    drive_state.restore_concurrent_lane(previous);
    result
}

fn run_lane_steps(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    entry: StepIdx,
    bound: usize,
) -> RuntimeEngineResult<LaneOutcome> {
    jump(ctx.run, entry)?;
    for _ in 0..bound {
        match super::lane_step(ctx)? {
            LaneStep::Exhausted => return Ok(LaneOutcome::Exhausted),
            LaneStep::Continue => {}
            LaneStep::End(end_owner, result) => {
                if end_owner != owner {
                    return Err(invariant("concurrent lane end owner mismatch"));
                }
                return lane_result(ctx, result);
            }
        }
    }
    Err(invariant("concurrent lane step bound exceeded"))
}

fn lane_result(
    ctx: &mut NodeDispatchContext<'_>,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<LaneOutcome> {
    let Some(slot) = result else {
        return Ok(LaneOutcome::Value(None));
    };
    let value = *ctx.run.read_slot(slot).map_err(RuntimeEngineError::Core)?;
    let taint = ctx.run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
    Ok(LaneOutcome::Value(Some((value, taint))))
}

pub(in crate::engine::execute) fn lane_end(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let start = ctx
        .plan
        .node(owner)
        .ok_or_else(|| invariant("lane end owner missing"))?;
    if is_active_lane(ctx, owner) {
        continue_active_fixed_lane(ctx, start)?;
        return Ok(RuntimeSignal::Continue);
    }
    match &start.kind {
        CompiledNodeKind::ForEachConcurrentStart { .. } => {
            crate::engine::execute::handlers_concurrent_foreach::complete_lane(ctx, owner, result)
        }
        CompiledNodeKind::TogetherConcurrentStart { join, .. } => {
            finish_fixed_lane(ctx, owner, *join, false, result)
        }
        CompiledNodeKind::FirstSuccessStart { join, .. } => {
            finish_fixed_lane(ctx, owner, *join, true, result)
        }
        _ => Err(invariant("lane end owner is not a concurrent start")),
    }
}

fn is_active_lane(ctx: &NodeDispatchContext<'_>, owner: StepIdx) -> bool {
    ctx.drive_state
        .as_deref()
        .and_then(|state| state.active_concurrent_lane())
        == Some(owner)
}

fn continue_active_fixed_lane(
    ctx: &mut NodeDispatchContext<'_>,
    start: &CompiledNode,
) -> RuntimeEngineResult<()> {
    let join = match &start.kind {
        CompiledNodeKind::TogetherConcurrentStart { join, .. }
        | CompiledNodeKind::FirstSuccessStart { join, .. } => *join,
        _ => return Err(invariant("active fixed lane owner changed kind")),
    };
    jump(ctx.run, join)
}

fn finish_fixed_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    join: StepIdx,
    first_success: bool,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let (value, taint) = lane_value(ctx, result)?;
    if first_success {
        return finish_first_success_lane(ctx, owner, join, value, taint);
    }
    finish_together_lane(ctx, owner, join, value, taint)
}

fn lane_value(
    ctx: &mut NodeDispatchContext<'_>,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<(SlotValue, Taint)> {
    match result {
        Some(slot) => Ok((
            *ctx.run.read_slot(slot).map_err(RuntimeEngineError::Core)?,
            ctx.run.read_taint(slot).map_err(RuntimeEngineError::Core)?,
        )),
        None => Ok((SlotValue::Null, Taint::Clean)),
    }
}

fn finish_first_success_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    join: StepIdx,
    value: SlotValue,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    finish_group_value(ctx, join, value, taint)?;
    super::complete_resumed_first_success(ctx, owner)?;
    Ok(RuntimeSignal::Continue)
}

fn finish_together_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    join: StepIdx,
    value: SlotValue,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    let output = group_output(ctx.plan, join)?;
    let current = match *ctx
        .run
        .read_slot(output)
        .map_err(RuntimeEngineError::Core)?
    {
        SlotValue::List(list) => list,
        _ => return Err(invariant("concurrent group cursor is not a list")),
    };
    append_group_value(ctx, output, current, value, taint)?;
    ctx.run
        .mark_waiting(owner)
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, owner)?;
    Ok(RuntimeSignal::Continue)
}
