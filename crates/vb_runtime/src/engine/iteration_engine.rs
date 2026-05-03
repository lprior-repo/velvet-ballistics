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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::engine::EngineSignal;
    use vb_core::ids::{RunId, SeqNo, SlotIdx};
    use vb_core::value::SlotValue;
    use vb_core::value::Taint;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};
    use vb_core::frame::RunFrame;
    use vb_core::value_store::ValueStore;

    // =====================================================================
    // runtime_from_core signal mapping
    // =====================================================================

    #[test]
    fn runtime_from_core_continue() {
        let result = runtime_from_core(EngineSignal::Continue);
        assert_eq!(result, RuntimeSignal::Continue);
    }

    #[test]
    fn runtime_from_core_step_budget_exhausted() {
        let result = runtime_from_core(EngineSignal::StepBudgetExhausted);
        assert_eq!(result, RuntimeSignal::StepBudgetExhausted);
    }

    #[test]
    fn runtime_from_core_awaiting_wait() {
        let result = runtime_from_core(EngineSignal::AwaitingWait);
        assert_eq!(result, RuntimeSignal::AwaitingWait);
    }

    #[test]
    fn runtime_from_core_awaiting_ask() {
        let result = runtime_from_core(EngineSignal::AwaitingAsk);
        assert_eq!(result, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn runtime_from_core_finished_extracts_value_discards_taint() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Secret));
        match result {
            RuntimeSignal::Finished(SlotValue::I64(v)) => assert_eq!(v, 42),
            other => {
                let msg = format!("expected Finished(I64(42)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_from_core_finished_with_clean_taint() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean));
        match result {
            RuntimeSignal::Finished(SlotValue::Bool(v)) => assert!(v),
            other => {
                let msg = format!("expected Finished(Bool(true)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_from_core_finished_with_null() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::Null, Taint::Clean));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::Null));
    }

    #[test]
    fn runtime_from_core_awaiting_action_produces_zero_ticket() {
        let result = runtime_from_core(EngineSignal::AwaitingAction);
        match result {
            RuntimeSignal::AwaitingAction(ticket) => {
                assert_eq!(ticket.run, RunId::ZERO);
                assert_eq!(ticket.step, StepIdx::ZERO);
                assert_eq!(ticket.seq, SeqNo::ZERO);
                assert_eq!(ticket.action, ActionId::new(0));
                assert_eq!(ticket.attempt, 1);
                assert_eq!(ticket.idempotency_key, 0);
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_from_core_each_variant_produces_distinct_runtime_signal() {
        let signals = [
            runtime_from_core(EngineSignal::Continue),
            runtime_from_core(EngineSignal::Finished(SlotValue::Null, Taint::Clean)),
            runtime_from_core(EngineSignal::StepBudgetExhausted),
            runtime_from_core(EngineSignal::AwaitingAction),
            runtime_from_core(EngineSignal::AwaitingWait),
            runtime_from_core(EngineSignal::AwaitingAsk),
        ];
        for (i, a) in signals.iter().enumerate() {
            for (j, b) in signals.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b, "same-index {i} should be equal");
                } else {
                    assert_ne!(a, b, "different indices {i},{j} should differ");
                }
            }
        }
    }

    // =====================================================================
    // execute_iteration_node: unsupported primitive fallback
    // =====================================================================

    fn make_minimal_workflow() -> CompiledWorkflow {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = WorkflowParts {
            name: Box::from("test_iter"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::from([]),
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(e) => {
                let msg = format!("failed to compile test workflow: {e}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn execute_iteration_node_returns_error_for_nop_kind() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        match result {
            Err(RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive })) => {
                assert_eq!(primitive, "not_an_iteration_node");
            }
            other => {
                let msg = format!("expected UnsupportedPrimitive, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn execute_iteration_node_returns_error_for_jump_kind() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(0),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        match result {
            Err(RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive })) => {
                assert_eq!(primitive, "not_an_iteration_node");
            }
            other => {
                let msg = format!("expected UnsupportedPrimitive, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn execute_iteration_node_returns_error_for_do_kind() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        match result {
            Err(RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive })) => {
                assert_eq!(primitive, "not_an_iteration_node");
            }
            other => {
                let msg = format!("expected UnsupportedPrimitive, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // execute_iteration_node: iteration variants with invalid frame state
    // =====================================================================

    #[test]
    fn execute_iteration_node_for_each_start_errors_on_uninitialized_frame() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 0) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        assert!(
            result.is_err(),
            "expected error for uninitialized frame, got {result:?}"
        );
    }

    #[test]
    fn execute_iteration_node_together_start_with_empty_branches() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::from([]),
                join: StepIdx::new(1),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        // May succeed or error depending on primitives implementation,
        // but must not panic.
        let _ = result;
    }

    #[test]
    fn execute_iteration_node_collect_start_errors_on_uninitialized_source() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 0) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 10,
                page_size: 5,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        assert!(
            result.is_err(),
            "expected error for uninitialized source, got {result:?}"
        );
    }

    #[test]
    fn execute_iteration_node_reduce_start_errors_on_uninitialized_input() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 0) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        assert!(
            result.is_err(),
            "expected error for uninitialized input, got {result:?}"
        );
    }

    #[test]
    fn execute_iteration_node_for_each_join_errors_on_missing_step_state() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(0),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        assert!(
            result.is_err(),
            "expected error for missing step state, got {result:?}"
        );
    }

    #[test]
    fn execute_iteration_node_for_each_next_errors_on_uninitialized_iterator() {
        let workflow = make_minimal_workflow();
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 0) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        };
        let result = execute_iteration_node(&workflow, &mut run, &mut store, &node);
        assert!(
            result.is_err(),
            "expected error for uninitialized iterator, got {result:?}"
        );
    }
}
