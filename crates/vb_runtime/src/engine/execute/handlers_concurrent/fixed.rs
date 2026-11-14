use super::*;

struct FixedGroupWork<'a> {
    owner: StepIdx,
    branches: &'a [StepIdx],
    join: StepIdx,
    bound: usize,
    first_success: bool,
}

struct FixedGroupProgress {
    output: SlotIdx,
    results: vb_core::ids::ListId,
    taint: Taint,
    completed: usize,
}

pub(super) fn fixed_group(
    ctx: &mut NodeDispatchContext<'_>,
    branches: &[StepIdx],
    join: StepIdx,
    first_success: bool,
) -> RuntimeEngineResult<RuntimeSignal> {
    let owner = ctx.node.id;
    check_lane_bound(ctx.plan, owner, branches.len())?;
    let bound = lane_bound(ctx.plan, branches.len());
    let output = group_output(ctx.plan, join)?;
    let mut progress = group_progress(ctx, output, branches.len())?;
    let work = FixedGroupWork {
        owner,
        branches,
        join,
        bound,
        first_success,
    };
    if let Some(signal) = run_fixed_lanes(ctx, &work, &mut progress)? {
        return Ok(signal);
    }
    finish_fixed_group(ctx, &work, progress)
}

fn group_progress(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
    branch_count: usize,
) -> RuntimeEngineResult<FixedGroupProgress> {
    let (results, taint) = group_cursor(ctx, output)?;
    let completed = ctx
        .store
        .list(results)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if completed > branch_count {
        return Err(invariant("concurrent group cursor exceeds branch count"));
    }
    Ok(FixedGroupProgress {
        output,
        results,
        taint,
        completed,
    })
}

fn group_cursor(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
) -> RuntimeEngineResult<(vb_core::ids::ListId, Taint)> {
    match ctx.run.read_slot(output) {
        Ok(SlotValue::List(list)) => Ok((
            *list,
            ctx.run
                .read_taint(output)
                .map_err(RuntimeEngineError::Core)?,
        )),
        Ok(SlotValue::Null) => initialize_group_cursor(ctx, output),
        Err(EngineError::SlotUninitialized { slot }) if slot == output => {
            initialize_group_cursor(ctx, output)
        }
        Ok(_) => Err(invariant("concurrent group cursor is not a list")),
        Err(error) => Err(RuntimeEngineError::Core(error)),
    }
}

fn initialize_group_cursor(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
) -> RuntimeEngineResult<(vb_core::ids::ListId, Taint)> {
    let empty = ctx
        .store
        .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
        .map_err(RuntimeEngineError::Core)?;
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(empty), Taint::Clean)
        .map_err(RuntimeEngineError::Core)?;
    Ok((empty, Taint::Clean))
}

fn run_fixed_lanes(
    ctx: &mut NodeDispatchContext<'_>,
    work: &FixedGroupWork<'_>,
    progress: &mut FixedGroupProgress,
) -> RuntimeEngineResult<Option<RuntimeSignal>> {
    while progress.completed < work.branches.len() {
        if let Some(signal) = run_next_lane(ctx, work, progress)? {
            return Ok(Some(signal));
        }
    }
    Ok(None)
}

fn run_next_lane(
    ctx: &mut NodeDispatchContext<'_>,
    work: &FixedGroupWork<'_>,
    progress: &mut FixedGroupProgress,
) -> RuntimeEngineResult<Option<RuntimeSignal>> {
    let entry = work
        .branches
        .get(progress.completed)
        .copied()
        .ok_or_else(|| invariant("concurrent branch cursor out of bounds"))?;
    let outcome = super::run_lane(ctx, work.owner, entry, work.bound);
    record_lane_outcome(ctx, work, progress, outcome)
}

fn record_lane_outcome(
    ctx: &mut NodeDispatchContext<'_>,
    work: &FixedGroupWork<'_>,
    progress: &mut FixedGroupProgress,
    outcome: RuntimeEngineResult<LaneOutcome>,
) -> RuntimeEngineResult<Option<RuntimeSignal>> {
    match outcome {
        Ok(LaneOutcome::Exhausted) => {
            ctx.run
                .mark_waiting(work.owner)
                .map_err(RuntimeEngineError::Core)?;
            Ok(Some(RuntimeSignal::StepBudgetExhausted))
        }
        Ok(LaneOutcome::Value(lane)) if work.first_success => {
            let (value, taint) = lane.unwrap_or((SlotValue::Null, Taint::Clean));
            Ok(Some(finish_first_success(ctx, work, value, taint)?))
        }
        Ok(LaneOutcome::Value(value)) => {
            append_continuing_lane(ctx, progress, value)?;
            Ok(None)
        }
        Err(error)
            if work.first_success && ordinary_lane_failure(ctx.plan, ctx.run.pc(), &error) =>
        {
            append_continuing_lane(ctx, progress, None)?;
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn append_continuing_lane(
    ctx: &mut NodeDispatchContext<'_>,
    progress: &mut FixedGroupProgress,
    lane: Option<(SlotValue, Taint)>,
) -> RuntimeEngineResult<()> {
    let (value, taint) = lane.unwrap_or((SlotValue::Null, Taint::Clean));
    progress.taint = join_taint(progress.taint, taint);
    (progress.results, progress.completed) =
        append_group_value(ctx, progress.output, progress.results, value, taint)?;
    Ok(())
}

fn finish_first_success(
    ctx: &mut NodeDispatchContext<'_>,
    work: &FixedGroupWork<'_>,
    value: SlotValue,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    finish_group_value(ctx, work.join, value, taint)
}

fn finish_fixed_group(
    ctx: &mut NodeDispatchContext<'_>,
    work: &FixedGroupWork<'_>,
    progress: FixedGroupProgress,
) -> RuntimeEngineResult<RuntimeSignal> {
    if work.first_success {
        return Err(invariant("first success group had no successful lane"));
    }
    ctx.run
        .write_slot_with_taint(
            progress.output,
            SlotValue::List(progress.results),
            progress.taint,
        )
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, work.join)?;
    Ok(RuntimeSignal::Continue)
}

pub(super) fn append_group_value(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
    current: vb_core::ids::ListId,
    value: SlotValue,
    value_taint: Taint,
) -> RuntimeEngineResult<(vb_core::ids::ListId, usize)> {
    let list = append_list_value(ctx, current, value, value_taint)?;
    write_group_output(ctx, output, list, value_taint)?;
    let completed = ctx
        .store
        .list(list)
        .map_err(RuntimeEngineError::Core)?
        .len();
    Ok((list, completed))
}

fn append_list_value(
    ctx: &mut NodeDispatchContext<'_>,
    current: vb_core::ids::ListId,
    value: SlotValue,
    value_taint: Taint,
) -> RuntimeEngineResult<vb_core::ids::ListId> {
    let (values, taints) = ctx
        .store
        .prepare_list_append(current, Some((value, value_taint)))
        .map_err(RuntimeEngineError::Core)?;
    ctx.store
        .insert_list_with_taint(values, taints)
        .map_err(RuntimeEngineError::Core)
}

fn write_group_output(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
    list: vb_core::ids::ListId,
    value_taint: Taint,
) -> RuntimeEngineResult<()> {
    let output_taint = join_taint(
        ctx.run
            .read_taint(output)
            .map_err(RuntimeEngineError::Core)?,
        value_taint,
    );
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(list), output_taint)
        .map_err(RuntimeEngineError::Core)
}

pub(super) fn finish_group_value(
    ctx: &mut NodeDispatchContext<'_>,
    join: StepIdx,
    value: SlotValue,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    let output = group_output(ctx.plan, join)?;
    ctx.run
        .write_slot_with_taint(output, value, taint)
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, join)?;
    Ok(RuntimeSignal::Continue)
}
