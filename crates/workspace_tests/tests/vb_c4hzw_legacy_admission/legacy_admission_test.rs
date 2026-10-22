use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyRule, RuntimePolicy};
use vb_core::value::SlotValue;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_runtime::shard::ShardConfig;
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::{FjallConfig, FjallJournal, JournalEvent, RunInputBinding, RunPolicyPin};

use crate::legacy_workflow::legacy_ask_action_workflow;
use crate::policy_dispatch::{
    admission_pin_unverified, compute_expected_input_digest, current_live_configured,
    make_expected_decision,
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

fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

fn external_contract(action: ActionId) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_c4hzw_legacy_action")?,
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

fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        action,
    )]))
}

fn open_journal(path: &std::path::Path) -> Result<Arc<FjallJournal>, String> {
    Ok(Arc::new(
        FjallJournal::open(path, Some(FjallConfig::default()))
            .map_err(|err| err.to_string())?,
    ))
}

fn append_events(journal: &FjallJournal, events: impl AsRef<[JournalEvent]>) -> Result<(), String> {
    for event in events.as_ref() {
        journal
            .append_strict(event)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[test]
fn legacy_unverified_admission_blocks_new_live_effect() -> TestResult {
    let action = ActionId::new(0);
    let run = RunId::new(4_420);
    let allowed_digest = PolicyDigest::from_bytes([0x41u8; 32]);
    let temp = tempfile::tempdir()?;

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = legacy_ask_action_workflow(action)?;
    let contract = external_contract(action)?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = open_journal(temp.path())?;
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract],
        &[pack],
    )?;
    let parts: vb_core::workflow::WorkflowParts =
        postcard::from_bytes(&artifact.ir)?;
    assert_eq!(parts.digest, workflow.digest());

    let legacy_events = [
        JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow: workflow.digest(),
        },
        JournalEvent::RunAdmission {
            input_digest: RunInputBinding::LegacyUnbound,
            run,
            seq: vb_storage::EventSeq::new(1),
            artifact_digest: artifact.digest,
            policy_pin: RunPolicyPin::Unverified,
            granted_capabilities: action_grants(action),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: vb_storage::EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: vb_storage::EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];
    append_events(&journal, &legacy_events)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    let recovery = recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?;
    match recovery {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        }
        other => {
            return Err(format!("expected open-ask resumable product, got {other:?}").into());
        }
    }
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    let ask_ticket = recovered
        .lookup_pending_ask_ticket(run, StepIdx::ZERO)
        .ok_or("recovered open ask must issue its persisted ticket")?;
    recovered
        .answer_ask(vb_runtime::shard::AskAnswer::with_encoded_len(
            ask_ticket,
            SlotIdx::new(1),
            SlotValue::I64(99),
            vb_core::value::Taint::Clean,
            1,
        ))
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    let action_ticket = recovered
        .lookup_pending_action_ticket(run, u64::from(StepIdx::new(2).get()))
        .ok_or("recovered Do action must issue its ticket")?;

    let grant = catalog.issue_register_grant(&artifact, action_ticket, scope())?;

    recovered.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    let register_path = temp.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;

    let pending_before = recovered.pending_boundary_snapshot(8);
    let result = recovered.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::new(2).get()),
        &grant,
    );
    let decision = match result {
        Err(RegisteredExecutionError::PolicyDenied(dec)) => *dec,
        other => {
            return Err(format!("expected PolicyDenied, got: {other:?}").into());
        }
    };
    assert_eq!(recovered.pending_boundary_snapshot(8), pending_before);

    assert_eq!(decision.outcome, DecisionCode::Denied);
    assert_eq!(decision.layer, vb_core::policy::PolicyLayer::Admission);
    assert_eq!(decision.admission, admission_pin_unverified());
    assert_eq!(
        decision.current_live,
        current_live_configured(allowed_digest.as_bytes())
    );
    assert_eq!(decision.ticket.run, run);
    assert_eq!(decision.ticket.step, StepIdx::new(2));

    let expected_digest = compute_expected_input_digest(99, vb_core::value::Taint::Clean, &action_ticket)?;
    assert_eq!(decision.input_digest, expected_digest);

    let expected_decision = make_expected_decision(
        action_ticket,
        admission_pin_unverified(),
        current_live_configured(allowed_digest.as_bytes()),
        expected_digest,
        DecisionCode::Denied,
    );
    assert_eq!(decision, expected_decision);

    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(
        adapter.value(scope())?,
        None,
        "target value must remain unset after admission denial"
    );

    let after_ticket = recovered
        .lookup_pending_action_ticket(run, u64::from(StepIdx::new(2).get()))
        .ok_or("pending ticket must remain after admission denial")?;
    assert_eq!(after_ticket, action_ticket);

    let snapshot = recovered.pending_boundary_snapshot(8);
    let shards = snapshot.shards();
    let pending_actions: Vec<_> = shards
        .iter()
        .flat_map(|s| s.pending_actions())
        .cloned()
        .collect();
    assert_eq!(
        pending_actions.len(),
        1,
        "exactly one pending action after ask answer"
    );
    let first_pending = pending_actions
        .first()
        .ok_or("expected at least one pending action")?;
    assert_eq!(first_pending.ticket(), action_ticket);

    let stored = vb_storage::read_run_events(&journal, run)?;
    let decisions: Vec<_> = stored
        .iter()
        .filter_map(|e| match e {
            JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(decisions, vec![expected_decision]);

    drop(recovered);
    Ok(())
}
