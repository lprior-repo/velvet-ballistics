use std::sync::Arc;

use blake3::Hasher;
use vb_core::ActionTicket;
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::runtime::RegisteredExecutionError;
use vb_storage::{FjallConfig, FjallJournal};

use super::*;

// Oracle: compute the expected input digest independently of production code.
// Uses canonical I64 bytes, byte 0 for Clean taint, and all 8 ticket fields
// with domain separation. This matches the production encoding but is the
// test's own oracle, not a call to LivePolicy.evaluate or encode_taint_canonical.

fn serialize_input_bytes(value: i64) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(&SlotValue::I64(value))
        .map_err(|e| format!("failed to serialize input: {e}"))
}

fn taint_byte_clean() -> u8 {
    // Canonical encoding: Clean = 0
    0
}

fn new_policy_hasher() -> Hasher {
    Hasher::new()
}

fn hash_input_domain(hasher: &mut Hasher, input_bytes: &[u8]) {
    hasher.update(b"velvet-ballistics:policy:input:");
    hasher.update(input_bytes);
}

fn hash_taint_domain(hasher: &mut Hasher, taint_byte: u8) {
    hasher.update(b"velvet-ballistics:policy:taint:");
    hasher.update(&[taint_byte]);
}

fn hash_ticket_domain_prefix(hasher: &mut Hasher) {
    hasher.update(b"velvet-ballistics:policy:ticket:");
}

fn hash_run_field(hasher: &mut Hasher, run: u64) {
    hasher.update(b"run:");
    hasher.update(&run.to_be_bytes());
}

fn hash_step_field(hasher: &mut Hasher, step: u16) {
    hasher.update(b"step:");
    hasher.update(&step.to_be_bytes());
}

fn hash_seq_field(hasher: &mut Hasher, seq: u64) {
    hasher.update(b"seq:");
    hasher.update(&seq.to_be_bytes());
}

fn hash_action_field(hasher: &mut Hasher, action: u16) {
    hasher.update(b"action:");
    hasher.update(&action.to_be_bytes());
}

fn hash_run_attempt_field(hasher: &mut Hasher, run_attempt: u16) {
    hasher.update(b"run_attempt:");
    hasher.update(&run_attempt.to_be_bytes());
}

fn hash_attempt_field(hasher: &mut Hasher, attempt: u16) {
    hasher.update(b"attempt:");
    hasher.update(&attempt.to_be_bytes());
}

fn hash_idempotency_key_field(hasher: &mut Hasher, key: u128) {
    hasher.update(b"key:");
    hasher.update(&key.to_be_bytes());
}

fn hash_capacity_field(hasher: &mut Hasher, capacity: u16) {
    hasher.update(b"capacity:");
    hasher.update(&capacity.to_be_bytes());
}

fn finalize_policy_hash(hasher: Hasher) -> PolicyDigest {
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(result.as_bytes());
    PolicyDigest::from_bytes(output)
}

fn compute_expected_input_digest(
    value: i64,
    taint: Taint,
    ticket: &ActionTicket,
) -> Result<PolicyDigest, String> {
    let input_bytes = serialize_input_bytes(value)?;
    let taint_byte = match taint {
        Taint::Clean => taint_byte_clean(),
        other => return Err(format!("unexpected taint variant for oracle: {other:?}")),
    };

    let mut hasher = new_policy_hasher();
    hash_input_domain(&mut hasher, &input_bytes);
    hash_taint_domain(&mut hasher, taint_byte);
    hash_ticket_domain_prefix(&mut hasher);
    hash_run_field(&mut hasher, ticket.run.get());
    hash_step_field(&mut hasher, ticket.step.get());
    hash_seq_field(&mut hasher, ticket.seq.get());
    hash_action_field(&mut hasher, ticket.action.get());
    hash_run_attempt_field(&mut hasher, ticket.run_attempt);
    hash_attempt_field(&mut hasher, ticket.attempt);
    hash_idempotency_key_field(&mut hasher, ticket.idempotency_key);
    hash_capacity_field(&mut hasher, ticket.capacity);

    Ok(finalize_policy_hash(hasher))
}

fn make_expected_decision(
    ticket: ActionTicket,
    admission_pin: PinnedAdmissionPolicy,
    current_live: CurrentPolicyIdentity,
    input_digest: PolicyDigest,
    outcome: DecisionCode,
    layer: PolicyLayer,
) -> vb_core::policy::DecisionRecord {
    vb_core::policy::DecisionRecord {
        ticket,
        admission: admission_pin,
        current_live,
        input_digest,
        taint: Taint::Clean,
        outcome,
        layer,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn admission_pin_from_artifact(policy_digest: [u8; 32]) -> PinnedAdmissionPolicy {
    PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(policy_digest))
}

fn current_live_configured(digest: [u8; 32]) -> CurrentPolicyIdentity {
    CurrentPolicyIdentity::Configured {
        digest: PolicyDigest::from_bytes(digest),
    }
}

fn read_journal_decision_events(
    journal: &Arc<FjallJournal>,
    run: RunId,
) -> Result<Vec<vb_core::policy::DecisionRecord>, Box<dyn std::error::Error>> {
    let events = vb_storage::read_run_events(journal, run)?;
    Ok(events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => {
                Some(decision.clone())
            }
            _ => None,
        })
        .collect())
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

    // No configure_live_policy call — runtime is Unconfigured.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        CurrentPolicyIdentity::Unconfigured,
        expected_input_digest,
        DecisionCode::Denied,
        PolicyLayer::Live,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Full 9-field DecisionRecord oracle comparison
            assert_eq!(
                *decision, expected_decision,
                "full DecisionRecord must match oracle for unconfigured deny"
            );
            // Run consistency: stored decision must reference the same run
            assert_eq!(decision.ticket.run, run);
        }
        other => return Err(format!("expected PolicyDenied, got: {other:?}").into()),
    }

    // Denial: no adapter effect, target value unchanged
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(
        adapter.value(scope())?,
        None,
        "target value must remain unset after policy denial"
    );

    // Exact stored decision list: exactly 1 decision matching oracle
    let stored_decisions = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        stored_decisions.len(),
        1,
        "exactly one policy decision must be stored"
    );
    assert_eq!(
        stored_decisions[0], expected_decision,
        "stored DecisionRecord must match oracle for unconfigured deny"
    );

    // Pending ticket preserved after denial
    let after_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("pending ticket must remain after policy denial")?;
    assert_eq!(
        after_ticket, ticket,
        "exact full ticket must be preserved after denial"
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

    // Configure allowed before scheduling.
    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(99))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Change policy to deny with a different digest before execute.
    let denied_digest = PolicyDigest::from_bytes([0x22u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;

    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(99, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(denied_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Denied,
        PolicyLayer::Live,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Full 9-field DecisionRecord oracle comparison
            assert_eq!(
                *decision, expected_decision,
                "full DecisionRecord must match oracle for policy-change deny"
            );
            // Distinct digests: admission differs from both manual live digests
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted) => {
                    assert_ne!(
                        admitted, denied_digest,
                        "admission must differ from current denied"
                    );
                    assert_ne!(
                        admitted, allowed_digest,
                        "admission must differ from original allowed"
                    );
                }
                _ => unreachable!("admission checked in DecisionRecord equality"),
            }
        }
        other => return Err(format!("expected PolicyDenied, got: {other:?}").into()),
    }

    // Denial: no adapter effect, target value unchanged
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(
        adapter.value(scope())?,
        None,
        "target value must remain unset after policy denial"
    );

    // Exact stored decision list: exactly 1 decision matching oracle
    let stored_decisions = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        stored_decisions.len(),
        1,
        "exactly one policy decision must be stored"
    );
    assert_eq!(
        stored_decisions[0], expected_decision,
        "stored DecisionRecord must match oracle for policy-change deny"
    );
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

    let allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
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

    // Allowed: exact receipt with value 100, effect count 1, target value 100
    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(100));

    // Verify full DecisionRecord persisted with Allowed outcome.
    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
        PolicyLayer::Live,
    );

    let stored_decisions = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        stored_decisions.len(),
        1,
        "exactly one policy decision must be stored"
    );
    assert_eq!(
        stored_decisions[0], expected_decision,
        "full stored DecisionRecord must match oracle for allowed policy"
    );
    // Run and ticket consistency
    assert_eq!(stored_decisions[0].ticket.run, run);
    assert_eq!(stored_decisions[0].ticket, ticket);

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

    let allowed_digest = PolicyDigest::from_bytes([0x44u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(55))]),
        action_grants(action),
        Box::from([contract]),
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
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    assert_eq!(adapter.effect_count()?, 1);

    // Verify the allowed decision was persisted with full record.
    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(55, Taint::Clean, &ticket)?;
    let expected_allowed_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
        PolicyLayer::Live,
    );
    let decisions_before_reconcile = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        decisions_before_reconcile.len(),
        1,
        "exactly one allowed decision must be stored before reconcile"
    );
    assert_eq!(
        decisions_before_reconcile[0], expected_allowed_decision,
        "stored allowed DecisionRecord must match oracle"
    );

    // Change policy to deny after the effect was created.
    let denied_digest = PolicyDigest::from_bytes([0x55u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;

    // Reconcile must read existing receipt and return it, unaffected by new deny.
    let reconciled = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    // Full receipt exact equality
    assert_eq!(
        reconciled, first,
        "reconciled receipt must exactly match original"
    );
    assert_eq!(reconciled.value(), 55);
    assert_eq!(adapter.effect_count()?, 1);

    // Reconcile must NOT create a new policy decision — exact same list.
    let decisions_after_reconcile = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        decisions_after_reconcile.len(),
        1,
        "reconcile must not add a new policy decision"
    );
    assert_eq!(
        decisions_after_reconcile[0], expected_allowed_decision,
        "stored decision list must be exactly the same before/after reconcile"
    );

    Ok(())
}
