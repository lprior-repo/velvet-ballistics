#![forbid(unsafe_code)]

//! Main node-kind dispatcher. Maps each `CompiledNodeKind` variant to the
//! corresponding per-kind handler helper and threads the matched
//! fields through.
//!
//! Splitting this dispatcher into per-kind helpers (rather than the
//! original 172-line single `match`) brings the on-page function size
//! back under the Holzman rule 6 ceiling while preserving the
//! deterministic dispatch contract.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::budget::handle_retry_check;
use crate::engine::execute::handlers_compound::{
    handle_collect_finish, handle_collect_next, handle_collect_page,
    handle_collect_start, handle_for_each_join, handle_for_each_next,
    handle_for_each_start, handle_reduce_finish, handle_reduce_next,
    handle_reduce_start, handle_repeat_attempt, handle_repeat_check,
    handle_repeat_finish, handle_repeat_start, handle_together_branch,
    handle_together_join, handle_together_start,
};
use crate::engine::execute::handlers_suspend::{
    handle_ask, handle_ask_resume, handle_do, handle_error_handler,
    handle_wait_event, handle_wait_until,
};
use crate::engine::execute::signals::handle_core_step_once;
use crate::engine::types::{RetryPolicy, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

#[allow(clippy::too_many_arguments)]
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    collect_states: &mut CollectStates,
    granted: &CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => handle_for_each_start(
            run,
            store,
            *input,
            *item_slot,
            *limit,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => handle_for_each_next(run, store, *iterator_slot, *body, *done, node.output),
        CompiledNodeKind::ForEachJoin { output } => {
            handle_for_each_join(run, *output, node.output, node.next, node.id)
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            handle_together_start(run, store, branches, *join, node.output)
        }
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => handle_together_branch(
            run,
            store,
            *branch,
            *entry,
            *join,
            *accumulator,
            node.output,
        ),
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => handle_together_join(
            run,
            store,
            *branch_count,
            *accumulator,
            node.output,
            node.next,
            node.id,
        ),
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => handle_collect_start(
            run,
            store,
            collect_states,
            *source,
            *limit,
            *page_size,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => handle_collect_page(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => handle_collect_next(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => handle_collect_finish(
            run,
            collect_states,
            *collector_slot,
            node.output,
            node.next,
            node.id,
        ),
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => handle_reduce_start(
            plan,
            run,
            store,
            *input,
            *accumulator,
            *initial,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => handle_reduce_next(
            run,
            store,
            *iterator_slot,
            *accumulator,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            handle_reduce_finish(run, *accumulator, node.output, node.next, node.id)
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => handle_repeat_start(run, *max_attempts, *body, *done, node.output),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => handle_repeat_attempt(run, *attempt_slot, *body, *done),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            handle_repeat_check(run, *attempt_slot, *done, node.next, node.id)
        }
        CompiledNodeKind::RepeatFinish { result } => {
            handle_repeat_finish(run, *result, node.output, node.next, node.id)
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => handle_wait_until(run, *deadline_slot),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => handle_wait_event(run, *event, *timeout_slot),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => handle_ask(run, *prompt, *timeout_slot),
        CompiledNodeKind::AskResume { answer } => {
            handle_ask_resume(run, *answer, node.output, node.next, node.id)
        }
        CompiledNodeKind::Do { action, input } => handle_do(
            run,
            *action,
            *input,
            contracts,
            granted,
            retry_policy,
            node.id,
        ),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => handle_retry_check(run, *policy_slot, *body, *exhausted, retry_policy),
        CompiledNodeKind::ErrorHandler {
            body: handler_body, ..
        } => handle_error_handler(run, *handler_body),
        _ => handle_core_step_once(plan, run, store),
    }
}