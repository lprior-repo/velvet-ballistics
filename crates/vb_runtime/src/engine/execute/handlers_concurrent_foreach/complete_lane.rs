use super::*;

#[derive(Clone, Copy)]
struct ForEachSpec {
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    body: StepIdx,
    join: StepIdx,
}

struct ForEachLane {
    spec: ForEachSpec,
    source: vb_core::ids::ListId,
    count: usize,
    output: SlotIdx,
    current: vb_core::ids::ListId,
    completed: usize,
}

pub(in crate::engine::execute) fn complete_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let lane = lane_context(ctx, owner)?;
    let next = append_lane_result(ctx, &lane, result)?;
    advance_lane(ctx, lane, next)?;
    Ok(RuntimeSignal::Continue)
}

fn lane_context(ctx: &NodeDispatchContext<'_>, owner: StepIdx) -> RuntimeEngineResult<ForEachLane> {
    let spec = foreach_spec(ctx.plan, owner)?;
    let (source, count) = source_list(ctx, spec)?;
    let (output, current, completed) = partial_result(ctx, spec.join, count)?;
    Ok(ForEachLane {
        spec,
        source,
        count,
        output,
        current,
        completed,
    })
}

fn foreach_spec(plan: &CompiledWorkflow, owner: StepIdx) -> RuntimeEngineResult<ForEachSpec> {
    match plan.node(owner).map(|node| &node.kind) {
        Some(CompiledNodeKind::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            body,
            join,
            ..
        }) => Ok(ForEachSpec {
            input: *input,
            item_slot: *item_slot,
            limit: *limit,
            body: *body,
            join: *join,
        }),
        _ => Err(invariant("for_each lane owner is not a for_each start")),
    }
}

fn source_list(
    ctx: &NodeDispatchContext<'_>,
    spec: ForEachSpec,
) -> RuntimeEngineResult<(vb_core::ids::ListId, usize)> {
    let source = match *ctx
        .run
        .read_slot(spec.input)
        .map_err(RuntimeEngineError::Core)?
    {
        SlotValue::List(id) => id,
        _ => return Err(invariant("concurrent for_each input must be a list")),
    };
    let count = ctx
        .store
        .list(source)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if count > spec.limit.as_usize() {
        return Err(RuntimeEngineError::Core(
            EngineError::IterationLimitExceeded {
                resource: "for_each_limit",
            },
        ));
    }
    Ok((source, count))
}

fn partial_result(
    ctx: &NodeDispatchContext<'_>,
    join: StepIdx,
    count: usize,
) -> RuntimeEngineResult<(SlotIdx, vb_core::ids::ListId, usize)> {
    let output = group_output(ctx.plan, join)?;
    let current = match *ctx
        .run
        .read_slot(output)
        .map_err(RuntimeEngineError::Core)?
    {
        SlotValue::List(id) => id,
        _ => return Err(invariant("for_each partial result is not a list")),
    };
    let completed = ctx
        .store
        .list(current)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if completed >= count {
        return Err(invariant("for_each lane completed beyond input length"));
    }
    Ok((output, current, completed))
}

fn append_lane_result(
    ctx: &mut NodeDispatchContext<'_>,
    lane: &ForEachLane,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<usize> {
    let (value, taint) = lane_value(ctx, result)?;
    append_lane_output(ctx, lane, value, taint)?;
    lane.completed
        .checked_add(1)
        .ok_or_else(|| invariant("for_each cursor overflow"))
}

fn lane_value(
    ctx: &NodeDispatchContext<'_>,
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

fn append_lane_output(
    ctx: &mut NodeDispatchContext<'_>,
    lane: &ForEachLane,
    value: SlotValue,
    lane_taint: Taint,
) -> RuntimeEngineResult<()> {
    let (values, taints) = ctx
        .store
        .prepare_list_append(lane.current, Some((value, lane_taint)))
        .map_err(RuntimeEngineError::Core)?;
    let list = ctx
        .store
        .insert_list_with_taint(values, taints)
        .map_err(RuntimeEngineError::Core)?;
    let output_taint = join_taint(
        ctx.run
            .read_taint(lane.output)
            .map_err(RuntimeEngineError::Core)?,
        lane_taint,
    );
    ctx.run
        .write_slot_with_taint(lane.output, SlotValue::List(list), output_taint)
        .map_err(RuntimeEngineError::Core)
}

fn advance_lane(
    ctx: &mut NodeDispatchContext<'_>,
    lane: ForEachLane,
    next: usize,
) -> RuntimeEngineResult<()> {
    if next == lane.count {
        return jump(ctx.run, lane.spec.join);
    }
    let (item, item_taint) = item_value(ctx.store, lane.source, next)?;
    let input_taint = ctx
        .run
        .read_taint(lane.spec.input)
        .map_err(RuntimeEngineError::Core)?;
    ctx.run
        .write_slot_with_taint(
            lane.spec.item_slot,
            item,
            join_taint(input_taint, item_taint),
        )
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, lane.spec.body)
}
