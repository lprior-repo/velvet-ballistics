#![forbid(unsafe_code)]

use proptest::prelude::*;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_ipc::IpcPayload;
use vb_ipc::server::IpcResponse;
use vb_ipc::server::handlers::handle_answer_ask;
use vb_runtime::RuntimeError;
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

#[derive(Debug)]
enum HandlerCase {
    Valid {
        run: RunId,
        value: SlotValue,
        taint: Option<Taint>,
    },
    Mismatch {
        run: RunId,
        value: SlotValue,
    },
    Malformed {
        run: RunId,
    },
}

fn config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 32,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    }
}

fn ask_workflow() -> CompiledWorkflow {
    let nodes = Box::from([
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
    ]);
    let parts = WorkflowParts {
        name: Box::from("vb_jpq7_21_handler"),
        digest: WorkflowDigest::from_bytes([28; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("ask workflow must validate")
}

fn runtime_with_pending_ask(run: RunId, journal: Arc<VolatileRuntimeJournal>) -> Runtime {
    let shard_count = NonZeroUsize::new(1).expect("one shard is non-zero");
    let mut runtime = Runtime::new_with_journal(shard_count, config(), journal);
    assert_eq!(runtime.submit_compiled(run, ask_workflow()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    runtime
}

fn slot_value_strategy() -> impl Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64)
    ]
}

fn taint_strategy() -> impl Strategy<Value = Option<Taint>> {
    prop_oneof![
        Just(None),
        Just(Some(Taint::Clean)),
        Just(Some(Taint::DerivedFromSecret)),
        Just(Some(Taint::Random)),
        Just(Some(Taint::TimeDependent))
    ]
}

fn case_strategy() -> impl Strategy<Value = HandlerCase> {
    let run = (1u64..=u64::MAX).prop_map(RunId::new);
    prop_oneof![
        (run.clone(), slot_value_strategy(), taint_strategy())
            .prop_map(|(run, value, taint)| HandlerCase::Valid { run, value, taint }),
        (run.clone(), slot_value_strategy())
            .prop_map(|(run, value)| HandlerCase::Mismatch { run, value }),
        run.prop_map(|run| HandlerCase::Malformed { run }),
    ]
}

fn answer_payload(run: RunId, slot: SlotIdx, answer: Vec<u8>, taint: Option<Taint>) -> Vec<u8> {
    postcard::to_allocvec(&IpcPayload::AnswerAsk {
        run_id: run,
        answer_slot: slot,
        answer,
        taint,
    })
    .expect("AnswerAsk payload must encode")
}

fn encoded_value(value: &SlotValue) -> Vec<u8> {
    postcard::to_allocvec(value).expect("SlotValue must encode")
}

fn slot_written_matches(
    journal: &VolatileRuntimeJournal,
    run: RunId,
    value_bytes: &[u8],
    taint: Taint,
) -> bool {
    let events = journal.snapshot().expect("journal snapshot succeeds");
    events.iter().any(|event| matches!(event, RuntimeJournalEvent::SlotWritten { run: event_run, slot, value, taint: event_taint, .. } if *event_run == run && *slot == SlotIdx::new(1) && value == value_bytes && *event_taint == taint))
}

proptest! {
    #[test]
    fn vb_jpq7_21_answer_ask_handler_generated(case in case_strategy()) {
        match case {
            HandlerCase::Valid { run, value, taint } => {
                let journal = Arc::new(VolatileRuntimeJournal::new());
                let mut runtime = runtime_with_pending_ask(run, journal.clone());
                let bytes = encoded_value(&value);
                let response = handle_answer_ask(&answer_payload(run, SlotIdx::new(1), bytes.clone(), taint), &mut runtime);
                prop_assert_eq!(response, IpcResponse::AcceptedRun { run_id: run.get() });
                prop_assert_eq!(runtime.tick_all(), Ok(true));
                prop_assert_eq!(runtime.counters_snapshot().runs_completed, 1);
                let expected_taint = taint.unwrap_or(Taint::Clean);
                prop_assert_eq!(slot_written_matches(&journal, run, &bytes, expected_taint), true);
            }
            HandlerCase::Mismatch { run, value } => {
                let journal = Arc::new(VolatileRuntimeJournal::new());
                let mut runtime = runtime_with_pending_ask(run, journal);
                let response = handle_answer_ask(&answer_payload(run, SlotIdx::ZERO, encoded_value(&value), None), &mut runtime);
                prop_assert_eq!(response, IpcResponse::RuntimeError { message: RuntimeError::InvalidActionCompletion.to_string() });
                prop_assert_eq!(runtime.counters_snapshot().runs_completed, 0);
                prop_assert_eq!(handle_answer_ask(&answer_payload(run, SlotIdx::new(1), encoded_value(&value), None), &mut runtime), IpcResponse::AcceptedRun { run_id: run.get() });
            }
            HandlerCase::Malformed { run } => {
                let journal = Arc::new(VolatileRuntimeJournal::new());
                let mut runtime = runtime_with_pending_ask(run, journal);
                let response = handle_answer_ask(&answer_payload(run, SlotIdx::new(1), vec![255, 255], None), &mut runtime);
                prop_assert_eq!(response, IpcResponse::RuntimeError { message: String::from("answer bytes are not valid postcard-encoded SlotValue") });
                prop_assert_eq!(runtime.counters_snapshot().runs_completed, 0);
                prop_assert_eq!(handle_answer_ask(&answer_payload(run, SlotIdx::new(1), encoded_value(&SlotValue::Bool(false)), None), &mut runtime), IpcResponse::AcceptedRun { run_id: run.get() });
            }
        }
    }
}
