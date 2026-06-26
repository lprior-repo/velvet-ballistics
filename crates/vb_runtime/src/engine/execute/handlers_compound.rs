#![forbid(unsafe_code)]

//! Compound iterator handlers: ForEach, Together, Collect, Reduce, Repeat.
//! Each `handle_*` helper wraps the matching primitive and converts the
//! resulting core signal into a runtime signal.

use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, FanoutLimit, SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

// ── ForEach handlers ──────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_for_each_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_start(
        run,
        store,
        input,
        item_slot,
        FanoutLimit::new(limit),
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

pub(super) fn handle_for_each_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_next(run, store, iterator_slot, body, done, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_for_each_join(
    run: &mut RunFrame,
    output: SlotIdx,
    node_output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::for_each::for_each_join(run, output, node_output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Together handlers ────────────────────────────────────────────
pub(super) fn handle_together_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branches: &[StepIdx],
    join: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_start(run, store, branches, join, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_together_branch(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch: u16,
    entry: StepIdx,
    join: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_branch(
        run,
        store,
        branch,
        entry,
        join,
        accumulator,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

pub(super) fn handle_together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch_count: u16,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::together::together_join(
        run,
        store,
        branch_count,
        accumulator,
        output,
        next,
        step,
    ) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Collect handlers ─────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_start(
        run, store, cs, source, limit, page_size, body, done, output, None,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

pub(super) fn handle_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_page(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_next(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_collect_finish(
    run: &mut RunFrame,
    cs: &mut CollectStates,
    collector_slot: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::collect::collect_finish(run, cs, collector_slot, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Reduce handlers ──────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_start(
        plan,
        run,
        store,
        input,
        accumulator,
        initial,
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

pub(super) fn handle_reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_next(
        run,
        store,
        iterator_slot,
        accumulator,
        body,
        done,
        output,
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core)
}

pub(super) fn handle_reduce_finish(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::reduce::reduce_finish(run, accumulator, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Repeat handlers ──────────────────────────────────────────────
pub(super) fn handle_repeat_start(
    run: &mut RunFrame,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_start(run, max_attempts, body, done, output)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_attempt(run, attempt_slot, body, done)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_repeat_check(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_check(run, attempt_slot, done, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_repeat_finish(
    run: &mut RunFrame,
    result: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_finish(run, result, output, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}
