#![forbid(unsafe_code)]

use proptest::prelude::*;
use std::num::NonZeroUsize;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

#[derive(Debug)]
enum Scenario {
    AbsentRun {
        run: RunId,
    },
    AbsentTimer {
        run: RunId,
    },
    ActionSuspended {
        run: RunId,
    },
    WaitTimer {
        run: RunId,
    },
    Valid {
        run: RunId,
        value: i64,
    },
    Mismatch {
        run: RunId,
        wrong_slot: SlotIdx,
        value: i64,
    },
}

fn runtime_config(step_budget: u64) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 32,
        step_budget_per_tick: step_budget,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    }
}

fn runtime(step_budget: u64) -> Runtime {
    let shard_count = NonZeroUsize::new(1).expect("one shard is non-zero");
    Runtime::new(shard_count, runtime_config(step_budget))
}

fn workflow(
    nodes: Box<[CompiledNode]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: Box::from("vb_jpq7_21_generated"),
        digest: WorkflowDigest::from_bytes([27; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("generated workflow must validate")
}

fn ask_workflow() -> CompiledWorkflow {
    workflow(
        Box::from([
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: Some(SlotIdx::ZERO),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]),
        Box::from([ConstValue::I64(10)]),
        2,
    )
}

fn no_timer_workflow() -> CompiledWorkflow {
    workflow(
        Box::from([
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            },
        ]),
        Box::from([ConstValue::I64(10)]),
        1,
    )
}

fn action_workflow() -> CompiledWorkflow {
    workflow(
        Box::from([CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        }]),
        Box::from([]),
        1,
    )
}

fn wait_workflow() -> CompiledWorkflow {
    workflow(
        Box::from([
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            },
        ]),
        Box::from([ConstValue::I64(10)]),
        1,
    )
}

fn submit_and_tick(rt: &mut Runtime, run: RunId, wf: CompiledWorkflow) {
    assert_eq!(rt.submit_compiled(run, wf), Ok(()));
    assert_eq!(rt.tick_all(), Ok(true));
}

fn action_contract(action: ActionId) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::new("generated-action").expect("action name is valid"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([Capability::new("generated".into(), action)]),
    }
}

fn submit_action_and_tick(rt: &mut Runtime, run: RunId, wf: CompiledWorkflow) {
    let action = ActionId::new(0);
    let caps = CapabilitySet::from_grants(Box::from([Capability::new("generated".into(), action)]));
    assert_eq!(
        rt.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            caps,
            Box::from([action_contract(action)]),
        ),
        Ok(())
    );
    assert_eq!(rt.tick_all(), Ok(true));
}

fn scenario_strategy() -> impl Strategy<Value = Scenario> {
    let run = (1u64..=u64::MAX).prop_map(RunId::new);
    prop_oneof![
        run.clone().prop_map(|run| Scenario::AbsentRun { run }),
        run.clone().prop_map(|run| Scenario::AbsentTimer { run }),
        run.clone()
            .prop_map(|run| Scenario::ActionSuspended { run }),
        run.clone().prop_map(|run| Scenario::WaitTimer { run }),
        (run.clone(), any::<i64>()).prop_map(|(run, value)| Scenario::Valid { run, value }),
        (
            run,
            prop_oneof![Just(SlotIdx::ZERO), Just(SlotIdx::MAX)],
            any::<i64>()
        )
            .prop_map(|(run, wrong_slot, value)| Scenario::Mismatch {
                run,
                wrong_slot,
                value
            }),
    ]
}

proptest! {
    #[test]
    fn vb_jpq7_21_answer_pending_ask_slot_generated(scenario in scenario_strategy()) {
        match scenario {
            Scenario::AbsentRun { run } => {
                let rt = runtime(4);
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean, 1), Err(RuntimeError::RunNotFound));
            }
            Scenario::AbsentTimer { run } => {
                let mut rt = runtime(1);
                submit_and_tick(&mut rt, run, no_timer_workflow());
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::ZERO, SlotValue::Bool(true), Taint::Clean, 1), Err(RuntimeError::InvalidActionCompletion));
            }
            Scenario::ActionSuspended { run } => {
                let mut rt = runtime(4);
                submit_action_and_tick(&mut rt, run, action_workflow());
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::ZERO, SlotValue::Bool(true), Taint::Clean, 1), Err(RuntimeError::InvalidActionCompletion));
            }
            Scenario::WaitTimer { run } => {
                let mut rt = runtime(4);
                submit_and_tick(&mut rt, run, wait_workflow());
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::ZERO, SlotValue::Bool(true), Taint::Clean, 1), Err(RuntimeError::InvalidActionCompletion));
            }
            Scenario::Valid { run, value } => {
                let mut rt = runtime(4);
                submit_and_tick(&mut rt, run, ask_workflow());
                let slot_value = SlotValue::I64(value);
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::new(1), slot_value, Taint::Clean, 9), Ok(()));
                prop_assert_eq!(rt.tick_all(), Ok(true));
                prop_assert_eq!(rt.counters_snapshot().runs_completed, 1);
            }
            Scenario::Mismatch { run, wrong_slot, value } => {
                let mut rt = runtime(4);
                submit_and_tick(&mut rt, run, ask_workflow());
                prop_assert_eq!(rt.answer_pending_ask_slot(run, wrong_slot, SlotValue::I64(value), Taint::Clean, 9), Err(RuntimeError::InvalidActionCompletion));
                prop_assert_eq!(rt.answer_pending_ask_slot(run, SlotIdx::new(1), SlotValue::I64(value), Taint::Clean, 9), Ok(()));
                prop_assert_eq!(rt.tick_all(), Ok(true));
                prop_assert_eq!(rt.counters_snapshot().runs_completed, 1);
            }
        }
    }
}
