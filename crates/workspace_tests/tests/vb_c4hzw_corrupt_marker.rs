#![forbid(unsafe_code)]
// Runtime policy recovery: corrupt emergency marker with unsupported version.
// Verifies that a corrupted emergency deny marker causes precise typed failure
// rather than silent default to inactive.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyRule, RuntimePolicy};
use vb_core::value::SlotValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::RegisteredExecutionError;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::JournalEvent;
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
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
        name: "vb_c4hzw_corrupt_marker".into(),
        semantic_source_identity: None,
        input_slots: Box::new([]),
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
        name: ActionName::new("vb_c4hzw_corrupt_marker")?,
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

fn corrupt_emergency_marker(path: &std::path::Path) -> TestResult {
    let db = fjall::Database::builder(path).cache_size(1 << 20).open()?;
    let ks = db.keyspace("emergency_deny", fjall::KeyspaceCreateOptions::default)?;
    ks.insert(b"active", [99u8])?;
    db.persist(fjall::PersistMode::SyncAll)?;
    drop(ks);
    drop(db);
    Ok(())
}

#[test]
fn corrupt_emergency_marker_causes_precise_typed_failure() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(12_001);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    // Phase 1: Persist accepted artifact only; no runtime execution
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
    drop(journal);

    // Phase 2: Corrupt the emergency_deny keyspace with version 99
    corrupt_emergency_marker(dir.path())?;

    // Phase 3: Reopen fresh runtime, configure policy, submit new legal run
    let journal_reopen = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal_reopen.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    let allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    let contract_for_submit = external_contract(action)?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action),
        Box::from([contract_for_submit]),
    )?;
    runtime.tick_all()?;

    // Derive actual ticket and catalog grant for real artifact
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Record full boundary snapshot BEFORE execute
    let pending_before = runtime.pending_boundary_snapshot(8);

    // Phase 4: Execute -> expect precise typed error from corrupt marker
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    // Verify precise typed error: StorageJournalAppend wrapping EmergencyDenyMarkerUnsupportedVersion { version: 99 }
    let is_correct_error = matches!(
        &result,
        Err(RegisteredExecutionError::Runtime(err))
            if matches!(
                err,
                vb_runtime::RuntimeError::StorageJournalAppend { source }
                    if matches!(
                        source.as_ref(),
                        vb_storage::JournalError::EmergencyDenyMarkerUnsupportedVersion { version }
                            if *version == 99
                    )
            )
    );
    assert!(
        is_correct_error,
        "expected precise typed error JournalError::EmergencyDenyMarkerUnsupportedVersion {{ version: 99 }}, got {result:?}"
    );

    // Verify zero effects
    assert_eq!(
        adapter.effect_count()?,
        0,
        "expected zero effects after corrupt marker failure"
    );
    assert!(
        adapter.value(scope())?.is_none(),
        "register target must be empty after failed execute"
    );

    // Verify boundary unchanged: full snapshot equal, ticket unchanged
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_before, pending_after,
        "pending boundary must be unchanged after corrupt marker failure"
    );
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("ticket disappeared after failure")?;
    assert_eq!(
        ticket, ticket_after,
        "exact ticket must be unchanged after corrupt marker failure"
    );

    // Verify no policy decision event persisted (error before strict decision)
    let events = vb_storage::read_run_events(&journal_reopen, run)?;
    let decision_events: Vec<&JournalEvent> = events
        .iter()
        .filter(|e| matches!(e, JournalEvent::PolicyDecisionEvent { .. }))
        .collect();
    assert!(
        decision_events.is_empty(),
        "no PolicyDecisionEvent expected when read fails before strict decision persist"
    );

    drop(runtime);
    drop(journal_reopen);
    Ok(())
}
