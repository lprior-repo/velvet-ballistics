use super::*;

#[derive(Clone, Copy)]
struct ForEachStart {
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    body: StepIdx,
    join: StepIdx,
}

#[derive(Clone, Copy)]
struct ForEachSource {
    list: vb_core::ids::ListId,
    count: usize,
}

pub(in crate::engine::execute) fn start(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let start = start_config(ctx.node)?;
    let source = checked_source(ctx, start)?;
    let capacity = usize::from(start.at_once.get())
        .min(source.count)
        .min(start.limit.as_usize());
    check_lane_bound(ctx.plan, ctx.node.id, capacity)?;
    let taint = initialize_output(ctx, start.input, start.join)?;
    if source.count == 0 {
        jump(ctx.run, start.join)?;
    } else {
        enter_first_item(ctx, start, source.list, taint)?;
    }
    Ok(RuntimeSignal::Continue)
}

fn start_config(node: &CompiledNode) -> RuntimeEngineResult<ForEachStart> {
    match &node.kind {
        CompiledNodeKind::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            at_once,
            body,
            join,
        } => Ok(ForEachStart {
            input: *input,
            item_slot: *item_slot,
            limit: *limit,
            at_once: *at_once,
            body: *body,
            join: *join,
        }),
        _ => Err(invariant(
            "concurrent for_each start has the wrong node kind",
        )),
    }
}

fn checked_source(
    ctx: &NodeDispatchContext<'_>,
    start: ForEachStart,
) -> RuntimeEngineResult<ForEachSource> {
    let list = match *ctx
        .run
        .read_slot(start.input)
        .map_err(RuntimeEngineError::Core)?
    {
        SlotValue::List(list) => list,
        _ => return Err(invariant("concurrent for_each input must be a list")),
    };
    let count = ctx
        .store
        .list(list)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if count > start.limit.as_usize() {
        return Err(RuntimeEngineError::Core(
            EngineError::IterationLimitExceeded {
                resource: "for_each_limit",
            },
        ));
    }
    Ok(ForEachSource { list, count })
}

fn initialize_output(
    ctx: &mut NodeDispatchContext<'_>,
    input: SlotIdx,
    join: StepIdx,
) -> RuntimeEngineResult<Taint> {
    let output = group_output(ctx.plan, join)?;
    let empty = ctx
        .store
        .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
        .map_err(RuntimeEngineError::Core)?;
    let taint = ctx
        .run
        .read_taint(input)
        .map_err(RuntimeEngineError::Core)?;
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(empty), taint)
        .map_err(RuntimeEngineError::Core)?;
    Ok(taint)
}

fn enter_first_item(
    ctx: &mut NodeDispatchContext<'_>,
    start: ForEachStart,
    source: vb_core::ids::ListId,
    input_taint: Taint,
) -> RuntimeEngineResult<()> {
    let (value, item_taint) = item_value(ctx.store, source, 0)?;
    ctx.run
        .write_slot_with_taint(start.item_slot, value, join_taint(input_taint, item_taint))
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, start.body)
}
