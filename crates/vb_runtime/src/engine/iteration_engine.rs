#![forbid(unsafe_code)]

//! Iteration/compound primitive node execution handlers.

use vb_core::action::ActionContract;
use vb_core::engine::{step_once, EngineSignal};
use vb_core::frame::RunFrame;
use vb_core::ids::SeqNo;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::signals::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::engine::RetryPolicy;

/// Handles ForEach, Together, Collect, and Reduce node variants.
pub fn execute_iteration_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
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

        _ => Err(RuntimeEngineError::Core(
            EngineError::UnsupportedPrimitive {
                primitive: "not_an_iteration_node",
            },
        )),
    }
}

use vb_core::action::{ActionId, ActionTicket};
use vb_core::errors::EngineError;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::Taint;

#[allow(clippy::needless_pass_by_value)]
pub fn runtime_from_core(signal: EngineSignal) -> RuntimeSignal {
    match signal {
        EngineSignal::Continue => RuntimeSignal::Continue,
        EngineSignal::Finished(value, _taint) => RuntimeSignal::Finished(value),
        EngineSignal::StepBudgetExhausted => RuntimeSignal::StepBudgetExhausted,
        EngineSignal::AwaitingAction => RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::ZERO,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        }),
        EngineSignal::AwaitingWait => RuntimeSignal::AwaitingWait,
        EngineSignal::AwaitingAsk => RuntimeSignal::AwaitingAsk,
    }
}
