#![forbid(unsafe_code)]

//! Node execution dispatch for all compiled node kinds.

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

/// Reads the current attempt count from the given policy slot.
/// Returns 0 if the slot is uninitialized (first attempt), or the stored
/// u16 attempt value if it contains an I64. Errors on type mismatch.
fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<u16> {
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

/// Executes one compiled node with full primitive dispatch.
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
        } => crate::primitives::for_each::for_each_start(
            run,
            store,
            *input,
            *item_slot,
            FanoutLimit::new(*limit),
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
                execute_do_without_contract(
                    run,
                    node.id,
                    *action,
                    *input,
                    seq,
                    granted,
                    retry_policy,
                )
            } else {
                execute_do(
                    run,
                    node.id,
                    *action,
                    *input,
                    seq,
                    resolve_contract(*action, contracts)?,
                    contracts,
                    granted,
                    retry_policy,
                )
            }
        }

        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => {
            let current_attempt = read_attempt_from_slot(run, *policy_slot)?;
            let target = execute_retry_check(current_attempt, retry_policy, *body, *exhausted);
            run.set_pc(target).map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        CompiledNodeKind::ErrorHandler {
            body: handler_body, ..
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
    use crate::primitives::collect::CollectStates;
    use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx};
    use vb_core::value::{SlotValue, Taint};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    /// Terminal Finish node. Valid sink for any workflow path.
    fn finish_node(id: u16, slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(slot)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(slot),
            },
        }
    }

    /// Nop node that jumps forward to `next_id`. Used as filler.
    fn nop_forward(id: u16, next_id: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: Some(StepIdx::new(next_id)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    /// Build a workflow from nodes. Panics on validation failure so tests
    /// fail loudly instead of silently skipping.
    fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow {
        make_workflow_with_constants(nodes, slot_count, Box::from([]))
    }

    /// Build a workflow with a non-empty constants pool.
    fn make_workflow_with_constants(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        constants: Box<[vb_core::value::ConstValue]>,
    ) -> CompiledWorkflow {
        let parts = WorkflowParts {
            name: Box::from("test_exec"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants,
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::from([]),
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(e) => {
                let msg = format!("workflow validation failed: {e}");
                panic!("{msg}");
            }
        }
    }

    fn make_run(slot_count: u16, step_state_count: u16) -> RunFrame {
        match RunFrame::new(RunId::new(1), StepIdx::new(0), slot_count, step_state_count) {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("RunFrame::new failed: {e}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Fallback dispatch: Nop falls through to step_once
    // =====================================================================

    #[test]
    fn execute_nop_returns_continue_or_budget_exhausted() {
        // Nop at 0 needs next pointing forward. Use Finish at 1 as sink.
        let node0 = nop_forward(0, 1);
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 2);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Ok(RuntimeSignal::Continue) => {}
            other => {
                let msg = format!("expected Continue, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Fallback dispatch: Jump falls through to step_once
    // =====================================================================

    #[test]
    fn execute_jump_falls_through_to_step_once() {
        // Jump at 0 targeting 1 (forward edge). Finish at 1 as sink.
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 2);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Ok(RuntimeSignal::Continue) => {}
            other => {
                let msg = format!("expected Continue, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Do node: empty contracts fails closed
    // =====================================================================

    #[test]
    fn execute_do_without_contract_rejects_without_ticket() {
        // Do has no kind-specific edges. Single-node workflow with next=None is valid
        // because the validator does not require next for Do (no kind-specific targets).
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(5),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        // Input slot must be initialized with clean taint for the no-contract path.
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(0));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied {
                action,
                ..
            })) => {
                assert_eq!(action, ActionId::new(5));
            }
            other => {
                let msg = format!("expected CapabilityDenied, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Do node: known contract returns AwaitingAction
    // =====================================================================

    #[test]
    fn execute_do_with_known_contract_returns_awaiting_action() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(10));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let contracts: Vec<ActionContract> = vec![
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::new([]),
            },
            ActionContract {
                id: ActionId::new(1),
                input_slot_count: 1,
                output_slot_count: 0,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 5000,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::new([]),
            },
        ];
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &contracts,
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(1));
                assert_eq!(ticket.run, RunId::new(1));
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Do node: unknown contract returns UnknownAction error
    // =====================================================================

    #[test]
    fn execute_do_with_unknown_contract_returns_error() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(99),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let contracts: Vec<ActionContract> = vec![ActionContract {
            id: ActionId::new(0),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }];
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &contracts,
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Err(RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction {
                action,
            })) => {
                assert_eq!(action, ActionId::new(99));
            }
            other => {
                let msg = format!("expected UnknownAction error, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Do node: deterministic-pure with secret input triggers taint violation
    // =====================================================================

    #[test]
    fn execute_do_taint_violation_for_deterministic_pure_with_secret_input() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let contracts: Vec<ActionContract> = vec![ActionContract {
            id: ActionId::new(0),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }];
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &contracts,
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Err(RuntimeEngineError::TaintViolation { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                let msg = format!("expected TaintViolation, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // RetryCheck: NEVER policy routes to exhausted (attempt 0 < max 1 -> body)
    // =====================================================================

    #[test]
    fn execute_retry_check_never_policy_uninitialized_routes_to_body() {
        // Uninitialized policy_slot -> attempt=0, NEVER policy: max_attempts=1
        // 0 < 1, so routes to body (not exhausted).
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        // NEVER policy: max_attempts=1, attempt=0 < 1, routes to body=step0
        let pc = run.pc();
        assert_eq!(pc, StepIdx::new(0), "expected PC routed to body step 0");
    }

    // =====================================================================
    // RetryCheck: NEVER policy with attempt=1 routes to exhausted
    // =====================================================================

    #[test]
    fn execute_retry_check_never_policy_attempt_one_routes_to_exhausted() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 4);
        // Write attempt=1 into the policy slot
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        // NEVER policy: max_attempts=1, attempt=1 >= 1, routes to exhausted=step1
        let pc = run.pc();
        assert_eq!(
            pc,
            StepIdx::new(1),
            "expected PC routed to exhausted step 1"
        );
    }

    // =====================================================================
    // RetryCheck: DEFAULT policy routes to body (attempt 1 < max 3)
    // =====================================================================

    #[test]
    fn execute_retry_check_default_policy_routes_to_body() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 4);
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::DEFAULT,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        // DEFAULT policy: max_attempts=3, attempt=1 < 3, routes to body=step0
        let pc = run.pc();
        assert_eq!(pc, StepIdx::new(0), "expected PC routed to body step 0");
    }

    // =====================================================================
    // RetryCheck: DEFAULT policy with attempt=3 routes to exhausted
    // =====================================================================

    #[test]
    fn execute_retry_check_default_policy_attempt_three_routes_to_exhausted() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 4);
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(3));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::DEFAULT,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        // DEFAULT policy: max_attempts=3, attempt=3 >= 3, routes to exhausted=step1
        let pc = run.pc();
        assert_eq!(
            pc,
            StepIdx::new(1),
            "expected PC routed to exhausted step 1"
        );
    }

    // =====================================================================
    // ErrorHandler: routes PC to body step
    // =====================================================================

    #[test]
    fn execute_error_handler_routes_to_body_step() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        let pc = run.pc();
        assert_eq!(pc, StepIdx::new(1), "expected PC routed to body step 1");
    }

    #[test]
    fn execute_error_handler_with_error_slot_routes_to_body_step() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: Some(SlotIdx::new(3)),
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: Some(SlotIdx::new(3)),
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        let pc = run.pc();
        assert_eq!(pc, StepIdx::new(1), "expected PC routed to body step 1");
    }

    // =====================================================================
    // ForEachStart: errors on uninitialized input slot
    // =====================================================================

    #[test]
    fn execute_for_each_start_errors_on_uninitialized_input() {
        // body=1 (forward, makes node 1 reachable), done=2 (forward)
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(5),
                item_slot: SlotIdx::new(6),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized input, got {result:?}"
        );
    }

    // =====================================================================
    // ForEachJoin: errors on missing step state
    // =====================================================================

    #[test]
    fn execute_for_each_join_errors_on_missing_step_state() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node0], 4);
        let mut run = make_run(4, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for missing step state, got {result:?}"
        );
    }

    // =====================================================================
    // ForEachNext: errors on uninitialized iterator slot
    // =====================================================================

    #[test]
    fn execute_for_each_next_errors_on_uninitialized_iterator() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized iterator, got {result:?}"
        );
    }

    // =====================================================================
    // TogetherStart: empty branches should not panic
    // =====================================================================

    #[test]
    fn execute_together_start_empty_branches_no_panic() {
        // join=1 must be forward
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::from([]),
                join: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 4);
        let mut run = make_run(4, 2);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        // This catch_unwind match is a panic-safety guard, not a fallible-result
        // assertion. It verifies that execute_node_full itself does not panic;
        // the inner RuntimeEngineResult is intentionally not asserted here.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_node_full(
                &wf,
                &mut run,
                &mut store,
                n,
                &[],
                RetryPolicy::NEVER,
                &mut cs,
                &CapabilitySet::empty(),
            )
        }));
        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    // =====================================================================
    // TogetherJoin: errors on missing step state
    // =====================================================================

    #[test]
    fn execute_together_join_errors_on_missing_step_state() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: SlotIdx::new(1),
            },
        };
        let wf = make_workflow(vec![node0], 4);
        let mut run = make_run(4, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for missing step state, got {result:?}"
        );
    }

    // =====================================================================
    // CollectStart: errors on uninitialized source slot
    // =====================================================================

    #[test]
    fn execute_collect_start_errors_on_uninitialized_source() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(5),
                limit: 10,
                page_size: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized source, got {result:?}"
        );
    }

    // =====================================================================
    // CollectPage: errors on uninitialized collector slot
    // =====================================================================

    #[test]
    fn execute_collect_page_errors_on_uninitialized_collector() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let node1 = finish_node(1, 0);
        let node2 = finish_node(2, 0);
        let wf = make_workflow(vec![node0, node1, node2], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized collector, got {result:?}"
        );
    }

    // =====================================================================
    // CollectNext: errors on uninitialized collector slot
    // =====================================================================

    #[test]
    fn execute_collect_next_errors_on_uninitialized_collector() {
        // 2-node: body=0 (backward to self), done=1 (forward)
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized collector, got {result:?}"
        );
    }

    // =====================================================================
    // CollectFinish: errors on uninitialized collector slot
    // =====================================================================

    #[test]
    fn execute_collect_finish_errors_on_uninitialized_collector() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node0], 4);
        let mut run = make_run(4, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized collector, got {result:?}"
        );
    }

    // =====================================================================
    // ReduceStart: errors on uninitialized input slot
    // =====================================================================

    #[test]
    fn execute_reduce_start_errors_on_uninitialized_input() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(5),
                accumulator: SlotIdx::new(6),
                initial: ConstIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let constants: Box<[vb_core::value::ConstValue]> =
            Box::from([vb_core::value::ConstValue::I64(0)]);
        let wf = make_workflow_with_constants(vec![node0, node1], 8, constants);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized input, got {result:?}"
        );
    }

    // =====================================================================
    // ReduceNext: errors on uninitialized iterator slot
    // =====================================================================

    #[test]
    fn execute_reduce_next_errors_on_uninitialized_iterator() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized iterator, got {result:?}"
        );
    }

    // =====================================================================
    // ReduceFinish: errors on missing step state
    // =====================================================================

    #[test]
    fn execute_reduce_finish_errors_on_missing_step_state() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node0], 4);
        let mut run = make_run(4, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for missing step state, got {result:?}"
        );
    }

    // =====================================================================
    // RepeatStart: zero max_attempts should not panic
    // =====================================================================

    #[test]
    fn execute_repeat_start_single_attempt_no_panic() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        // This catch_unwind match is a panic-safety guard, not a fallible-result
        // assertion. It verifies that execute_node_full itself does not panic;
        // the inner RuntimeEngineResult is intentionally not asserted here.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_node_full(
                &wf,
                &mut run,
                &mut store,
                n,
                &[],
                RetryPolicy::NEVER,
                &mut cs,
                &CapabilitySet::empty(),
            )
        }));
        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    // =====================================================================
    // RepeatAttempt: errors on uninitialized attempt slot
    // =====================================================================

    #[test]
    fn execute_repeat_attempt_errors_on_uninitialized_attempt_slot() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(5),
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized attempt slot, got {result:?}"
        );
    }

    // =====================================================================
    // RepeatFinish: errors on uninitialized result slot
    // =====================================================================

    #[test]
    fn execute_repeat_finish_errors_on_uninitialized_result_slot() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(5),
            },
        };
        let wf = make_workflow(vec![node0], 8);
        let mut run = make_run(8, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized result slot, got {result:?}"
        );
    }

    // =====================================================================
    // WaitUntil: errors on uninitialized deadline slot
    // =====================================================================

    #[test]
    fn execute_wait_until_errors_on_uninitialized_deadline() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(5),
            },
        };
        let wf = make_workflow(vec![node0], 8);
        let mut run = make_run(8, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized deadline, got {result:?}"
        );
    }

    // =====================================================================
    // WaitEvent: errors on uninitialized event slot
    // =====================================================================

    #[test]
    fn execute_wait_event_errors_on_uninitialized_event() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(5),
                timeout_slot: None,
            },
        };
        let wf = make_workflow(vec![node0], 8);
        let mut run = make_run(8, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized event, got {result:?}"
        );
    }

    // =====================================================================
    // Ask: errors on uninitialized prompt slot
    // =====================================================================

    #[test]
    fn execute_ask_errors_on_uninitialized_prompt() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(5),
                timeout_slot: None,
            },
        };
        let wf = make_workflow(vec![node0], 8);
        let mut run = make_run(8, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized prompt, got {result:?}"
        );
    }

    // =====================================================================
    // AskResume: errors on uninitialized answer slot
    // =====================================================================

    #[test]
    fn execute_ask_resume_errors_on_uninitialized_answer() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(5),
            },
        };
        let wf = make_workflow(vec![node0], 8);
        let mut run = make_run(8, 1);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            result.is_err(),
            "expected error for uninitialized answer, got {result:?}"
        );
    }

    // =====================================================================
    // RepeatCheck: routes to done when attempt exceeds max
    // =====================================================================

    #[test]
    fn execute_repeat_check_routes_forward_on_done() {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(1),
            },
        };
        let node1 = finish_node(1, 0);
        let wf = make_workflow(vec![node0, node1], 8);
        let mut run = make_run(8, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        // RepeatCheck reads attempt_slot, but since it is uninitialized we expect an error
        assert!(
            result.is_err(),
            "expected error for uninitialized attempt slot, got {result:?}"
        );
    }
}
