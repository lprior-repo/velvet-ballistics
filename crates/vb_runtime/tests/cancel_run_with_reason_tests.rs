#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
#![forbid(unsafe_code)]
//! RQ-W0-18: end-to-end coverage for `Runtime::cancel_run_with_reason`.
//!
//! The shard command path (`ShardCommand::Cancel { run, reason }`) already
//! carried a reason slot, but the public `Runtime` facade only exposed
//! `cancel_run` (no reason). This test exercises the new
//! `cancel_run_with_reason` entry point and asserts that the supplied
//! reason is preserved in the durable `RunCancelled` journal event.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx, WorkflowDigest,
};
use vb_runtime::Runtime;
use vb_runtime::journal::RuntimeJournalEvent;
use vb_runtime::shard::ShardConfig;

const SUSPENDED_WORKFLOW_NAME: &str = "rq_w0_18_cancel_reason";

fn minimal_workflow() -> CompiledWorkflow {
    let set_const = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        },
    };
    let parts = WorkflowParts {
        name: Box::<str>::from(SUSPENDED_WORKFLOW_NAME),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: Box::new([set_const, ask]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(0)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("workflow must compile")
}

fn build_runtime_with_capturing_journal()
-> (Runtime, Arc<vb_runtime::journal::VolatileRuntimeJournal>) {
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared = journal.clone();
    // Relaxed policy wires `AlwaysPresentArtifactStore` automatically so the
    // captured-journal runtime accepts the test workflow digest without seeding
    // a per-test artifact store. `coalesce_window_ticks: 1` disables the
    // coalescing buffer so journal events are appended immediately per tick.
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        ..ShardConfig::default()
    };
    let runtime = Runtime::new_with_journal(NonZeroUsize::MIN, config, shared)
        .expect("runtime construction must succeed with default config");
    (runtime, journal)
}

#[test]
fn cancel_run_with_reason_records_reason_in_journal_event() {
    let (mut runtime, journal) = build_runtime_with_capturing_journal();
    let run = RunId::new(0xC0DE_0001);
    let workflow = minimal_workflow();
    runtime
        .submit_compiled(run, workflow)
        .expect("submit must succeed");
    runtime.tick_all().expect("tick after submit");

    let reason = "operator-requested-shutdown-for-incident-2026-06-21";
    runtime
        .cancel_run_with_reason(run, Some(reason.to_string()))
        .expect("cancel_run_with_reason must succeed for active run");
    runtime.tick_all().expect("tick after cancel");

    let events = journal.snapshot().expect("journal snapshot must succeed");
    let cancel_event = events
        .iter()
        .find_map(|event| match event {
            RuntimeJournalEvent::RunCancelled {
                run: r,
                reason: ev_reason,
            } if *r == run => Some(ev_reason.clone()),
            _ => None,
        })
        .expect("RunCancelled event must be recorded for the cancelled run");

    assert_eq!(
        cancel_event.as_deref(),
        Some(reason),
        "RunCancelled journal event must preserve the reason supplied to cancel_run_with_reason"
    );

    assert_eq!(
        runtime.snapshot_run(run, 0),
        Ok(vb_runtime::shard::InspectResponse::Terminal {
            run,
            correlation: 0,
            outcome: vb_runtime::shard::TerminalOutcome::Cancelled,
        }),
        "cancelled run must surface as Terminal with Cancelled outcome"
    );
}

#[test]
fn cancel_run_without_reason_records_none_in_journal_event() {
    let (mut runtime, journal) = build_runtime_with_capturing_journal();
    let run = RunId::new(0xC0DE_0002);
    let workflow = minimal_workflow();
    runtime
        .submit_compiled(run, workflow)
        .expect("submit must succeed");
    runtime.tick_all().expect("tick after submit");

    runtime
        .cancel_run(run)
        .expect("cancel_run must succeed for active run");
    runtime.tick_all().expect("tick after cancel");

    let events = journal.snapshot().expect("journal snapshot must succeed");
    let cancel_event = events
        .iter()
        .find_map(|event| match event {
            RuntimeJournalEvent::RunCancelled { run: r, reason } if *r == run => {
                Some(reason.clone())
            }
            _ => None,
        })
        .expect("RunCancelled event must be recorded");
    assert!(
        cancel_event.is_none(),
        "cancel_run without reason must record None in journal event"
    );
}
