use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{
    DecisionCode, PinnedAdmissionPolicy, PolicyDigest, PolicyRule, RuntimePolicy,
};
use vb_core::value::SlotValue;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::{FjallConfig, FjallJournal, JournalEvent, RunInputBinding, RunPolicyPin};

use crate::legacy_workflow::legacy_ask_action_workflow;
use crate::policy_dispatch::{compute_expected_input_digest, current_live_configured};

type TestResult = Result<(), Box<dyn std::error::Error>>;

use crate::common::runtime_fixture::{shard_count, strict_config};

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

pub(super) fn open_journal(path: &std::path::Path) -> Result<Arc<FjallJournal>, String> {
    Ok(Arc::new(
        FjallJournal::open(path, Some(FjallConfig::default())).map_err(|err| err.to_string())?,
    ))
}

pub(super) fn append_events(
    journal: &FjallJournal,
    events: impl AsRef<[JournalEvent]>,
) -> Result<(), String> {
    for event in events.as_ref() {
        journal
            .append_strict(event)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[test]
fn legacy_unverified_admission_is_rejected_before_resume() -> TestResult {
    let action = ActionId::new(0);
    let run = RunId::new(4_420);
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
    let parts: vb_core::workflow::WorkflowParts = postcard::from_bytes(&artifact.ir)?;
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
    let recovered =
        Runtime::new(shard_count(1)?, strict_config(), shared).expect("valid test configuration");

    let events_before = vb_storage::read_run_events(&journal, run)?;
    let pending_before = recovered.pending_boundary_snapshot(8);
    let register_path = temp.path().join("register");
    let register = FjallRegister::open(&register_path, 8)?;
    assert_eq!(register.effect_count()?, 0);
    assert_eq!(register.value(scope())?, None);
    assert_eq!(
        recovered.recover_and_resume(run),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_missing")
        })
    );
    assert_eq!(vb_storage::read_run_events(&journal, run)?, events_before);
    assert_eq!(recovered.pending_boundary_snapshot(8), pending_before);
    assert_eq!(register.effect_count()?, 0);
    assert_eq!(register.value(scope())?, None);
    drop(register);
    drop(recovered);
    Ok(())
}

#[test]
fn current_valid_admission_denies_live_effect_without_mutation() -> TestResult {
    let action = ActionId::new(0);
    let run = RunId::new(4_423);
    let live_digest = PolicyDigest::from_bytes([0x42u8; 32]);
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
        &[contract.clone()],
        &[pack],
    )?;
    let shared = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime =
        Runtime::new(shard_count(1)?, strict_config(), shared).expect("valid test configuration");
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    let ask_ticket = runtime
        .lookup_pending_ask_ticket(run, StepIdx::ZERO)
        .ok_or("current submission must issue its ask ticket")?;
    runtime.answer_ask(vb_runtime::shard::AskAnswer::with_encoded_len(
        ask_ticket,
        SlotIdx::new(1),
        SlotValue::I64(99),
        vb_core::value::Taint::Clean,
        1,
    ))?;
    runtime.tick_all()?;
    let action_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::new(2).get()))
        .ok_or("current submission must issue its action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, action_ticket, scope())?;
    let (scoped, dispatch) = crate::scoped_grant_fixture::issue_scoped(
        &catalog,
        &artifact,
        &grant,
        action_ticket,
        scope(),
        24,
    )?;
    runtime.configure_live_policy(
        live_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;
    let register_path = temp.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let pending_before = runtime.pending_boundary_snapshot(8);
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::new(2).get()),
        &grant,
        vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]),
    );
    let decision = match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => *decision,
        other => return Err(format!("expected PolicyDenied, got {other:?}").into()),
    };
    let admission_pin = PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(
        artifact.policy_digest.as_bytes(),
    ));
    let expected_input =
        compute_expected_input_digest(99, vb_core::value::Taint::Clean, &action_ticket)?;
    let expected_decision = vb_core::policy::DecisionRecord {
        ticket: action_ticket,
        admission: admission_pin,
        current_live: current_live_configured(live_digest.as_bytes()),
        input_digest: expected_input,
        taint: vb_core::value::Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: vb_core::policy::PolicyLayer::Live,
        emergency_influence: vb_core::policy::InfluenceState::Inactive,
        compat_influence: vb_core::policy::InfluenceState::NotEvaluated,
    };
    assert_eq!(decision, expected_decision);
    assert_eq!(runtime.pending_boundary_snapshot(8), pending_before);
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(adapter.value(scope())?, None);
    let events = vb_storage::read_run_events(&journal, run)?;
    assert!(matches!(
        events.get(1),
        Some(JournalEvent::RunAdmissionBudget {
            input_digest: RunInputBinding::Digest(_),
            policy_pin: RunPolicyPin::Pinned(_),
            ..
        })
    ));
    drop(runtime);
    Ok(())
}
