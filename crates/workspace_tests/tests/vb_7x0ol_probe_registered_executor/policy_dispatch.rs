use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer, PolicyRule, encode_taint_canonical,
};
use vb_core::value::Taint;
use vb_runtime::runtime::RegisteredExecutionError;

use super::*;

fn compute_expected_input_digest(
    value: i64,
    taint: Taint,
    ticket: &ActionTicket,
) -> Result<PolicyDigest, String> {
    let input_bytes = postcard::to_allocvec(&SlotValue::I64(value))
        .map_err(|e| format!("failed to serialize input: {e}"))?;
    let taint_byte = encode_taint_canonical(taint)
        .ok_or_else(|| "unexpected taint variant".to_string())?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"velvet-ballistics:policy:input:");
    hasher.update(&input_bytes);
    hasher.update(b"velvet-ballistics:policy:taint:");
    hasher.update(&[taint_byte]);
    hasher.update(b"velvet-ballistics:policy:ticket:");
    hasher.update(b"run:");
    hasher.update(&ticket.run.get().to_be_bytes());
    hasher.update(b"step:");
    hasher.update(&ticket.step.get().to_be_bytes());
    hasher.update(b"seq:");
    hasher.update(&ticket.seq.get().to_be_bytes());
    hasher.update(b"action:");
    hasher.update(&ticket.action.get().to_be_bytes());
    hasher.update(b"run_attempt:");
    hasher.update(&ticket.run_attempt.to_be_bytes());
    hasher.update(b"attempt:");
    hasher.update(&ticket.attempt.to_be_bytes());
    hasher.update(b"key:");
    hasher.update(&ticket.idempotency_key.to_be_bytes());
    hasher.update(b"capacity:");
    hasher.update(&ticket.capacity.to_be_bytes());

    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(result.as_bytes());
    Ok(PolicyDigest::from_bytes(output))
}

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
            if !matches!(decision.current_live, CurrentPolicyIdentity::Unconfigured) {
                return Err("unconfigured policy must be recorded in decision".into());
            }
            assert_eq!(decision.ticket, ticket);
            let expected_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
            assert_eq!(decision.input_digest, expected_digest);
        }
        other => return Err(format!("expected PolicyDenied, got: {other:?}")),
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
                other => return Err(format!("expected Configured live policy, got: {other:?}")),
            }
            let expected_admission =
                PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                    assert_ne!(admitted_digest, denied_digest);
                    assert_ne!(admitted_digest, allowed_digest);
                }
                other => return Err(format!("expected Artifact admission pin, got: {other:?}")),
            }
            assert_eq!(decision.ticket, ticket);
            let expected_input_digest =
                compute_expected_input_digest(99, Taint::Clean, &ticket)?;
            assert_eq!(decision.input_digest, expected_input_digest);
        }
        other => return Err(format!("expected PolicyDenied, got: {other:?}")),
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

    if decision_events.is_empty() {
        return Err("expected at least one policy decision event".into());
    }
    let last_decision = decision_events.last()
        .ok_or_else(|| "decision events list should be non-empty".to_string())?;
    assert_eq!(last_decision.outcome, DecisionCode::Allowed);
    assert_eq!(last_decision.layer, PolicyLayer::Live);
    match last_decision.current_live {
        CurrentPolicyIdentity::Configured { digest } => {
            assert_eq!(digest, allowed_digest);
        }
        other => return Err(format!("expected Configured live policy, got: {other:?}")),
    }
    assert_eq!(last_decision.ticket, ticket);
    let expected_input_digest =
        compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    assert_eq!(last_decision.input_digest, expected_input_digest);

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
