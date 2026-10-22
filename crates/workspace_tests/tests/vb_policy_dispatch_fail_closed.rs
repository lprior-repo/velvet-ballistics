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

use fixtures::*;

use std::sync::Arc;
use vb_core::action_pack::RecoveryClass;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer, PolicyRule, RuntimePolicy,
};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::{EventSeq, FjallConfig, FjallJournal};

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

    // Compute expected full DecisionRecord independently of returned decision.
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    let expected = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: suspended_digest,
        },
        input_digest: expected_input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Suspended,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before execute for fail-closed verification.
    let pending_before = runtime.pending_boundary_snapshot(8);

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
            // Retain existing individual field checks.
            assert_eq!(decision.outcome, DecisionCode::Suspended);
            assert_eq!(decision.layer, PolicyLayer::Live);

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

            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                }
                other => {
                    return Err(format!("expected Artifact admission pin, got: {other:?}").into());
                }
            }

            assert_eq!(decision.ticket, ticket);
            assert_eq!(decision.input_digest, expected_input_digest);
            assert_eq!(decision.taint, Taint::Clean);
            assert_eq!(decision.emergency_influence, InfluenceState::Inactive);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected,
                "returned decision must match full expected record"
            );
        }
        other => {
            return Err(
                format!("expected PolicyDenied with Suspended decision, got: {other:?}").into(),
            );
        }
    }

    // Real FjallRegister effect count is zero.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "suspended policy must not produce any effect"
    );

    // Pending state unchanged after suspended policy rejection.
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after, pending_before,
        "suspended policy rejection must not alter pending state"
    );

    // Pending ticket unchanged — compare exact ticket identity.
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after rejection")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket),
        "pending ticket must remain identical after suspended policy rejection"
    );

    // Exactly one stored decision event matching full expected record.
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
    assert_eq!(
        decision_events,
        vec![&expected],
        "stored decision event must exactly match expected full record"
    );

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

    // Compute expected full DecisionRecord for first emergency denial attempt.
    let expected_input_digest_first = compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    let expected_first = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: allowed_digest,
        },
        input_digest: expected_input_digest_first,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before first execute.
    let pending_before_first = runtime.pending_boundary_snapshot(8);

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
            // Retain existing individual field checks.
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
            assert_eq!(decision.ticket, ticket);
            assert_eq!(decision.input_digest, expected_input_digest_first);
            assert_eq!(decision.taint, Taint::Clean);

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

            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                }
                other => {
                    return Err(format!("expected Artifact admission pin, got: {other:?}").into());
                }
            }

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected_first,
                "first emergency denial record must match full expected"
            );
        }
        other => {
            return Err(
                format!("expected PolicyDenied with Emergency decision, got: {other:?}").into(),
            );
        }
    }

    // Zero effects.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "emergency deny must not produce any effect"
    );

    // Pending state unchanged after first emergency deny.
    let pending_after_first = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after_first, pending_before_first,
        "first emergency deny must not alter pending state"
    );

    // Pending ticket unchanged — compare exact identity.
    let ticket_after_first = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after first deny")?;
    assert_eq!(
        Some(ticket_after_first),
        Some(ticket),
        "pending ticket must remain identical after first emergency deny"
    );

    // Show that configuring Allowed after emergency cannot loosen the overlay.
    let later_allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        later_allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    let second_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket still present")?;
    let second_grant = catalog.issue_register_grant(&artifact, second_ticket, scope())?;

    // Compute expected full record for second attempt.
    let second_input_value = 100; // same workflow, same input
    let expected_input_digest_second =
        compute_expected_input_digest(second_input_value, Taint::Clean, &second_ticket)?;
    let expected_second = DecisionRecord {
        ticket: second_ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: later_allowed_digest,
        },
        input_digest: expected_input_digest_second,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before second execute.
    let pending_before_second = runtime.pending_boundary_snapshot(8);

    let register_path2 = dir.path().join("register2");
    let mut adapter2 = FjallRegister::open(&register_path2, 8)?;
    let result2 = runtime.execute_registered_action(
        &mut adapter2,
        run,
        u64::from(StepIdx::ZERO.get()),
        &second_grant,
    );

    match result2 {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Retain existing individual field checks.
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
            assert_eq!(decision.ticket, second_ticket);
            assert_eq!(decision.input_digest, expected_input_digest_second);
            assert_eq!(decision.taint, Taint::Clean);

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected_second,
                "second emergency denial record must match full expected"
            );
        }
        other => {
            return Err(format!(
                "expected second execute to also be denied by emergency overlay, got: {other:?}"
            )
            .into());
        }
    }
    assert_eq!(
        adapter2.effect_count()?,
        0,
        "second execute must also have zero effects"
    );

    // Pending state unchanged after second emergency deny.
    let pending_after_second = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after_second, pending_before_second,
        "second emergency deny must not alter pending state"
    );

    // Both stored decision events must exactly match the two expected records in order.
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
        2,
        "exactly two policy decision events must be stored"
    );
    assert_eq!(
        decision_events,
        vec![&expected_first, &expected_second],
        "stored decision events must exactly match expected records in order"
    );

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

    // Use a test wrapper that overrides only append_policy_decision_strict.
    let wrapped = Arc::new(FailingPolicyPortJournal::new(
        journal.clone(),
        catalog.clone(),
    ));
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), wrapped.clone());

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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(77))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Save pending state before execute for fail-closed verification.
    let pending_before = runtime.pending_boundary_snapshot(8);

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    // Expect exact typed error propagation.
    match result {
        Err(RegisteredExecutionError::Runtime(runtime_err)) => {
            assert!(
                matches!(
                    runtime_err,
                    RuntimeError::UnsupportedOperation { operation }
                        if operation == "test_policy_decision_failure"
                ),
                "expected exact UnsupportedOperation error, got: {runtime_err:?}"
            );
        }
        other => {
            return Err(format!(
                "expected RegisteredExecutionError::Runtime with UnsupportedOperation, got: {other:?}"
            )
            .into());
        }
    }

    // Zero effects on the real FjallRegister.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "strict port failure must not produce any effect"
    );

    // Pending state unchanged after strict port failure.
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after, pending_before,
        "strict port failure must not alter pending state"
    );

    // Pending ticket unchanged — compare exact identity.
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after failure")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket),
        "pending ticket must remain identical after strict port failure"
    );

    // No stored decisions (strict port failed before any event).
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
        0,
        "strict port failure must not store any policy decision"
    );

    // Verify the port was called exactly once with the expected decision
    // and the actual current sequence number.
    let expected_seq = {
        let events = vb_storage::read_run_events(&journal, run)?;
        let last = events
            .last()
            .ok_or("expected at least one prior event for seq computation")?;
        let next = last
            .seq()
            .get()
            .checked_add(1)
            .ok_or("expected seq arithmetic to stay within u64")?;
        EventSeq::new(next)
    };

    let expected_input_digest = compute_expected_input_digest(77, Taint::Clean, &ticket)?;
    let expected_record = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(
            artifact.policy_digest.as_bytes(),
        )),
        current_live: CurrentPolicyIdentity::Configured {
            digest: allowed_digest,
        },
        input_digest: expected_input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };

    assert!(
        wrapped.policy_port_called(),
        "strict port must have been called"
    );
    let captured = wrapped.captured_decision()?.ok_or("no decision captured")?;
    assert_eq!(
        captured, expected_record,
        "single-decision view must match full oracle"
    );
    assert_eq!(
        wrapped.captured_calls()?,
        vec![(expected_record, expected_seq)],
        "exact single call with full record and correct seq must be captured"
    );

    Ok(())
}
