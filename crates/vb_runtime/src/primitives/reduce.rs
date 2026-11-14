#![forbid(unsafe_code)]
//! Reduce accumulation primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use super::helpers::{
    build_iterator_state, decode_iterator_state, expect_list, jump_to, jump_to_next,
    preflight_slot_writes, require_output,
};
mod start;

fn get_initial_value(plan: &CompiledWorkflow, initial: ConstIdx) -> Result<SlotValue, EngineError> {
    plan.constant(initial)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: initial })?
        .to_slot_value()
}

fn get_first_item(items: &[SlotValue]) -> Result<SlotValue, EngineError> {
    items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce items checked nonempty".into(),
        })
}

fn allocate_iterator_state(
    store: &mut ValueStore,
    list_id: ListId,
    cursor: usize,
) -> Result<ListId, EngineError> {
    let state = build_iterator_state(list_id, cursor)?;
    store.insert_list(state)
}

fn get_item_at(source: &[SlotValue], cursor: usize) -> Result<SlotValue, EngineError> {
    source
        .get(cursor)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce cursor bounds-checked above".into(),
        })
}

fn advance_cursor(cursor: usize) -> Result<usize, EngineError> {
    cursor
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce cursor overflow".into(),
        })
}

fn bounded_input(
    run: &RunFrame,
    store: &ValueStore,
    input: SlotIdx,
    limit: u32,
) -> Result<(ListId, Taint), EngineError> {
    let list_id = expect_list(*run.read_slot(input)?)?;
    let input_taint = run.read_taint(input)?;
    let items = store.list(list_id)?;
    let limit = usize::try_from(limit).map_err(|_| EngineError::IterationLimitExceeded {
        resource: "reduce_limit",
    })?;
    if items.len() > limit {
        return Err(EngineError::IterationLimitExceeded {
            resource: "reduce_limit",
        });
    }
    Ok((list_id, input_taint))
}

/// Executes ReduceStart: initializes accumulator from constant pool,
/// reads input list, binds first item, writes remaining tail to
/// iterator slot.
#[allow(clippy::too_many_arguments)]
pub fn reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    start::reduce_start_with_request(
        plan,
        run,
        store,
        start::make_reduce_start_request(input, accumulator, initial, limit, body, done, output),
    )
}

/// Executes ReduceNext: advances iterator, binds next item to
/// iterator slot, or exits when all items consumed.
pub(crate) fn reduce_next_with_policy<P: super::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    _accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let seed = decode_reduce_advance_state(run, store, iterator_slot)?;
    let Some(plan) = prepare_reduce_advance(run, store, seed, output, body)? else {
        return jump_to(run, done);
    };
    commit_reduce_advance::<P>(run, store, plan)
}

struct ReduceAdvanceSeed {
    iterator_slot: SlotIdx,
    iter_taint: Taint,
    source_id: ListId,
    cursor: usize,
}

fn decode_reduce_advance_state(
    run: &RunFrame,
    store: &ValueStore,
    iterator_slot: SlotIdx,
) -> Result<ReduceAdvanceSeed, EngineError> {
    let state_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let iter_taint = run.read_taint(iterator_slot)?;
    let state_items = store.list(state_id)?;
    // Empty state is corruption, not exhaustion; decoder owns this refusal.
    let (source_id, cursor) = decode_iterator_state(state_items)?;
    Ok(ReduceAdvanceSeed {
        iterator_slot,
        iter_taint,
        source_id,
        cursor,
    })
}

fn prepare_reduce_advance(
    run: &RunFrame,
    store: &ValueStore,
    seed: ReduceAdvanceSeed,
    output: Option<SlotIdx>,
    body: StepIdx,
) -> Result<Option<ReduceAdvancePlan>, EngineError> {
    let source = store.list(seed.source_id)?;
    if seed.cursor >= source.len() {
        return Ok(None);
    }
    let item_output = require_output(output, run.pc())?;
    let (item, item_taint) =
        reduce_advance_item(store, source, seed.source_id, seed.cursor, seed.iter_taint)?;
    Ok(Some(ReduceAdvancePlan {
        iterator_slot: seed.iterator_slot,
        item_output,
        item,
        item_taint,
        iter_taint: seed.iter_taint,
        source_id: seed.source_id,
        cursor: seed.cursor,
        body,
    }))
}

fn reduce_advance_item(
    store: &ValueStore,
    source: &[SlotValue],
    source_id: ListId,
    cursor: usize,
    iter_taint: Taint,
) -> Result<(SlotValue, Taint), EngineError> {
    let item = get_item_at(source, cursor)?;
    let item_taint = vb_core::value::join_taint(
        iter_taint,
        store
            .list_item_with_taint(
                source_id,
                u32::try_from(cursor).map_err(|_| EngineError::AllocationFailed)?,
            )?
            .1,
    );
    Ok((item, item_taint))
}

struct ReduceAdvancePlan {
    iterator_slot: SlotIdx,
    item_output: SlotIdx,
    item: SlotValue,
    item_taint: Taint,
    iter_taint: Taint,
    source_id: ListId,
    cursor: usize,
    body: StepIdx,
}

fn commit_reduce_advance<P: super::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: ReduceAdvancePlan,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(run, &[plan.item_output, plan.iterator_slot], plan.body)?;
    let next_cursor = advance_cursor(plan.cursor)?;
    let state_id = allocate_iterator_state(store, plan.source_id, next_cursor)?;
    run.write_slot_with_taint(plan.item_output, plan.item, plan.item_taint)?;
    run.write_slot_with_taint(
        plan.iterator_slot,
        SlotValue::List(state_id),
        plan.iter_taint,
    )?;
    P::jump(run, plan.body)
}

/// Legacy entry-only state reset; this API does not validate a complete region.
pub fn reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    actual_accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    reduce_next_with_policy::<super::body_entry::LegacyEntry>(
        run,
        store,
        iterator_slot,
        actual_accumulator,
        body,
        done,
        output,
    )
}

/// Executes ReduceFinish: writes the final accumulator to output.
pub fn reduce_finish(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let value = *run.read_slot(accumulator)?;
    let taint = run.read_taint(accumulator)?;
    let out = require_output(output, step)?;
    run.write_slot_with_taint(out, value, taint)?;
    jump_to_next(run, next, step)
}

#[cfg(test)]
#[path = "reduce/tests.rs"]
mod tests;
