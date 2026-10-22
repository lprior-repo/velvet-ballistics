#![forbid(unsafe_code)]
//! CLI durable retry integration tests (vb-r5c1c).
//!
//! These tests produce a real failed run (with a durable retry checkpoint) through
//! the public `Runtime` + `FjallJournal` boundary, then invoke the actual
//! `velvet-ballistics retry` binary and assert re-execution or non-mutating
//! rejection. No manually-constructed checkpoints, no test flags, no fabricated
//! journal events.

use std::num::NonZeroUsize;
use std::process::Output;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionTicket, Idempotency,
    RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::SlotValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

fn shard_count() -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
}

fn canonical_workflow_digest(parts: &WorkflowParts) -> Result<WorkflowDigest, String> {
    let mut canonical = parts.clone();
    canonical.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let bytes =
        postcard::to_allocvec(&canonical).map_err(|e| format!("workflow encode failed: {e}"))?;
    Ok(WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes()))
}

fn action_workflow() -> Result<CompiledWorkflow, String> {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("cli-retry-action"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest = canonical_workflow_digest(&parts)?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("workflow invalid: {e:?}"))
}

fn completed_workflow() -> Result<CompiledWorkflow, String> {
    let set = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("cli-retry-completed"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([set, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(1)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest = canonical_workflow_digest(&parts)?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("workflow invalid: {e:?}"))
}

fn action_contract() -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").map_err(|e| e.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

fn non_retryable_failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn pending_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("run {run:?} has no pending action ticket"))
}

/// Produces a durable failed run (with a retry checkpoint) and closes the journal.
/// Returns the run id and the durable event count at failure time.
fn produce_failed_run(path: &std::path::Path, run: RunId) -> Result<usize, String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = action_workflow()?;
    let contracts = Box::from([action_contract()?]);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )
    .map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow.clone(),
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;
    let ticket = pending_ticket(&runtime, run)?;
    runtime
        .fail_action(ticket, non_retryable_failure())
        .map_err(|e| format!("fail_action failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, JournalEvent::RetryCheckpoint { .. })),
        "failure boundary must persist a durable retry checkpoint"
    );
    Ok(events.len())
}

fn produce_completed_run(path: &std::path::Path, run: RunId) -> Result<usize, String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = completed_workflow()?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[],
    )
    .map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .submit_direct(run, workflow.clone())
        .map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(events.len())
}

fn run_cli(args: &[&std::ffi::OsStr]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output()
        .expect("failed to spawn CLI binary")
}

fn events_count(path: &std::path::Path, run: RunId) -> Result<usize, String> {
    let journal = FjallJournal::open(path, None).map_err(|e| e.to_string())?;
    journal.events_for_run(run).map(|e| e.len()).map_err(|e| e.to_string())
}

#[test]
fn cli_retry_reexecutes_the_failed_step() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_001);
    let before = produce_failed_run(&path, run)?;

    let run_id = run.get().to_string();
    let output = run_cli(&[
        std::ffi::OsStr::new("retry"),
        std::ffi::OsStr::new(&run_id),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--db"),
        path.as_os_str(),
    ]);
    assert!(
        output.status.success(),
        "retry must succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = events_count(&path, run)?;
    assert!(
        after > before,
        "retry must append re-execution evidence (before={before}, after={after})"
    );
    Ok(())
}

#[test]
fn cli_retry_rejects_wrong_step_without_mutation() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_002);
    let before = produce_failed_run(&path, run)?;

    let run_id = run.get().to_string();
    let output = run_cli(&[
        std::ffi::OsStr::new("retry"),
        std::ffi::OsStr::new(&run_id),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("7"),
        std::ffi::OsStr::new("--db"),
        path.as_os_str(),
    ]);
    assert!(
        !output.status.success(),
        "retry with a stale step must fail"
    );

    let after = events_count(&path, run)?;
    assert_eq!(before, after, "rejection must not mutate the journal");
    Ok(())
}

#[test]
fn cli_retry_rejects_non_failed_run_without_mutation() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_003);
    let before = produce_completed_run(&path, run)?;

    let run_id = run.get().to_string();
    let output = run_cli(&[
        std::ffi::OsStr::new("retry"),
        std::ffi::OsStr::new(&run_id),
        std::ffi::OsStr::new("--step"),
        std::ffi::OsStr::new("0"),
        std::ffi::OsStr::new("--db"),
        path.as_os_str(),
    ]);
    assert!(
        !output.status.success(),
        "retry on a completed run must fail"
    );

    let after = events_count(&path, run)?;
    assert_eq!(before, after, "rejection must not mutate the journal");
    Ok(())
}
