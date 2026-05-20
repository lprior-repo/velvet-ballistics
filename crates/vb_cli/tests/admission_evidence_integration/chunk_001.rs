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
