#![forbid(unsafe_code)]

//! Node execution dispatch handler functions.
//! Each CompiledNodeKind variant has its own handler.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{FanoutLimit, SeqNo, SlotIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::action::{
    execute_do, execute_do_without_contract, execute_retry_check, resolve_contract,
};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<u16> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v).map_err(|_| {
                RuntimeEngineError::Core(vb_core::errors::EngineError::TypeMismatch {
                    expected: "non-negative u16 attempt count",
                    found: "out-of-range i64",
                })
            }),
            _ => Err(RuntimeEngineError::Core(vb_core::errors::EngineError::TypeMismatch {
                expected: "number",
                found: value.type_name(),
            })),
        },
        Err(_) => Ok(0),
    }
}

// ── ForEach handlers ──────────────────────────────────────────────
fn handle_for_each_start(
    run: &mut RunFrame, store: &mut ValueStore,
    input: SlotIdx, item_slot: SlotIdx, limit: u64, body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_start(
        run, store, input, item_slot, FanoutLimit::new(limit), body, done, output,
    ).map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_for_each_next(
    run: &mut RunFrame, store: &mut ValueStore,
    iterator_slot: SlotIdx, body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::for_each::for_each_next(
        run, store, iterator_slot, body, done, output,
    ).map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_for_each_join(
    run: &mut RunFrame, output: Option<SlotIdx>, node_output: Option<SlotIdx>,
    next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::for_each::for_each_join(run, output, node_output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Together handlers ────────────────────────────────────────────
fn handle_together_start(
    run: &mut RunFrame, store: &mut ValueStore, branches: &[u32], join: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_start(run, store, branches, join, output)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_together_branch(
    run: &mut RunFrame, store: &mut ValueStore,
    branch: u32, entry: u32, join: u32, accumulator: SlotIdx, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::together::together_branch(run, store, branch, entry, join, accumulator, output)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_together_join(
    run: &mut RunFrame, store: &mut ValueStore,
    branch_count: u32, accumulator: SlotIdx, output: Option<SlotIdx>, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::together::together_join(run, store, branch_count, accumulator, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Collect handlers ─────────────────────────────────────────────
fn handle_collect_start(
    run: &mut RunFrame, store: &mut ValueStore, cs: &mut CollectStates,
    source: SlotIdx, limit: u64, page_size: u64, body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_start(run, store, cs, source, limit, page_size, body, done, output, None)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_collect_page(
    run: &mut RunFrame, store: &mut ValueStore, cs: &mut CollectStates,
    collector_slot: SlotIdx, body: u32, done: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_page(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_collect_next(
    run: &mut RunFrame, store: &mut ValueStore, cs: &mut CollectStates,
    collector_slot: SlotIdx, body: u32, done: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::collect::collect_next(run, store, cs, collector_slot, body, done)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_collect_finish(
    run: &mut RunFrame, cs: &mut CollectStates,
    collector_slot: SlotIdx, output: Option<SlotIdx>, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::collect::collect_finish(run, cs, collector_slot, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Reduce handlers ──────────────────────────────────────────────
fn handle_reduce_start(
    plan: &CompiledWorkflow, run: &mut RunFrame, store: &mut ValueStore,
    input: SlotIdx, accumulator: SlotIdx, initial: &SlotValue,
    body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_start(plan, run, store, input, accumulator, initial.clone(), body, done, output)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_reduce_next(
    run: &mut RunFrame, store: &mut ValueStore,
    iterator_slot: SlotIdx, accumulator: SlotIdx, body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::reduce::reduce_next(run, store, iterator_slot, accumulator, body, done, output)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_reduce_finish(
    run: &mut RunFrame, accumulator: SlotIdx, output: Option<SlotIdx>, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    match crate::primitives::reduce::reduce_finish(run, accumulator, output, next, step) {
        Ok(signal) => Ok(runtime_from_core(signal)),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}

// ── Repeat handlers ──────────────────────────────────────────────
fn handle_repeat_start(
    run: &mut RunFrame, max_attempts: u32, body: u32, done: u32, output: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_start(run, max_attempts, body, done, output)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_repeat_attempt(
    run: &mut RunFrame, attempt_slot: SlotIdx, body: u32, done: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_attempt(run, attempt_slot, body, done)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_repeat_check(
    run: &mut RunFrame, attempt_slot: SlotIdx, done: u32, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_check(run, attempt_slot, done, next, step)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_repeat_finish(
    run: &mut RunFrame, result: Option<SlotIdx>, output: Option<SlotIdx>, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::repeat::repeat_finish(run, result, output, next, step)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

// ── Wait/Ask handlers ────────────────────────────────────────────
fn handle_wait_until(run: &mut RunFrame, deadline_slot: SlotIdx) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_until(run, deadline_slot)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_wait_event(run: &mut RunFrame, event: u64, timeout_slot: Option<SlotIdx>) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::wait_event(run, event, timeout_slot)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_ask(run: &mut RunFrame, prompt: SlotIdx, timeout_slot: Option<SlotIdx>) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask(run, prompt, timeout_slot)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

fn handle_ask_resume(
    run: &mut RunFrame, answer: SlotIdx, output: Option<SlotIdx>, next: Option<u32>, step: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    crate::primitives::wait_ask::ask_resume(run, answer, output, next, step)
        .map_err(RuntimeEngineError::Core).map(runtime_from_core)
}

// ── Do handler ───────────────────────────────────────────────────
fn handle_do(
    run: &mut RunFrame, action: u64, input: SlotIdx,
    contracts: &[ActionContract], granted: &CapabilitySet,
    retry_policy: RetryPolicy, node_id: u32,
) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(run.executed());
    if contracts.is_empty() {
        execute_do_without_contract(run, node_id, action, input, seq, granted, retry_policy)
    } else {
        execute_do(run, node_id, action, input, seq,
            resolve_contract(action, contracts)?, contracts, granted, retry_policy)
    }
}

// ── RetryCheck handler ───────────────────────────────────────────
fn handle_retry_check(
    run: &mut RunFrame, policy_slot: SlotIdx, body: u32, exhausted: u32, retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let current_attempt = read_attempt_from_slot(run, policy_slot)?;
    let target = execute_retry_check(current_attempt, retry_policy, body, exhausted);
    run.set_pc(target).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}

// ── ErrorHandler handler ─────────────────────────────────────────
fn handle_error_handler(run: &mut RunFrame, handler_body: u32) -> RuntimeEngineResult<RuntimeSignal> {
    run.set_pc(handler_body).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}

// ── Core fallback handler ────────────────────────────────────────
fn handle_core_step_once(
    plan: &CompiledWorkflow, run: &mut RunFrame, store: &mut ValueStore,
) -> RuntimeEngineResult<RuntimeSignal> {
    let cs = vb_core::engine::step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
    Ok(runtime_from_core(cs))
}

// ── Main dispatcher ──────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub fn execute_node_full(
    plan: &CompiledWorkflow, run: &mut RunFrame, store: &mut ValueStore,
    node: &CompiledNode, contracts: &[ActionContract],
    retry_policy: RetryPolicy, collect_states: &mut CollectStates, granted: &CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } =>
            handle_for_each_start(run, store, *input, *item_slot, *limit, *body, *done, node.output),
        CompiledNodeKind::ForEachNext { iterator_slot, body, done } =>
            handle_for_each_next(run, store, *iterator_slot, *body, *done, node.output),
        CompiledNodeKind::ForEachJoin { output } =>
            handle_for_each_join(run, *output, node.output, node.next, node.id),
        CompiledNodeKind::TogetherStart { branches, join } =>
            handle_together_start(run, store, branches, *join, node.output),
        CompiledNodeKind::TogetherBranch { branch, entry, join, accumulator } =>
            handle_together_branch(run, store, *branch, *entry, *join, *accumulator, node.output),
        CompiledNodeKind::TogetherJoin { branch_count, accumulator } =>
            handle_together_join(run, store, *branch_count, *accumulator, node.output, node.next, node.id),
        CompiledNodeKind::CollectStart { source, limit, page_size, body, done } =>
            handle_collect_start(run, store, collect_states, *source, *limit, *page_size, *body, *done, node.output),
        CompiledNodeKind::CollectPage { collector_slot, body, done } =>
            handle_collect_page(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectNext { collector_slot, body, done } =>
            handle_collect_next(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } =>
            handle_collect_finish(run, collect_states, *collector_slot, node.output, node.next, node.id),
        CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done } =>
            handle_reduce_start(plan, run, store, *input, *accumulator, initial, *body, *done, node.output),
        CompiledNodeKind::ReduceNext { iterator_slot, accumulator, body, done } =>
            handle_reduce_next(run, store, *iterator_slot, *accumulator, *body, *done, node.output),
        CompiledNodeKind::ReduceFinish { accumulator } =>
            handle_reduce_finish(run, *accumulator, node.output, node.next, node.id),
        CompiledNodeKind::RepeatStart { max_attempts, body, done } =>
            handle_repeat_start(run, *max_attempts, *body, *done, node.output),
        CompiledNodeKind::RepeatAttempt { attempt_slot, body, done } =>
            handle_repeat_attempt(run, *attempt_slot, *body, *done),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } =>
            handle_repeat_check(run, *attempt_slot, *done, node.next, node.id),
        CompiledNodeKind::RepeatFinish { result } =>
            handle_repeat_finish(run, *result, node.output, node.next, node.id),
        CompiledNodeKind::WaitUntil { deadline_slot } => handle_wait_until(run, *deadline_slot),
        CompiledNodeKind::WaitEvent { event, timeout_slot } => handle_wait_event(run, *event, *timeout_slot),
        CompiledNodeKind::Ask { prompt, timeout_slot } => handle_ask(run, *prompt, *timeout_slot),
        CompiledNodeKind::AskResume { answer } => handle_ask_resume(run, *answer, node.output, node.next, node.id),
        CompiledNodeKind::Do { action, input } => handle_do(run, *action, *input, contracts, granted, retry_policy, node.id),
        CompiledNodeKind::RetryCheck { policy_slot, body, exhausted } =>
            handle_retry_check(run, *policy_slot, *body, *exhausted, retry_policy),
        CompiledNodeKind::ErrorHandler { body: handler_body, .. } => handle_error_handler(run, *handler_body),
        _ => handle_core_step_once(plan, run, store),
    }
}

#[cfg(test)]
mod execute_tests;
