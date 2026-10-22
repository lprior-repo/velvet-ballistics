#![forbid(unsafe_code)]
//! Policy dispatch fail-closed tests (vb-l5ru0).
//!
//! Tests actual public Runtime APIs: live policy evaluation, emergency
//! deny overlay, and strict policy port failure propagation. All three
//! tests use genuine accepted artifacts, pending action tickets, and
//! real FjallRegister effects (or lack thereof). No fake state or cache
//! substitutes.

#[path = "vb_policy_dispatch_fail_closed/fixtures.rs"]
mod fixtures;

use std::sync::Arc;

use fixtures::{
    action_grants, action_then_finish_workflow, compute_expected_input_digest,
    external_contract, scope, strict_config, FailingPolicyPortJournal,
};
use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState,
    PinnedAdmissionPolicy, PolicyDigest, PolicyLayer, PolicyRule, RuntimePolicy,
};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::admission::{
    RegisteredExecutionError, RegisteredReconcileError, RuntimeAdmissionError,
};
use vb_runtime::runtime::Runtime;
use vb_runtime::{RuntimeError, ShardConfig};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_storage::executor::{ExecutorCatalog, FjallRegister, RecoveryClass};
use vb_storage::{FjallConfig, FjallJournal};

fn shard_count(value: usize) -> Result<std::num::NonZeroUsize, String> {
    std::num::NonZeroUsize::new(value)
        .ok_or_else(|| format!("invalid shard count {value}"))
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test 1: Actual current Suspended policy before execute on a genuine
/// accepted artifact / pending action. Expect typed PolicyDenied with
/// full 9-field DecisionRecord, zero effects, unchanged pending state.
#[test]
fn suspended_policy_denies_execution_with_full_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_001);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
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

    // Configure a Suspended live policy for this action.
    let suspended_digest = PolicyDigest::from_bytes([0x99u8; 32]);
    runtime.configure_live_policy(
        suspended_digest,
        PolicyRule {
            action,
            code: DecisionCode::Suspended,
        },
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

    // Execute with suspended policy — must be rejected before any effect.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            assert_eq!(decision.outcome, DecisionCode::Suspended);
            assert_eq!(decision.layer, PolicyLayer::Live);

            // Verify current_live is the Suspended config we set.
            match decision.current_live {
                CurrentPolicyIdentity::Configured { digest } => {
                    assert_eq!(digest, suspended_digest);
                }
                other => {
                    return Err(format!(
                        "expected Configured live policy with Suspended rule, got: {other:?}"
                    )
                    .into());
                }
            }

            // Verify admission pin matches the artifact's policy digest.
            let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                }
                other => {
                    return Err(format!("expected Artifact admission pin, got: {other:?}").into());
                }
            }

            // Verify ticket identity (full 8-field).
            assert_eq!(decision.ticket, ticket);

            // Verify exact input digest.
            let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
            assert_eq!(decision.input_digest, expected_input_digest);

            // Verify taint field.
            assert_eq!(decision.taint, Taint::Clean);

            // Verify influence states (no emergency, no compat).
            assert_eq!(decision.emergency_influence, InfluenceState::Inactive);
            assert_eq!(decision.compat_influence, InfluenceState::Inactive);
        }
        other => {
            return Err(format!(
                "expected PolicyDenied with Suspended decision, got: {other:?}"
            )
            .into());
        }
    }

    // Real FjallRegister effect count is zero.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "suspended policy must not produce any effect"
    );

    // Pending ticket unchanged.
    assert!(
        runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .is_some(),
        "pending ticket must remain after suspended policy rejection"
    );

    // Exactly one stored decision event.
    let events = vb_storage::read_run_events(&journal, run)?;
    let decision_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert_eq!(
        decision_events.len(),
        1,
        "exactly one policy decision event must be stored"
    );
    assert_eq!(decision_events[0].outcome, DecisionCode::Suspended);
    assert_eq!(decision_events[0].layer, PolicyLayer::Live);

    Ok(())
}

/// Test 2: Configure Allowed, submit, tick, activate emergency deny via
/// actual dedicated store persist, then execute. Expect typed Emergency
/// Denied decision with Active influence, zero effects, unchanged pending.
/// Show that configuring Allowed afterwards cannot loosen the overlay.
#[test]
fn emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_002);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
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

    // Configure Allowed before scheduling.
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Activate emergency deny via actual dedicated store persist.
    runtime.activate_emergency_deny()?;

    // Execute — emergency overlay must reject even though live config is Allowed.
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);

            // Verify emergency influence is Active.
            assert_eq!(decision.emergency_influence, InfluenceState::Active);

            // Verify compat influence is Inactive.
            assert_eq!(decision.compat_influence, InfluenceState::Inactive);

            // Verify ticket and input digest are correct.
            assert_eq!(decision.ticket, ticket);
            let expected_input_digest =
                compute_expected_input_digest(100, Taint::Clean, &ticket)?;
            assert_eq!(decision.input_digest, expected_input_digest);
            assert_eq!(decision.taint, Taint::Clean);

            // Verify current_live is still the Allowed config.
            match decision.current_live {
                CurrentPolicyIdentity::Configured { digest } => {
                    assert_eq!(digest, allowed_digest);
                }
                other => {
                    return Err(format!(
                        "expected Configured live policy (Allowed), got: {other:?}"
                    )
                    .into());
                }
            }

            // Verify admission pin is the artifact's digest.
            let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                }
                other => {
                    return Err(format!("expected Artifact admission pin, got: {other:?}").into());
                }
            }
        }
        other => {
            return Err(format!(
                "expected PolicyDenied with Emergency decision, got: {other:?}"
            )
            .into());
        }
    }

    // Zero effects.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "emergency deny must not produce any effect"
    );

    // Pending ticket unchanged.
    assert!(
        runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .is_some(),
        "pending ticket must remain after emergency deny"
    );

    // Exactly one stored decision event.
    let events = vb_storage::read_run_events(&journal, run)?;
    let decision_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert_eq!(decision_events.len(), 1);
    assert_eq!(decision_events[0].outcome, DecisionCode::Denied);
    assert_eq!(decision_events[0].layer, PolicyLayer::Emergency);

    // Show that configuring Allowed after emergency cannot loosen the overlay.
    let later_allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        later_allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
        }
        other => {
            return Err(format!(
                "expected PolicyDenied after re-configure Allowed, got: {other:?}"
            )
            .into());
        }
    }

    Ok(())
}

/// Test 3: Strict-port typed failure via test-only RuntimeJournal wrapper.
/// Real store, journal, catalog, grants, register remain actual. Override
/// only append_policy_decision_strict to inject a strict port failure.
#[test]
fn strict_port_failure_returns_typed_runtime_error_with_zero_effects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_003);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    // Real base FjallJournal; wrapper overrides strict port only.
    let real_journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let failing = Arc::new(FailingPolicyPortJournal(real_journal.clone()));

    let artifact = catalog.submit_artifact(
        &real_journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        failing.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    runtime.configure_live_policy(
        PolicyDigest::from_bytes([0x77u8; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(200))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Execute with failing strict port.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    match result {
        Err(RegisteredExecutionError::RuntimeError(err)) => {
            let msg = err.to_string();
            assert!(
                msg.contains("failed to persist policy decision record via strict port"),
                "expected strict port failure message, got: {msg}"
            );
        }
        other => {
            return Err(format!(
                "expected RuntimeError from strict port failure, got: {other:?}"
            )
            .into());
        }
    }

    // Real effect count is zero.
    assert_eq!(adapter.effect_count()?, 0);

    // Ticket still pending.
    assert!(
        runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .is_some()
    );

    Ok(())
}