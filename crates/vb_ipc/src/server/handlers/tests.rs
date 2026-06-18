#![forbid(unsafe_code)]
//! Handler-specific IPC v1 tests.
//!
//! Tests live here to keep each handler file focused on its domain,
//! while the test module has full access to internal helpers via `super`.

use super::actions::handle_answer_ask;
use crate::server::IpcResponse;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_runtime::RuntimeError;
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

fn runtime_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    }
}

fn ask_then_finish_workflow() -> Option<CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::ZERO),
        },
    };
    let resume = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: Some(StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::AskResume {
            answer: SlotIdx::new(1),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ipc_ask_then_finish"),
        digest: WorkflowDigest::from_bytes([31; 32]),
        nodes: Box::from([set_prompt, ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn answer_payload(
    run_id: RunId,
    answer_slot: SlotIdx,
    value_bytes: Vec<u8>,
    taint: Option<Taint>,
) -> Option<Vec<u8>> {
    postcard::to_allocvec(&crate::IpcPayload::AnswerAsk {
        run_id,
        answer_slot,
        answer: value_bytes,
        taint,
    })
    .ok()
}

fn encoded_value(value: &SlotValue) -> Option<Vec<u8>> {
    postcard::to_allocvec(value).ok()
}

fn must_encoded_value(value: &SlotValue) -> Vec<u8> {
    match encoded_value(value) {
        Some(bytes) => bytes,
        None => panic!("test setup failed: SlotValue {value:?} must postcard encode"),
    }
}

fn must_answer_payload(
    run_id: RunId,
    answer_slot: SlotIdx,
    value_bytes: Vec<u8>,
    taint: Option<Taint>,
) -> Vec<u8> {
    match answer_payload(run_id, answer_slot, value_bytes, taint) {
        Some(payload) => payload,
        None => panic!("test setup failed: AnswerAsk IPC payload must postcard encode"),
    }
}

fn runtime_with_pending_ask(
    run_id: RunId,
    journal: Arc<VolatileRuntimeJournal>,
) -> Option<Runtime> {
    let shard_count = NonZeroUsize::new(1)?;
    let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal);
    let workflow = ask_then_finish_workflow()?;
    if runtime.submit_compiled(run_id, workflow) != Ok(()) {
        return None;
    }
    if runtime.tick_all() != Ok(true) {
        return None;
    }
    Some(runtime)
}

#[test]
fn handle_answer_ask_accepts_valid_postcard_slot_value_and_default_clean_taint() {
    let run_id = RunId::new(3101);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
        Some(runtime) => runtime,
        None => panic!("test setup failed: runtime must reach pending ask state"),
    };
    let expected_answer = must_encoded_value(&SlotValue::I64(42));
    let payload = must_answer_payload(run_id, SlotIdx::new(1), expected_answer.clone(), None);

    assert_eq!(
        handle_answer_ask(&payload, &mut runtime),
        IpcResponse::AcceptedRun { run_id: 3101 }
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);

    let snapshot = journal.snapshot();
    match snapshot {
        Ok(events) => {
            let matched = events.iter().any(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                        if *run == run_id
                            && *slot == SlotIdx::new(1)
                            && *value == expected_answer
                            && *taint == Taint::Clean
                )
            });
            assert_eq!(
                matched, true,
                "journal must contain exact SlotValue::I64(42) postcard bytes for answered slot"
            );
        }
        Err(e) => panic!("journal snapshot failed: {e}"),
    }
}

#[test]
fn handle_answer_ask_rejects_mismatched_answer_slot_without_consuming_pending_ask() {
    let run_id = RunId::new(3102);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
        Some(runtime) => runtime,
        None => panic!("test setup failed: runtime must reach pending ask state"),
    };
    let wrong_answer = must_encoded_value(&SlotValue::I64(7));
    let wrong_payload = must_answer_payload(run_id, SlotIdx::ZERO, wrong_answer, None);

    match handle_answer_ask(&wrong_payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(message, RuntimeError::InvalidActionCompletion.to_string());
        }
        other => panic!("expected RuntimeError, got {other:?}"),
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);

    let valid_answer = must_encoded_value(&SlotValue::I64(8));
    let valid_payload = must_answer_payload(run_id, SlotIdx::new(1), valid_answer.clone(), None);
    assert_eq!(
        handle_answer_ask(&valid_payload, &mut runtime),
        IpcResponse::AcceptedRun { run_id: 3102 }
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = match journal.snapshot() {
        Ok(events) => events,
        Err(e) => panic!("journal snapshot failed after valid answer: {e}"),
    };
    let matched = events.iter().any(|event| {
        matches!(
            event,
            RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                if *run == run_id
                    && *slot == SlotIdx::new(1)
                    && *value == valid_answer
                    && *taint == Taint::Clean
        )
    });
    assert_eq!(
        matched, true,
        "valid retry must write exact SlotValue::I64(8) postcard bytes after wrong slot rejection"
    );
}

#[test]
fn handle_answer_ask_rejects_absent_pending_ask() {
    let shard_count = match NonZeroUsize::new(1) {
        Some(shard_count) => shard_count,
        None => panic!("test setup failed: shard_count must be non-zero"),
    };
    let mut runtime = Runtime::new(shard_count, runtime_config());
    let run_id = RunId::new(3103);
    let answer = must_encoded_value(&SlotValue::Bool(true));
    let payload = must_answer_payload(run_id, SlotIdx::new(1), answer, None);

    match handle_answer_ask(&payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(message, RuntimeError::RunNotFound.to_string());
        }
        other => panic!("expected RuntimeError, got {other:?}"),
    }
}

#[test]
fn handle_answer_ask_rejects_malformed_slot_value_bytes_before_runtime_mutation() {
    let run_id = RunId::new(3104);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
        Some(runtime) => runtime,
        None => panic!("test setup failed: runtime must reach pending ask state"),
    };
    let malformed_payload = must_answer_payload(run_id, SlotIdx::new(1), vec![255, 255], None);

    match handle_answer_ask(&malformed_payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(
                message,
                "answer bytes are not valid postcard-encoded SlotValue"
            );
        }
        other => panic!("expected RuntimeError, got {other:?}"),
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);

    let valid_answer = must_encoded_value(&SlotValue::Bool(false));
    let valid_payload = must_answer_payload(run_id, SlotIdx::new(1), valid_answer.clone(), None);
    assert_eq!(
        handle_answer_ask(&valid_payload, &mut runtime),
        IpcResponse::AcceptedRun { run_id: 3104 }
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = match journal.snapshot() {
        Ok(events) => events,
        Err(e) => panic!("journal snapshot failed after malformed rejection recovery: {e}"),
    };
    let matched = events.iter().any(|event| {
        matches!(
            event,
            RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                if *run == run_id
                    && *slot == SlotIdx::new(1)
                    && *value == valid_answer
                    && *taint == Taint::Clean
        )
    });
    assert_eq!(
        matched, true,
        "pending ask must remain consumable and write exact SlotValue::Bool(false) bytes after malformed rejection"
    );
}
