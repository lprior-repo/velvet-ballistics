#![forbid(unsafe_code)]
//! ForEach iteration primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{FanoutLimit, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::{ListId, Taint};

use super::helpers::{
    build_iterator_state, decode_iterator_state, empty_list, expect_list, jump_to, jump_to_next,
    preflight_slot_writes, require_output,
};
mod start;

/// Executes ForEachStart: validates input list, binds first item, and writes
/// the cursor-based iterator state to the output slot.
#[allow(clippy::too_many_arguments)]
pub fn for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: impl Into<FanoutLimit>,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    start::for_each_start_with_request(
        run,
        store,
        ForEachStartRequest {
            input,
            item_slot,
            limit: limit.into(),
            body,
            done,
            output,
        },
    )
}

pub(super) struct ForEachStartRequest {
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
}

fn start_empty_iteration(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iter_output: SlotIdx,
    input_taint: Taint,
    done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(run, &[iter_output], done)?;
    run.write_slot_with_taint(
        iter_output,
        SlotValue::List(store.insert_list(empty_list())?),
        input_taint,
    )?;
    jump_to(run, done)
}

struct ForEachStartPlan {
    list_id: ListId,
    first: SlotValue,
    item_taint: Taint,
    input_taint: Taint,
    item_slot: SlotIdx,
    iter_output: SlotIdx,
    body: StepIdx,
}

fn start_first_iteration(
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: ForEachStartPlan,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(run, &[plan.item_slot, plan.iter_output], plan.body)?;
    let state = build_iterator_state(plan.list_id, 1)?;
    let state_id = store.insert_list(state)?;
    run.write_slot_with_taint(plan.item_slot, plan.first, plan.item_taint)?;
    run.write_slot_with_taint(
        plan.iter_output,
        SlotValue::List(state_id),
        plan.input_taint,
    )?;
    jump_to(run, plan.body)
}

/// Executes ForEachNext: advances the cursor, binds the next item, or exits.
pub(crate) fn for_each_next_with_policy<P: super::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let Some(seed) = decode_for_each_advance_state(run, store, iterator_slot)? else {
        return jump_to(run, done);
    };
    let Some(plan) = prepare_for_each_advance(run, store, seed, output, body)? else {
        return jump_to(run, done);
    };
    commit_for_each_advance::<P>(run, store, plan)
}

struct ForEachAdvanceSeed {
    iterator_slot: SlotIdx,
    iter_taint: Taint,
    source_id: ListId,
    cursor: usize,
}

fn decode_for_each_advance_state(
    run: &RunFrame,
    store: &ValueStore,
    iterator_slot: SlotIdx,
) -> Result<Option<ForEachAdvanceSeed>, EngineError> {
    let state_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let iter_taint = run.read_taint(iterator_slot)?;
    let state_items = store.list(state_id)?;
    if state_items.is_empty() {
        return Ok(None);
    }
    let (source_id, cursor) = decode_iterator_state(state_items)?;
    Ok(Some(ForEachAdvanceSeed {
        iterator_slot,
        iter_taint,
        source_id,
        cursor,
    }))
}

fn prepare_for_each_advance(
    run: &RunFrame,
    store: &ValueStore,
    seed: ForEachAdvanceSeed,
    output: Option<SlotIdx>,
    body: StepIdx,
) -> Result<Option<ForEachAdvancePlan>, EngineError> {
    let source = store.list(seed.source_id)?;
    if seed.cursor >= source.len() {
        return Ok(None);
    }
    let item_output = require_output(output, run.pc())?;
    let (item, item_taint) =
        next_for_each_item(store, seed.source_id, seed.cursor, seed.iter_taint)?;
    Ok(Some(ForEachAdvancePlan {
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

fn next_for_each_item(
    store: &ValueStore,
    source_id: ListId,
    cursor: usize,
    iter_taint: Taint,
) -> Result<(SlotValue, Taint), EngineError> {
    let source = store.list(source_id)?;
    let item = source
        .get(cursor)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "for_each cursor bounds-checked above".into(),
        })?;
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

struct ForEachAdvancePlan {
    iterator_slot: SlotIdx,
    item_output: SlotIdx,
    item: SlotValue,
    item_taint: Taint,
    iter_taint: Taint,
    source_id: ListId,
    cursor: usize,
    body: StepIdx,
}

fn commit_for_each_advance<P: super::body_entry::BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: ForEachAdvancePlan,
) -> Result<vb_core::EngineSignal, EngineError> {
    preflight_slot_writes(run, &[plan.item_output, plan.iterator_slot], plan.body)?;
    let next_cursor =
        plan.cursor
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "for_each cursor overflow".into(),
            })?;
    let next_state = build_iterator_state(plan.source_id, next_cursor)?;
    let next_state_id = store.insert_list(next_state)?;
    run.write_slot_with_taint(plan.item_output, plan.item, plan.item_taint)?;
    run.write_slot_with_taint(
        plan.iterator_slot,
        SlotValue::List(next_state_id),
        plan.iter_taint,
    )?;
    P::jump(run, plan.body)
}

/// Legacy entry-only state reset; this API does not validate a complete region.
pub fn for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    for_each_next_with_policy::<super::body_entry::LegacyEntry>(
        run,
        store,
        iterator_slot,
        body,
        done,
        output,
    )
}

/// Executes ForEachJoin: materializes ordered loop results to the output slot.
pub fn for_each_join(
    run: &mut RunFrame,
    materialized: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let output_slot = require_output(output, step)?;
    let value = *run.read_slot(materialized)?;
    let taint = run.read_taint(materialized)?;
    expect_list(value)?;
    run.write_slot_with_taint(output_slot, value, taint)?;
    jump_to_next(run, next, step)
}

#[cfg(test)]
#[path = "../for_each_tests.rs"]
mod tests;
