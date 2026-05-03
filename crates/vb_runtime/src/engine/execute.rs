#![forbid(unsafe_code)]

//! Node execution dispatch for all compiled node kinds.

use vb_core::action::ActionContract;
use vb_core::frame::RunFrame;
use vb_core::ids::SeqNo;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::action::{
    execute_do, execute_do_without_contract, execute_retry_check, resolve_contract,
};
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

/// Executes one compiled node with full primitive dispatch.
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    collect_states: &mut CollectStates,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => crate::primitives::for_each::for_each_start(
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
        } => crate::primitives::for_each::for_each_next(
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
            match crate::primitives::for_each::for_each_join(
                run,
                *output,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            crate::primitives::together::together_start(run, store, branches, *join, node.output)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => crate::primitives::together::together_branch(
            run,
            store,
            *branch,
            *entry,
            *join,
            *accumulator,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            let step = node.id;
            match crate::primitives::together::together_join(
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
        } => crate::primitives::collect::collect_start(
            run,
            store,
            collect_states,
            *source,
            *limit,
            *page_size,
            *body,
            *done,
            node.output,
            None,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => crate::primitives::collect::collect_page(
            run,
            store,
            collect_states,
            *collector_slot,
            *body,
            *done,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => crate::primitives::collect::collect_next(
            run,
            store,
            collect_states,
            *collector_slot,
            *body,
            *done,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::CollectFinish { collector_slot } => {
            let step = node.id;
            match crate::primitives::collect::collect_finish(
                run,
                collect_states,
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
        } => crate::primitives::reduce::reduce_start(
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
        } => crate::primitives::reduce::reduce_next(
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
            match crate::primitives::reduce::reduce_finish(
                run,
                *accumulator,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => crate::primitives::repeat::repeat_start(run, *max_attempts, *body, *done, node.output)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => crate::primitives::repeat::repeat_attempt(run, *attempt_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            crate::primitives::repeat::repeat_check(run, *attempt_slot, *done, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::RepeatFinish { result } => {
            crate::primitives::repeat::repeat_finish(run, *result, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitUntil { deadline_slot } => {
            crate::primitives::wait_ask::wait_until(run, *deadline_slot)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => crate::primitives::wait_ask::wait_event(run, *event, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => crate::primitives::wait_ask::ask(run, *prompt, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::AskResume { answer } => {
            crate::primitives::wait_ask::ask_resume(run, *answer, node.output, node.next, node.id)
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
            ..
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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts,
    };
    use vb_core::value_store::ValueStore;
    use crate::primitives::collect::CollectStates;

    // Single-node workflow: all step refs must be ZERO or None.
    fn make_workflow(node: CompiledNode) -> Option<CompiledWorkflow> {
        let parts = WorkflowParts {
            name: Box::from("test_exec"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::from([]),
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn make_run(slot_count: u16, step_state_count: u16) -> Option<RunFrame> {
        RunFrame::new(RunId::new(1), StepIdx::new(0), slot_count, step_state_count).ok()
    }

    fn get_node(wf: &CompiledWorkflow) -> Option<&CompiledNode> {
        wf.node(StepIdx::ZERO)
    }

    // Nop dispatch (falls through to step_once)
    #[test]
    fn execute_nop_returns_continue() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None, kind: CompiledNodeKind::Nop,
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        match result {
            Ok(RuntimeSignal::Continue) | Ok(RuntimeSignal::StepBudgetExhausted) => {}
            other => { let msg = format!("expected Continue or StepBudgetExhausted, got {other:?}"); panic!("{msg}"); }
        }
    }

    // Do dispatch with empty contracts
    #[test]
    fn execute_do_without_contract_returns_awaiting_action() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Do { action: ActionId::new(5), input: SlotIdx::new(0) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => assert_eq!(ticket.action, ActionId::new(5)),
            other => { let msg = format!("expected AwaitingAction, got {other:?}"); panic!("{msg}"); }
        }
    }

    // Do dispatch with known contract
    #[test]
    fn execute_do_with_known_contract_returns_awaiting_action() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Do { action: ActionId::new(1), input: SlotIdx::new(0) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        if run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(10)).is_err() { return; }
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let contracts: Vec<ActionContract> = vec![
            ActionContract { id: ActionId::new(0), input_slot_count: 0, output_slot_count: 0, max_input_bytes: 0, max_output_bytes: 0, timeout_ms: 0, idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]) },
            ActionContract { id: ActionId::new(1), input_slot_count: 1, output_slot_count: 0, max_input_bytes: 1024, max_output_bytes: 1024, timeout_ms: 5000, idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]) },
        ];
        let result = execute_node_full(&wf, &mut run, &mut store, n, &contracts, RetryPolicy::NEVER, &mut cs);
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => { assert_eq!(ticket.action, ActionId::new(1)); assert_eq!(ticket.run, RunId::new(1)); }
            other => { let msg = format!("expected AwaitingAction, got {other:?}"); panic!("{msg}"); }
        }
    }

    // Do dispatch with unknown contract
    #[test]
    fn execute_do_with_unknown_contract_returns_error() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Do { action: ActionId::new(99), input: SlotIdx::new(0) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let contracts: Vec<ActionContract> = vec![
            ActionContract { id: ActionId::new(0), input_slot_count: 0, output_slot_count: 0, max_input_bytes: 0, max_output_bytes: 0, timeout_ms: 0, idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]) },
        ];
        let result = execute_node_full(&wf, &mut run, &mut store, n, &contracts, RetryPolicy::NEVER, &mut cs);
        match result {
            Err(RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction { action })) => assert_eq!(action, ActionId::new(99)),
            other => { let msg = format!("expected UnknownAction error, got {other:?}"); panic!("{msg}"); }
        }
    }

    // RetryCheck with NEVER policy
    #[test]
    fn execute_retry_check_with_never_policy_returns_continue() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::RetryCheck { policy_slot: SlotIdx::new(0), body: StepIdx::ZERO, exhausted: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert_eq!(result, Ok(RuntimeSignal::Continue));
    }

    // RetryCheck with DEFAULT policy
    #[test]
    fn execute_retry_check_with_default_policy_returns_continue() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::RetryCheck { policy_slot: SlotIdx::new(0), body: StepIdx::ZERO, exhausted: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::DEFAULT, &mut cs);
        assert_eq!(result, Ok(RuntimeSignal::Continue));
    }

    // ErrorHandler
    #[test]
    fn execute_error_handler_returns_continue() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::ErrorHandler { body: StepIdx::ZERO, handler: StepIdx::ZERO, error_slot: None },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert_eq!(result, Ok(RuntimeSignal::Continue));
    }

    #[test]
    fn execute_error_handler_with_error_slot_returns_continue() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::ErrorHandler { body: StepIdx::ZERO, handler: StepIdx::ZERO, error_slot: Some(SlotIdx::new(3)) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert_eq!(result, Ok(RuntimeSignal::Continue));
    }

    // ForEachStart error path
    #[test]
    fn execute_for_each_start_errors_on_uninitialized_slot() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::ForEachStart { input: SlotIdx::new(5), item_slot: SlotIdx::new(6), limit: 10, body: StepIdx::ZERO, done: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized slot, got {result:?}");
    }

    // RepeatStart zero attempts
    #[test]
    fn execute_repeat_start_with_zero_attempts_no_panic() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::RepeatStart { max_attempts: 0, body: StepIdx::ZERO, done: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        let _ = result;
    }

    // CollectStart error path
    #[test]
    fn execute_collect_start_errors_on_uninitialized_source() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::CollectStart { source: SlotIdx::new(5), limit: 10, page_size: 5, body: StepIdx::ZERO, done: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized source, got {result:?}");
    }

    // TogetherStart empty branches
    #[test]
    fn execute_together_start_with_empty_branches_no_panic() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::TogetherStart { branches: Box::from([]), join: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        let _ = result;
    }

    // ReduceStart error path
    #[test]
    fn execute_reduce_start_errors_on_uninitialized_input() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::ReduceStart { input: SlotIdx::new(5), accumulator: SlotIdx::new(6), initial: vb_core::ids::ConstIdx::new(0), body: StepIdx::ZERO, done: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized input, got {result:?}");
    }

    // Jump dispatch
    #[test]
    fn execute_jump_dispatches_to_step_once() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Jump { target: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        match result {
            Ok(RuntimeSignal::Continue) | Ok(RuntimeSignal::StepBudgetExhausted) => {}
            other => { let msg = format!("expected Continue or StepBudgetExhausted, got {other:?}"); panic!("{msg}"); }
        }
    }

    // WaitUntil error path
    #[test]
    fn execute_wait_until_errors_on_uninitialized_deadline() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::new(5) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized deadline, got {result:?}");
    }

    // Ask error path
    #[test]
    fn execute_ask_errors_on_uninitialized_prompt() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Ask { prompt: SlotIdx::new(5), timeout_slot: None },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized prompt, got {result:?}");
    }

    // Do with tainted input -> TaintViolation
    #[test]
    fn execute_do_taint_violation_for_deterministic_pure_with_secret_input() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::Do { action: ActionId::new(1), input: SlotIdx::new(0) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(4, 2) { Some(f) => f, None => return };
        if run.write_slot_with_taint(SlotIdx::new(0), vb_core::value::SlotValue::I64(1), vb_core::value::Taint::Secret).is_err() { return; }
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let contracts: Vec<ActionContract> = vec![
            ActionContract { id: ActionId::new(0), input_slot_count: 0, output_slot_count: 0, max_input_bytes: 0, max_output_bytes: 0, timeout_ms: 0, idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]) },
            ActionContract { id: ActionId::new(1), input_slot_count: 1, output_slot_count: 0, max_input_bytes: 1024, max_output_bytes: 1024, timeout_ms: 5000, idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]) },
        ];
        let result = execute_node_full(&wf, &mut run, &mut store, n, &contracts, RetryPolicy::NEVER, &mut cs);
        match result {
            Err(RuntimeEngineError::TaintViolation { step }) => assert_eq!(step, StepIdx::ZERO),
            other => { let msg = format!("expected TaintViolation, got {other:?}"); panic!("{msg}"); }
        }
    }

    // WaitEvent error path
    #[test]
    fn execute_wait_event_errors_on_uninitialized_event() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::WaitEvent { event: SlotIdx::new(5), timeout_slot: None },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized event, got {result:?}");
    }

    // AskResume error path
    #[test]
    fn execute_ask_resume_errors_on_uninitialized_answer() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::AskResume { answer: SlotIdx::new(5) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized answer, got {result:?}");
    }

    // RepeatAttempt error path
    #[test]
    fn execute_repeat_attempt_errors_on_uninitialized_attempt_slot() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: None, next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt { attempt_slot: SlotIdx::new(5), body: StepIdx::ZERO, done: StepIdx::ZERO },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized attempt slot, got {result:?}");
    }

    // RepeatFinish error path
    #[test]
    fn execute_repeat_finish_errors_on_uninitialized_result_slot() {
        let node = CompiledNode {
            id: StepIdx::ZERO, output: Some(SlotIdx::new(0)), next: Some(StepIdx::ZERO),
            on_error: None, error_slot: None,
            kind: CompiledNodeKind::RepeatFinish { result: SlotIdx::new(5) },
        };
        let wf = match make_workflow(node) { Some(w) => w, None => return };
        let mut run = match make_run(8, 4) { Some(f) => f, None => return };
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match get_node(&wf) { Some(n) => n, None => return };
        let result = execute_node_full(&wf, &mut run, &mut store, n, &[], RetryPolicy::NEVER, &mut cs);
        assert!(result.is_err(), "expected error for uninitialized result slot, got {result:?}");
    }
}
