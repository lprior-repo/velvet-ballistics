#![forbid(unsafe_code)]
// Emergency deny state survives journal reopen (vb-c4hzw).
// Persists emergency marker, drops all handles, reopens journal,
// and verifies a NEW legal live execution is still blocked at the
// Emergency layer with a full 9-field decision record.

#[path = "vb_c4hzw_emergency_reopen/oracle.rs"]
mod oracle;

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::ActionTicket;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer, PolicyRule, RuntimePolicy,
};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::journal::EmergencyDenyState;
use vb_storage::{FjallConfig, FjallJournal};

use oracle::{
    admission_pin_from_artifact, compute_expected_input_digest, current_live_configured,
};

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

fn build_workflow() -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_c4hzw_emergency_reopen"),
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
            node(1, None, None, CompiledNodeKind::Finish { result: SlotIdx::new(1) }),
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
    let hash = postcard::to_allocvec(&parts).map_err(|e| e.to_string())?;
    let mut parts = parts;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

fn build_contract(action: ActionId) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_c4hzw_emergency_reopen_action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new(
            "velvet.register.write".into(),
            action,
        )]),
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

fn expected_emergency_decision(
    ticket: ActionTicket,
    admission_pin: PinnedAdmissionPolicy,
    current_live: CurrentPolicyIdentity,
    input_digest: PolicyDigest,
) -> DecisionRecord {
    DecisionRecord {
        ticket,
        admission: admission_pin,
        current_live,
        input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

#[test]
fn emergency_deny_survives_journal_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let action = ActionId::new(0);
    let workflow = build_workflow()?;
    let contract = build_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal, &workflow, RuntimePolicy::Strict, &[contract.clone()], &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage.clone());

    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.activate_emergency_deny()?;

    // Drop all handles to force process-level close
    drop(runtime);
    drop(storage);
    drop(journal);

    // Reopen and verify emergency state persisted
    let journal_reopen = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let durable_state = journal_reopen.emergency_deny_state()?;
    assert_eq!(
        durable_state,
        EmergencyDenyState::Active,
        "emergency deny must be durable after reopen"
    );

    let storage_reopen = StorageRuntimeJournal::shared_strict_with_catalog(
        journal_reopen.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime_reopen = Runtime::new(shard_count(1)?, strict_config(), storage_reopen);

    // Submit a NEW legal run and attempt execution
    let run = RunId::new(9_201);
    runtime_reopen.configure_live_policy(
        allowed_digest,
        PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime_reopen.submit_direct_with_inputs_grants_and_contracts(
        run,
        build_workflow()?,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action),
        Box::from([build_contract(action)?]),
    )?;
    runtime_reopen.tick_all()?;

    let ticket = runtime_reopen
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;

    let boundary_before = runtime_reopen.pending_boundary_snapshot(8);

    let result = runtime_reopen.execute_registered_action(
        &mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant,
    );

    let decision = match result {
        Err(vb_runtime::runtime::RegisteredExecutionError::PolicyDenied(record)) => record,
        other => {
            return Err(format!("expected PolicyDenied, got {:?}", other).into());
        }
    };

    // Verify full 9-field decision record
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_live = current_live_configured([0x11u8; 32]);
    let expected = expected_emergency_decision(
        ticket, expected_admission, expected_live, expected_input_digest,
    );

    assert_eq!(
        decision.as_ref(), &expected,
        "decision record must match full 9-field expected record"
    );

    // Ticket unchanged after denied execution
    let ticket_after = runtime_reopen
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after denied execution")?;
    assert_eq!(ticket_after, ticket, "ticket must be unchanged after denied execution");

    // No adapter effect
    assert_eq!(adapter.effect_count()?, 0, "no adapter effect on denied execution");
    assert_eq!(
        adapter.value(scope())?,
        None,
        "adapter value must be None for denied execution"
    );

    // Boundary unchanged after denied execution
    let boundary_after = runtime_reopen.pending_boundary_snapshot(8);
    assert_eq!(
        boundary_after, boundary_before,
        "boundary must be unchanged after denied execution"
    );

    // Verify exact PolicyDecisionEvent persisted in journal
    let events = vb_storage::read_run_events(&journal_reopen, run)?;
    let policy_events: Vec<&vb_core::policy::DecisionRecord> = events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert_eq!(
        policy_events.len(),
        1,
        "exactly one policy decision event must be stored"
    );
    let stored_decision = policy_events.first().ok_or("expected one decision event")?;
    assert_eq!(
        *stored_decision, &expected,
        "stored decision must match expected 9-field record"
    );
    assert_eq!(
        stored_decision.ticket.run, run,
        "stored decision must reference correct run"
    );

    Ok(())
}