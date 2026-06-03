#![forbid(unsafe_code)]

//! Node execution handler functions organized by CompiledNodeKind variant.
//! Each handler group maps to a specific node kind family.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, ConstIdx, FanoutLimit, SeqNo, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::action::{
    execute_do, execute_do_without_contract, execute_retry_check, resolve_contract,
};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

pub(super) fn read_attempt_from_slot(
    run: &RunFrame,
    slot: SlotIdx,
) -> RuntimeEngineResult<u16> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v).map_err(|_| {
                RuntimeEngineError::Core(vb_core::errors::EngineError::TypeMismatch {
                    expected: "non-negative u16 attempt count",
                    found: "out-of-range i64",
                })
            }),
            _ => Err(RuntimeEngineError::Core(
                vb_core::errors::EngineError::TypeMismatch {
                    expected: "number",
                    found: value.type_name(),
                },
            )),
        },
        Err(_) => Ok(0),
    }
}

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
        run, store, input, item_slot, FanoutLimit::new(limit), body, done, output,
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
        run, store, branch, entry, join, accumulator, output,
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
        run, store, branch_count, accumulator, output, next, step,
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
        plan, run, store, input, accumulator, initial, body, done, output,
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
        run, store, iterator_slot, accumulator, body, done, output,
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

// ── Wait/Ask handlers ────────────────────────────────────────────

pub(super) fn handle_wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_until(run, deadline_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_wait_event(
    run: &mut RunFrame,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_event(run, event, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_ask(
    run: &mut RunFrame,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask(run, prompt, timeout_slot)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

pub(super) fn handle_ask_resume(
    run: &mut RunFrame,
    answer: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask_resume(run, answer, output, next, step)
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core)
}

// ── Do handler ───────────────────────────────────────────────────

pub(super) fn handle_do(
    run: &mut RunFrame,
    action: ActionId,
    input: SlotIdx,
    contracts: &[ActionContract],
    granted: &CapabilitySet,
    retry_policy: RetryPolicy,
    node_id: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(run.executed());
    if contracts.is_empty() {
        execute_do_without_contract(run, node_id, action, input, seq, granted, retry_policy)
    } else {
        execute_do(
            run,
            node_id,
            action,
            input,
            seq,
            resolve_contract(action, contracts)?,
            contracts,
            granted,
            retry_policy,
        )
    }
}

// ── RetryCheck handler ───────────────────────────────────────────

pub(super) fn handle_retry_check(
    run: &mut RunFrame,
    policy_slot: SlotIdx,
    body: StepIdx,
    exhausted: StepIdx,
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let current_attempt = read_attempt_from_slot(run, policy_slot)?;
    let target = execute_retry_check(current_attempt, retry_policy, body, exhausted);
    run.set_pc(target).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}

// ── ErrorHandler handler ─────────────────────────────────────────

pub(super) fn handle_error_handler(
    run: &mut RunFrame,
    handler_body: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    run.set_pc(handler_body).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}

// ── Core fallback handler ────────────────────────────────────────

pub(super) fn handle_core_step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> RuntimeEngineResult<RuntimeSignal> {
    let cs = vb_core::engine::step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
    Ok(runtime_from_core(cs))
}
