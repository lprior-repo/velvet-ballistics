use super::*;

pub(super) struct ReduceStartRequest {
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
}

pub(super) fn make_reduce_start_request(
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> ReduceStartRequest {
    ReduceStartRequest {
        input,
        accumulator,
        initial,
        limit,
        body,
        done,
        output,
    }
}

pub(super) fn reduce_start_with_request(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    request: ReduceStartRequest,
) -> Result<vb_core::EngineSignal, EngineError> {
    let (list_id, input_taint) = bounded_input(run, store, request.input, request.limit)?;
    let init_value = get_initial_value(plan, request.initial)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return finish_empty_reduce(run, request.accumulator, init_value, request.done);
    }
    let iter_output = require_output(request.output, run.pc())?;
    let first = get_first_item(items)?;
    start_nonempty_reduce_from_preparation(
        run,
        store,
        ReduceStartPreparation {
            request,
            list_id,
            input_taint,
            init_value,
            iter_output,
            first,
        },
    )
}

struct ReduceStartPreparation {
    request: ReduceStartRequest,
    list_id: ListId,
    input_taint: Taint,
    init_value: SlotValue,
    iter_output: SlotIdx,
    first: SlotValue,
}

fn start_nonempty_reduce_from_preparation(
    run: &mut RunFrame,
    store: &mut ValueStore,
    prepared: ReduceStartPreparation,
) -> Result<vb_core::EngineSignal, EngineError> {
    let item_taint = vb_core::value::join_taint(
        prepared.input_taint,
        store.list_item_with_taint(prepared.list_id, 0)?.1,
    );
    start_nonempty_reduce(
        run,
        store,
        ReduceStartPlan {
            input: prepared.request.input,
            accumulator: prepared.request.accumulator,
            iter_output: prepared.iter_output,
            init_value: prepared.init_value,
            first: prepared.first,
            item_taint,
            input_taint: prepared.input_taint,
            list_id: prepared.list_id,
            body: prepared.request.body,
        },
    )
}

fn finish_empty_reduce(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    init_value: SlotValue,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(run, &[accumulator], done)?;
    run.write_slot(accumulator, init_value)?;
    jump_to(run, done)
}

struct ReduceStartPlan {
    input: SlotIdx,
    accumulator: SlotIdx,
    iter_output: SlotIdx,
    init_value: SlotValue,
    first: SlotValue,
    item_taint: Taint,
    input_taint: Taint,
    list_id: ListId,
    body: StepIdx,
}

fn start_nonempty_reduce(
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: ReduceStartPlan,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(
        run,
        &[plan.accumulator, plan.input, plan.iter_output],
        plan.body,
    )?;
    let state_id = allocate_iterator_state(store, plan.list_id, 1)?;
    run.write_slot(plan.accumulator, plan.init_value)?;
    run.write_slot_with_taint(plan.input, plan.first, plan.item_taint)?;
    run.write_slot_with_taint(
        plan.iter_output,
        SlotValue::List(state_id),
        plan.input_taint,
    )?;
    jump_to(run, plan.body)
}
