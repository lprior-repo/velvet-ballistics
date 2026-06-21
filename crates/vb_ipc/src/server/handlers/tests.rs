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
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
    let bytes = encoded_value(value);
    assert!(
        bytes.is_some(),
        "test setup failed: SlotValue {:?} must postcard encode",
        value
    );
    // Unreachable in practice: the assertion above guarantees postcard encoding
    // succeeded. `unwrap_or_default` avoids the panic path denied by the
    // workspace's `clippy::unwrap_used` / `clippy::panic` policy.
    bytes.unwrap_or_default()
}

fn must_answer_payload(
    run_id: RunId,
    answer_slot: SlotIdx,
    value_bytes: Vec<u8>,
    taint: Option<Taint>,
) -> Vec<u8> {
    let payload = answer_payload(run_id, answer_slot, value_bytes, taint);
    assert!(
        payload.is_some(),
        "test setup failed: AnswerAsk IPC payload must postcard encode"
    );
    // Unreachable in practice: the assertion above guarantees the payload encoded.
    payload.unwrap_or_default()
}

fn runtime_with_pending_ask(
    run_id: RunId,
    journal: Arc<VolatileRuntimeJournal>,
) -> Option<Runtime> {
    let shard_count = NonZeroUsize::new(1)?;
    let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal)
        .expect("runtime config is valid");
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
    let runtime_opt = runtime_with_pending_ask(run_id, journal.clone());
    assert!(
        runtime_opt.is_some(),
        "test setup failed: runtime must reach pending ask state"
    );
    let mut runtime = match runtime_opt {
        Some(runtime) => runtime,
        // Unreachable: the assertion above guarantees the runtime reached a pending ask.
        None => return,
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
    assert!(
        snapshot.is_ok(),
        "journal snapshot failed: {:?}",
        snapshot.as_ref().err()
    );
    let events = match snapshot {
        Ok(events) => events,
        // Unreachable: the assertion above guarantees the snapshot succeeded.
        Err(_) => return,
    };
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
    assert!(
        matched,
        "journal must contain exact SlotValue::I64(42) postcard bytes for answered slot"
    );
}

#[test]
fn handle_answer_ask_rejects_mismatched_answer_slot_without_consuming_pending_ask() {
    let run_id = RunId::new(3102);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let runtime_opt = runtime_with_pending_ask(run_id, journal.clone());
    assert!(
        runtime_opt.is_some(),
        "test setup failed: runtime must reach pending ask state"
    );
    let mut runtime = match runtime_opt {
        Some(runtime) => runtime,
        // Unreachable: the assertion above guarantees the runtime reached a pending ask.
        None => return,
    };
    let wrong_answer = must_encoded_value(&SlotValue::I64(7));
    let wrong_payload = must_answer_payload(run_id, SlotIdx::ZERO, wrong_answer, None);

    match handle_answer_ask(&wrong_payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(message, RuntimeError::InvalidActionCompletion.to_string());
        }
        other => {
            assert!(
                matches!(other, IpcResponse::RuntimeError { .. }),
                "expected RuntimeError, got {:?}",
                other
            );
        }
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
    let snapshot = journal.snapshot();
    assert!(
        snapshot.is_ok(),
        "journal snapshot failed after valid answer: {:?}",
        snapshot.as_ref().err()
    );
    let events = match snapshot {
        Ok(events) => events,
        // Unreachable: the assertion above guarantees the snapshot succeeded.
        Err(_) => return,
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
    assert!(
        matched,
        "valid retry must write exact SlotValue::I64(8) postcard bytes after wrong slot rejection"
    );
}

#[test]
fn handle_answer_ask_rejects_absent_pending_ask() {
    let shard_nz = NonZeroUsize::new(1);
    assert!(
        shard_nz.is_some(),
        "test setup failed: shard_count must be non-zero"
    );
    let shard_count = match shard_nz {
        Some(shard_count) => shard_count,
        // Unreachable: the assertion above guarantees shard_count is non-zero.
        None => return,
    };
    let mut runtime = Runtime::new(shard_count, runtime_config())
        .expect("runtime config is valid");
    let run_id = RunId::new(3103);
    let answer = must_encoded_value(&SlotValue::Bool(true));
    let payload = must_answer_payload(run_id, SlotIdx::new(1), answer, None);

    match handle_answer_ask(&payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(message, RuntimeError::RunNotFound.to_string());
        }
        other => {
            assert!(
                matches!(other, IpcResponse::RuntimeError { .. }),
                "expected RuntimeError, got {:?}",
                other
            );
        }
    }
}

#[test]
fn handle_answer_ask_rejects_malformed_slot_value_bytes_before_runtime_mutation() {
    let run_id = RunId::new(3104);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let runtime_opt = runtime_with_pending_ask(run_id, journal.clone());
    assert!(
        runtime_opt.is_some(),
        "test setup failed: runtime must reach pending ask state"
    );
    let mut runtime = match runtime_opt {
        Some(runtime) => runtime,
        // Unreachable: the assertion above guarantees the runtime reached a pending ask.
        None => return,
    };
    let malformed_payload = must_answer_payload(run_id, SlotIdx::new(1), vec![255, 255], None);

    match handle_answer_ask(&malformed_payload, &mut runtime) {
        IpcResponse::RuntimeError { message } => {
            assert_eq!(
                message,
                "answer bytes are not valid postcard-encoded SlotValue"
            );
        }
        other => {
            assert!(
                matches!(other, IpcResponse::RuntimeError { .. }),
                "expected RuntimeError, got {:?}",
                other
            );
        }
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
    let snapshot = journal.snapshot();
    assert!(
        snapshot.is_ok(),
        "journal snapshot failed after malformed rejection recovery: {:?}",
        snapshot.as_ref().err()
    );
    let events = match snapshot {
        Ok(events) => events,
        // Unreachable: the assertion above guarantees the snapshot succeeded.
        Err(_) => return,
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
    assert!(
        matched,
        "pending ask must remain consumable and write exact SlotValue::Bool(false) bytes after malformed rejection"
    );
}
