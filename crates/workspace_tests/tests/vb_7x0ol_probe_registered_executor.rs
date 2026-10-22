#![forbid(unsafe_code)]
// Independent recovery re-review test (vb-7x0ol). Exercises the public
// Runtime::execute_registered_action / Runtime::reconcile_registered_action
// with a real ExecutorCatalog + FjallRegister wired into a public Runtime
// via StorageRuntimeJournal::shared_strict_with_catalog.

mod policy_dispatch;

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{PolicyDigest, PolicyRule, RuntimePolicy, DecisionCode};
use vb_core::value::SlotValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::executor::{ExecutorCatalog, ExecutorError, FjallRegister};
use vb_storage::{FjallConfig, FjallJournal};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn action_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_7x0ol_probe_registered_executor"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|err| err.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|err| err.to_string())
}

fn external_contract(action: ActionId) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_7x0ol_registered_action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new("velvet.register.write".into(), action)]),
    })
}

fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        action,
    )]))
}

#[test]
fn execute_registered_action_commits_one_effect_and_completes_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8_101);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;

    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));

    // Complete the action through the registered completion path.
    runtime.tick_all()?;
    // The pending action must now be consumed (run advanced to Finish).
    assert!(
        runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .is_none(),
        "pending action must be consumed after registered completion"
    );
    Ok(())
}

#[test]
fn reconcile_registered_action_reads_receipt_without_new_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8_102);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    let allowed_digest = PolicyDigest::from_bytes([0x12u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let first = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    assert_eq!(adapter.effect_count()?, 1);

    // Reconcile (still-pending action) must read the same receipt and add no effect.
    let reconciled = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    assert_eq!(reconciled, first);
    assert_eq!(
        adapter.effect_count()?,
        1,
        "reconcile must never add an effect"
    );

    // A missing receipt (fresh register, different ticket) fails closed.
    let second = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let fresh_path = dir.path().join("register_fresh");
    let mut fresh = FjallRegister::open(&fresh_path, 8)?;
    // Same ticket/scope but empty register: reconcile must report missing receipt.
    let missing = fresh.reconcile(&second, ticket, 7);
    assert!(matches!(missing, Err(ExecutorError::ReceiptMissing)));
    Ok(())
}

#[test]
fn execute_registered_action_rejects_mismatched_grant_artifact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8_103);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let _artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(3))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Issue a grant for a DIFFERENT artifact (different contract digest) so the
    // grant.artifact_digest does not match the runtime's admitted artifact.
    let other_contract = external_contract(ActionId::new(0))?;
    // Mutate one field to produce a different artifact while keeping the pack valid.
    let mut divergent = other_contract;
    divergent.max_output_bytes = 65;
    let other_pack = catalog.register_pack(&divergent, scope(), RecoveryClass::ReconcileReceipt)?;
    let other_workflow = action_then_finish_workflow()?;
    let other_artifact = catalog.submit_artifact(
        &journal,
        &other_workflow,
        RuntimePolicy::Strict,
        &[divergent],
        &[other_pack],
    )?;
    let forged_grant = catalog.issue_register_grant(&other_artifact, ticket, scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &forged_grant,
    );
    assert!(matches!(
        result,
        Err(vb_runtime::runtime::RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ));
    assert_eq!(
        adapter.effect_count()?,
        0,
        "rejected grant must not write an effect"
    );
    Ok(())
}

fn submit_and_reach_pending(
    dir: &std::path::Path,
    journal: &Arc<FjallJournal>,
    catalog: &ExecutorCatalog,
    run: RunId,
    action: ActionId,
    recovery: RecoveryClass,
) -> TestResult {
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let pack = catalog.register_pack(&contract, scope(), recovery)?;
    let _artifact = catalog.submit_artifact(
        journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(9))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    assert!(
        runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .is_some(),
        "expected pending action before recovery"
    );
    drop(runtime);
    let _ = dir;
    Ok(())
}

#[test]
fn recovery_of_pending_registered_action_blocked_pack_rejects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8_104);
    let action = ActionId::new(0);
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);

    // Blocked (non-ReconcileReceipt) pack: recovery must fail closed.
    submit_and_reach_pending(
        dir.path(),
        &journal,
        &catalog,
        run,
        action,
        RecoveryClass::Blocked,
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    let result = runtime.recover_and_resume(run);
    match result {
        Err(vb_runtime::RuntimeError::RecoveryCannotResume { reason }) => {
            assert_eq!(reason.as_str(), "registered_recovery_blocked");
        }
        other => {
            return Err(format!(
                "expected RecoveryCannotResume(registered_recovery_blocked), got: {other:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn recovery_of_pending_registered_action_reconcile_pack_is_resumable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8_105);
    let action = ActionId::new(0);
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);

    submit_and_reach_pending(
        dir.path(),
        &journal,
        &catalog,
        run,
        action,
        RecoveryClass::ReconcileReceipt,
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    let product = runtime.recover_product(run)?;
    match product {
        vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(resumable) => {
            assert_eq!(resumable.run_id(), run);
            assert_eq!(
                resumable.boundary_kind(),
                vb_runtime::recovery::RecoveredRunBoundaryKind::PendingAction
            );
        }
        other => {
            return Err(format!(
                "expected Resumable pending-action product, got: {other:?}"
            ));
        }
    }
    Ok(())
}
