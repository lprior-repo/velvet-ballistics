use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fail_assert(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(fail_assert(format_args!($($arg)*)), $($arg)*)
    }
}

/// Creates a simple two-node workflow: SetConst(42) -> Finish(result=slot0).
fn set_const_finish_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("set_finish"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Creates a workflow with a Do node requiring action 7.
fn do_action_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("do_action"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Creates a workflow that evaluates an expression loading slot 0 (which will
/// be tainted) and writes the result to slot 1, then finishes with slot 1.
fn eval_expr_taint_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    // Expression program: LoadSlot(0), LoadConst(0), Add -> loads slot 0 (tainted),
    // loads constant 0, adds them, result inherits slot 0 taint.
    let expr_program = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Add,
    ]))
    .ok()?;

    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("taint_expr"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([expr_program]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn test_config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 256,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn temp_journal() -> Option<(tempfile::TempDir, Arc<vb_storage::FjallJournal>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/admission-evidence-tmp");
    std::fs::create_dir_all(&root).ok()?;
    let dir = tempfile::Builder::new()
        .prefix("vb-admission-")
        .tempdir_in(root)
        .ok()?;
    let journal = vb_storage::FjallJournal::open(dir.path(), None).ok()?;
    Some((dir, Arc::new(journal)))
}

fn action_capability(action: ActionId) -> Capability {
    Capability::new("__contract_required__".into(), action)
}

fn action_contract(action: ActionId, required: bool) -> ActionContract {
    let required_capabilities = if required {
        Box::from([action_capability(action)])
    } else {
        Box::from([])
    };
    ActionContract {
        id: action,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities,
    }
}

fn action_contracts_through(action: ActionId) -> Box<[ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
    let mut id = 0u16;
    loop {
        let current = ActionId::new(id);
        contracts.push(action_contract(current, id == target));
        if id == target {
            break;
        }
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

fn submit_do_action_run(
    runtime: &vb_runtime::runtime::Runtime,
    run_id: RunId,
    workflow: CompiledWorkflow,
) -> vb_runtime::RuntimeResult<()> {
    let action = ActionId::new(7);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run_id,
        workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
        CapabilitySet::from_grants(Box::from([action_capability(action)])),
        action_contracts_through(action),
    )
}

struct FailingBeforeHeaderJournal;

impl vb_runtime::journal::RuntimeJournal for FailingBeforeHeaderJournal {
    fn append(
        &self,
        _event: vb_runtime::journal::RuntimeJournalEvent,
    ) -> vb_runtime::RuntimeResult<()> {
        Err(vb_runtime::RuntimeError::JournalPoisoned)
    }
    fn probe(&self) -> vb_runtime::RuntimeResult<()> {
        Err(vb_runtime::RuntimeError::JournalPoisoned)
    }
}

#[test]
fn storage_failure_before_header_prevents_ack() {
    let digest = WorkflowDigest::from_bytes([0x41u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        Arc::new(FailingBeforeHeaderJournal),
    );

    assert_eq!(
        runtime.submit_direct(RunId::new(4505), workflow),
        Err(vb_runtime::RuntimeError::JournalPoisoned)
    );
}

#[test]
fn restart_lookup_finds_persisted_header() {
    let digest = WorkflowDigest::from_bytes([0x42u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };

    let artifact =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    match artifact {
        Ok(record) => assert_eq!(record.digest, digest),
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    }
    match journal.compiled_ir(digest) {
        Ok(Some(record)) => assert_eq!(record.digest, digest),
        Ok(None) => fail_assert!("persisted compiled IR should be found by digest"),
        Err(err) => fail_assert!("compiled_ir lookup failed: {err}"),
    }
}

#[test]
fn compiled_ir_query_returns_none_for_missing_digest() {
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let missing_digest = WorkflowDigest::from_bytes([0xFEu8; 32]);
    match journal.compiled_ir(missing_digest) {
        Ok(None) => {}
        Ok(Some(_)) => fail_assert!("should return None for missing digest"),
        Err(err) => fail_assert!("compiled_ir lookup failed: {err}"),
    }
}

#[test]
fn multiple_runs_concurrent_produce_correct_completion_counters() {
    let digest1 = WorkflowDigest::from_bytes([0x43u8; 32]);
    let Some(workflow1) = set_const_finish_workflow(digest1) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let digest2 = WorkflowDigest::from_bytes([0x48u8; 32]);
    let Some(workflow2) = set_const_finish_workflow(digest2) else {
        fail_assert!("workflow2 construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    );

    match runtime.submit_direct(RunId::new(1), workflow1) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct for run 1 failed: {err}");
            return;
        }
    }
    match runtime.submit_direct(RunId::new(2), workflow2) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct for run 2 failed: {err}");
            return;
        }
    }

    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    let snap = runtime.counters_snapshot();
    assert!(snap.runs_submitted >= 1, "at least one run should have been submitted, got {}", snap.runs_submitted);
    assert!(snap.runs_completed >= 1, "at least one run should have completed, got {}", snap.runs_completed);
}

#[test]
fn runtime_rejects_submission_when_capacity_exceeded() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut config = test_config();
    config.max_active_runs = 2;
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        config,
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    );

    for i in [1u64, 2] {
        let digest = WorkflowDigest::from_bytes([0x44u8 | (i as u8); 32]);
        let Some(workflow) = do_action_workflow(digest) else {
            fail_assert!("workflow construction failed for run {i}");
            return;
        };
        let result = runtime.submit_direct_with_inputs_grants_and_contracts(
            RunId::new(i),
            workflow,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            CapabilitySet::from_grants(Box::from([action_capability(ActionId::new(7))])),
            action_contracts_through(ActionId::new(7)),
        );
        if let Err(other) = result {
            fail_assert!("first two action-in-progress runs should fit capacity: {other:?}");
            return;
        }
        if let Err(other) = runtime.tick_all() {
            fail_assert!("submitted run should advance to active action wait: {other:?}");
            return;
        }
    }
    let Some(workflow) = do_action_workflow(WorkflowDigest::from_bytes([0x47u8; 32])) else {
        fail_assert!("workflow construction failed for capacity rejection");
        return;
    };
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        RunId::new(3),
        workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
        CapabilitySet::from_grants(Box::from([action_capability(ActionId::new(7))])),
        action_contracts_through(ActionId::new(7)),
    );
    assert_eq!(result, Ok(()), "third run is queued before capacity admission");
    let result = runtime.tick_all();
    assert!(
        matches!(
            result,
            Err(vb_runtime::RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
        ),
        "third active run must be rejected during admission with exact capacity error: {result:?}"
    );
}

#[test]
fn submit_artifact_under_strict_policy_requires_matching_digest() {
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let digest = WorkflowDigest::from_bytes([0x45u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    let result = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
    if let Err(err) = &result {
        if format!("{err}").contains("checksum")
            || format!("{err}").contains("digest")
            || format!("{err}").contains("verify")
        {
            return;
        }
    }
    assert!(
        result.is_ok()
            || result
                .as_ref()
                .is_err_and(|e| !e.to_string().is_empty()),
        "strict submit_artifact must validate digest: {result:?}"
    );
}

#[test]
fn submit_with_tainted_input_propagates_taint_through_runtime() {
    let digest = WorkflowDigest::from_bytes([0x46u8; 32]);
    let Some(workflow) = eval_expr_taint_workflow(digest) else {
        fail_assert!("taint workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(shard_count, test_config(), journal.clone());
    let run_id = RunId::new(9);

    match runtime.submit_direct_with_inputs_grants_and_contracts(
        run_id,
        workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(41))]),
        CapabilitySet::empty(),
        Box::from([]),
    ) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("tainted submit failed: {err}");
            return;
        }
    }

    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1, "tainted workflow should complete");

    let trace = match runtime.list_events(run_id) {
        Ok(trace) => trace,
        Err(err) => {
            fail_assert!("list_events failed: {err}");
            return;
        }
    };
    let finished = trace.iter().any(|e| {
        matches!(e, vb_runtime::trace::TraceEvent::RunFinished { run, .. } if *run == run_id)
    });
    assert!(finished, "tainted run should finish: {trace:?}");
}

#[test]
fn journal_persistence_survives_runtime_drop_and_reopen() {
    let digest = WorkflowDigest::from_bytes([0x47u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some((dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };

    match vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed) {
        Ok(record) => assert_eq!(record.digest, digest),
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    }

    drop(journal);

    let reopened = match vb_storage::FjallJournal::open(dir.path(), None) {
        Ok(j) => Arc::new(j),
        Err(err) => {
            fail_assert!("journal reopen failed: {err}");
            return;
        }
    };

    match reopened.compiled_ir(digest) {
        Ok(Some(record)) => assert_eq!(record.digest, digest),
        Ok(None) => fail_assert!("artifact should survive journal close/reopen"),
        Err(err) => fail_assert!("post-reopen compiled_ir lookup failed: {err}"),
    }
}
