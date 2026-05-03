#![forbid(unsafe_code)]

//! Node execution dispatch for all compiled node kinds.

use vb_core::action::ActionContract;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::action::{
    execute_do, execute_do_without_contract, execute_retry_check, resolve_contract,
};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives;

/// Executes one compiled node with full primitive dispatch.
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => primitives::for_each::for_each_start(
            run,
            store,
            *input,
            *item_slot,
            *limit,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => primitives::for_each::for_each_next(
            run,
            store,
            *iterator_slot,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ForEachJoin { output } => {
            let step = node.id;
            match primitives::for_each::for_each_join(run, *output, node.output, node.next, step) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            primitives::together::together_start(run, store, branches, *join, node.output)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            let step = node.id;
            match primitives::together::together_join(
                run,
                store,
                *branch_count,
                *accumulator,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => primitives::collect::collect_start(
            run,
            store,
            *source,
            *limit,
            *page_size,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => primitives::collect::collect_page(run, store, *collector_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => primitives::collect::collect_next(run, store, *collector_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::CollectFinish { collector_slot } => {
            let step = node.id;
            match primitives::collect::collect_finish(
                run,
                *collector_slot,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => primitives::reduce::reduce_start(
            plan,
            run,
            store,
            *input,
            *accumulator,
            *initial,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => primitives::reduce::reduce_next(
            run,
            store,
            *iterator_slot,
            *accumulator,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ReduceFinish { accumulator } => {
            let step = node.id;
            match primitives::reduce::reduce_finish(run, *accumulator, node.output, node.next, step)
            {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => primitives::repeat::repeat_start(run, *max_attempts, *body, *done, node.output)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => primitives::repeat::repeat_attempt(run, *attempt_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            primitives::repeat::repeat_check(run, *attempt_slot, *done, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::RepeatFinish { result } => {
            primitives::repeat::repeat_finish(run, *result, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            primitives::wait_ask::wait_until(run, *deadline_slot)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => primitives::wait_ask::wait_event(run, *event, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => primitives::wait_ask::ask(run, *prompt, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::AskResume { answer } => {
            primitives::wait_ask::ask_resume(run, *answer, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }
        CompiledNodeKind::Do { action, input } => {
            let seq = SeqNo::new(run.executed());
            if contracts.is_empty() {
                execute_do_without_contract(run, node.id, *action, *input, seq)
            } else {
                execute_do(
                    run,
                    node.id,
                    *action,
                    *input,
                    seq,
                    resolve_contract(*action, contracts)?,
                    contracts,
                )
            }
        }
        CompiledNodeKind::RetryCheck {
            policy_slot: _,
            body,
            exhausted,
        } => {
            let target = execute_retry_check(1, retry_policy, *body, *exhausted);
            run.set_pc(target).map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        CompiledNodeKind::ErrorHandler {
            body: handler_body,
            handler: _,
        } => {
            run.set_pc(*handler_body)
                .map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        _ => {
            let core_signal =
                vb_core::engine::step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
            Ok(runtime_from_core(core_signal))
        }
    }
}
