use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer, PolicyRule,
};
use vb_runtime::runtime::RegisteredExecutionError;

use super::*;

#[test]
fn unconfigured_policy_denies_execution() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_101);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal, &workflow, RuntimePolicy::Strict, &[contract.clone()], &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(), Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action), Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // No configure_live_policy call — runtime is Unconfigured.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(&mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant);

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Live);
            assert!(
                matches!(decision.current_live, CurrentPolicyIdentity::Unconfigured),
                "unconfigured policy must be recorded in decision"
            );
            assert_eq!(decision.ticket, ticket);
            assert_ne!(decision.input_digest, PolicyDigest::from_bytes([0u8; 32]));
        }
        other => panic!("expected PolicyDenied, got: {other:?}"),
    }

    assert_eq!(adapter.effect_count()?, 0);
    assert!(
        runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get())).is_some(),
        "pending ticket must remain after policy denial"
    );
    Ok(())
}

#[test]
fn after_scheduling_policy_change_to_deny() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_102);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal, &workflow, RuntimePolicy::Strict, &[contract.clone()], &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(), Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    // Configure allowed before scheduling.
    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    runtime.configure_live_policy(
        allowed_digest, PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(99))]),
        action_grants(action), Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Change policy to deny with a different digest before execute.
    let denied_digest = PolicyDigest::from_bytes([0x22u8; 32]);
    runtime.configure_live_policy(
        denied_digest, PolicyRule { action, code: DecisionCode::Denied },
    )?;

    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(&mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant);

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Live);
            match decision.current_live {
                CurrentPolicyIdentity::Configured { digest } => {
                    assert_eq!(digest, denied_digest);
                    assert_ne!(digest, allowed_digest);
                }
                other => panic!("expected Configured live policy, got: {other:?}"),
            }
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, allowed_digest);
                    assert_ne!(admitted_digest, denied_digest);
                }
                other => panic!("expected Artifact admission pin, got: {other:?}"),
            }
            assert_eq!(decision.ticket, ticket);
        }
        other => panic!("expected PolicyDenied, got: {other:?}"),
    }

    assert_eq!(adapter.effect_count()?, 0);
    Ok(())
}

#[test]
fn allowed_policy_permits_effect_and_records_decision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_103);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal, &workflow, RuntimePolicy::Strict, &[contract.clone()], &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(), Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    let allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        allowed_digest, PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action), Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = runtime.execute_registered_action(
        &mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant,
    )?;

    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(100));

    // Verify decision event persisted with Allowed outcome.
    let events = vb_storage::public_api::read_run_events(&journal, run)?;
    let decision_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();

    assert!(!decision_events.is_empty(), "expected at least one policy decision event");
    let last_decision = decision_events.last().expect("events non-empty");
    assert_eq!(last_decision.outcome, DecisionCode::Allowed);
    assert_eq!(last_decision.layer, PolicyLayer::Live);
    match last_decision.current_live {
        CurrentPolicyIdentity::Configured { digest } => {
            assert_eq!(digest, allowed_digest);
        }
        other => panic!("expected Configured live policy, got: {other:?}"),
    }
    assert_eq!(last_decision.ticket, ticket);
    assert_ne!(last_decision.input_digest, PolicyDigest::from_bytes([0u8; 32]));

    Ok(())
}

#[test]
fn reconcile_ignores_policy_change_after_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_104);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(dir.path(), Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal, &workflow, RuntimePolicy::Strict, &[contract.clone()], &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(), Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    let allowed_digest = PolicyDigest::from_bytes([0x44u8; 32]);
    runtime.configure_live_policy(
        allowed_digest, PolicyRule { action, code: DecisionCode::Allowed },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(55))]),
        action_grants(action), Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Execute allowed: creates actual receipt but do not tick to completion.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let first = runtime.execute_registered_action(
        &mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant,
    )?;
    assert_eq!(adapter.effect_count()?, 1);

    // Change policy to deny after the effect was created.
    let denied_digest = PolicyDigest::from_bytes([0x55u8; 32]);
    runtime.configure_live_policy(
        denied_digest, PolicyRule { action, code: DecisionCode::Denied },
    )?;

    // Reconcile must read existing receipt and return it, unaffected by new deny.
    let reconciled = runtime.reconcile_registered_action(
        &mut adapter, run, u64::from(StepIdx::ZERO.get()), &grant,
    )?;
    assert_eq!(reconciled, first);
    assert_eq!(reconciled.value(), 55);
    assert_eq!(adapter.effect_count()?, 1);
    Ok(())
}
