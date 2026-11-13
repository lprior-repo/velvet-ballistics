use super::*;

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
    let (mut results, mut taint) = match ctx.run.read_slot(output) {
        Ok(SlotValue::List(list)) => (
            *list,
            ctx.run
                .read_taint(output)
                .map_err(RuntimeEngineError::Core)?,
        ),
        Ok(SlotValue::Null) => {
            let empty = ctx
                .store
                .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
                .map_err(RuntimeEngineError::Core)?;
            ctx.run
                .write_slot_with_taint(output, SlotValue::List(empty), Taint::Clean)
                .map_err(RuntimeEngineError::Core)?;
            (empty, Taint::Clean)
        }
        Err(EngineError::SlotUninitialized { slot }) if slot == output => {
            let empty = ctx
                .store
                .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
                .map_err(RuntimeEngineError::Core)?;
            ctx.run
                .write_slot_with_taint(output, SlotValue::List(empty), Taint::Clean)
                .map_err(RuntimeEngineError::Core)?;
            (empty, Taint::Clean)
        }
        Ok(_) => return Err(invariant("concurrent group cursor is not a list")),
        Err(error) => return Err(RuntimeEngineError::Core(error)),
    };
    let mut completed = ctx
        .store
        .list(results)
        .map_err(RuntimeEngineError::Core)?
        .len();
    if completed > branches.len() {
        return Err(invariant("concurrent group cursor exceeds branch count"));
    }
    while completed < branches.len() {
        let entry = branches
            .get(completed)
            .copied()
            .ok_or_else(|| invariant("concurrent branch cursor out of bounds"))?;
        match super::run_lane(ctx, owner, entry, bound) {
            Ok(LaneOutcome::Exhausted) => {
                ctx.run
                    .mark_waiting(owner)
                    .map_err(RuntimeEngineError::Core)?;
                return Ok(RuntimeSignal::StepBudgetExhausted);
            }
            Ok(LaneOutcome::Value(lane)) if first_success => {
                let (value, lane_taint) = lane.unwrap_or((SlotValue::Null, Taint::Clean));
                return finish_group_value(ctx, join, value, lane_taint);
            }
            Ok(LaneOutcome::Value(Some((value, lane_taint)))) => {
                taint = join_taint(taint, lane_taint);
                (results, completed) = append_group_value(ctx, output, results, value, lane_taint)?;
            }
            Ok(LaneOutcome::Value(None)) => {
                (results, completed) =
                    append_group_value(ctx, output, results, SlotValue::Null, Taint::Clean)?;
            }
            Err(error)
                if first_success && ordinary_lane_failure(ctx.plan, ctx.run.pc(), &error) =>
            {
                (results, completed) =
                    append_group_value(ctx, output, results, SlotValue::Null, Taint::Clean)?;
            }
            Err(error) => return Err(error),
        }
    }
    if first_success {
        return Err(invariant("first success group had no successful lane"));
    }
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(results), taint)
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, join)?;
    Ok(RuntimeSignal::Continue)
}

pub(super) fn append_group_value(
    ctx: &mut NodeDispatchContext<'_>,
    output: SlotIdx,
    current: vb_core::ids::ListId,
    value: SlotValue,
    value_taint: Taint,
) -> RuntimeEngineResult<(vb_core::ids::ListId, usize)> {
    let (values, taints) = ctx
        .store
        .prepare_list_append(current, Some((value, value_taint)))
        .map_err(RuntimeEngineError::Core)?;
    let list = ctx
        .store
        .insert_list_with_taint(values, taints)
        .map_err(RuntimeEngineError::Core)?;
    let output_taint = join_taint(
        ctx.run
            .read_taint(output)
            .map_err(RuntimeEngineError::Core)?,
        value_taint,
    );
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(list), output_taint)
        .map_err(RuntimeEngineError::Core)?;
    let completed = ctx
        .store
        .list(list)
        .map_err(RuntimeEngineError::Core)?
        .len();
    Ok((list, completed))
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
