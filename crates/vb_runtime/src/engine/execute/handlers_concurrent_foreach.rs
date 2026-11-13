#![forbid(unsafe_code)]

use vb_core::errors::EngineError;
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint, join_taint};
use vb_core::workflow::CompiledNodeKind;

use crate::engine::execute::NodeDispatchContext;
use crate::engine::execute::concurrent_support::{
    check_lane_bound, group_output, invariant, item_value, jump,
};
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

pub(super) fn start(
    ctx: &mut NodeDispatchContext<'_>,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    body: StepIdx,
    join: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let list_id = match *ctx.run.read_slot(input).map_err(RuntimeEngineError::Core)? {
        SlotValue::List(id) => id,
        _ => return Err(invariant("concurrent for_each input must be a list")),
    };
    let count = ctx
        .store
        .list(list_id)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if count > limit.as_usize() {
        return Err(RuntimeEngineError::Core(
            EngineError::IterationLimitExceeded {
                resource: "for_each_limit",
            },
        ));
    }
    let capacity = usize::from(at_once.get()).min(count).min(limit.as_usize());
    check_lane_bound(ctx.plan, ctx.node.id, capacity)?;
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
    if count == 0 {
        jump(ctx.run, join)?;
    } else {
        let (value, item_taint) = item_value(ctx.store, list_id, 0)?;
        ctx.run
            .write_slot_with_taint(item_slot, value, join_taint(taint, item_taint))
            .map_err(RuntimeEngineError::Core)?;
        jump(ctx.run, body)?;
    }
    Ok(RuntimeSignal::Continue)
}

pub(super) fn complete_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let (input, item_slot, limit, body, join) = match ctx.plan.node(owner).map(|node| &node.kind) {
        Some(CompiledNodeKind::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            body,
            join,
            ..
        }) => (*input, *item_slot, *limit, *body, *join),
        _ => return Err(invariant("for_each lane owner is not a for_each start")),
    };
    let source = match *ctx.run.read_slot(input).map_err(RuntimeEngineError::Core)? {
        SlotValue::List(id) => id,
        _ => return Err(invariant("concurrent for_each input must be a list")),
    };
    let count = ctx
        .store
        .list(source)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if count > limit.as_usize() {
        return Err(RuntimeEngineError::Core(
            EngineError::IterationLimitExceeded {
                resource: "for_each_limit",
            },
        ));
    }
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
    let (value, lane_taint) = match result {
        Some(slot) => (
            *ctx.run.read_slot(slot).map_err(RuntimeEngineError::Core)?,
            ctx.run.read_taint(slot).map_err(RuntimeEngineError::Core)?,
        ),
        None => (SlotValue::Null, Taint::Clean),
    };
    let (values, taints) = ctx
        .store
        .prepare_list_append(current, Some((value, lane_taint)))
        .map_err(RuntimeEngineError::Core)?;
    let list = ctx
        .store
        .insert_list_with_taint(values, taints)
        .map_err(RuntimeEngineError::Core)?;
    let output_taint = join_taint(
        ctx.run
            .read_taint(output)
            .map_err(RuntimeEngineError::Core)?,
        lane_taint,
    );
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(list), output_taint)
        .map_err(RuntimeEngineError::Core)?;
    let next = completed
        .checked_add(1)
        .ok_or_else(|| invariant("for_each cursor overflow"))?;
    if next == count {
        jump(ctx.run, join)?;
    } else {
        let (item, item_taint) = item_value(ctx.store, source, next)?;
        let input_taint = ctx
            .run
            .read_taint(input)
            .map_err(RuntimeEngineError::Core)?;
        ctx.run
            .write_slot_with_taint(item_slot, item, join_taint(input_taint, item_taint))
            .map_err(RuntimeEngineError::Core)?;
        jump(ctx.run, body)?;
    }
    Ok(RuntimeSignal::Continue)
}
