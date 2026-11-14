use super::*;

pub(super) fn for_each_start_with_request(
    run: &mut RunFrame,
    store: &mut ValueStore,
    request: ForEachStartRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    let list_id = expect_list(*run.read_slot(request.input)?)?;
    let input_taint = run.read_taint(request.input)?;
    let items = store.list(list_id)?;
    validate_for_each_limit(items.len(), request.limit)?;
    let iter_output = require_output(request.output, run.pc())?;
    if items.is_empty() {
        return start_empty_iteration(run, store, iter_output, input_taint, request.done);
    }
    let plan = first_iteration_plan(request, store, list_id, input_taint, iter_output, items)?;
    start_first_iteration(run, store, plan)
}

fn validate_for_each_limit(item_count: usize, limit: FanoutLimit) -> Result<(), EngineError> {
    if item_count > limit.as_usize() {
        return Err(EngineError::IterationLimitExceeded {
            resource: "for_each_limit",
        });
    }
    Ok(())
}

fn first_iteration_plan(
    request: ForEachStartRequest,
    store: &ValueStore,
    list_id: ListId,
    input_taint: Taint,
    iter_output: SlotIdx,
    items: &[SlotValue],
) -> Result<ForEachStartPlan, EngineError> {
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each item_count checked nonzero".into(),
        })?;
    let item_taint =
        vb_core::value::join_taint(input_taint, store.list_item_with_taint(list_id, 0)?.1);
    Ok(ForEachStartPlan {
        list_id,
        first,
        item_taint,
        input_taint,
        item_slot: request.item_slot,
        iter_output,
        body: request.body,
    })
}
