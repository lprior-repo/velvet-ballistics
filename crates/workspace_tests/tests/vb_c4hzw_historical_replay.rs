#![forbid(unsafe_code)]

#[path = "vb_c4hzw_historical_replay/fixtures.rs"]
mod fixtures;

#[path = "vb_c4hzw_historical_replay/policy_dispatch.rs"]
mod policy_dispatch;

use std::sync::Arc;

use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyRule, RuntimePolicy};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::runtime::Runtime;
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::journal::EmergencyDenyState;
use vb_storage::{FjallConfig, FjallJournal};

use fixtures::{action_grants, build_contract, build_workflow, scope, shard_count, strict_config};
use vb_core::action_pack::RecoveryClass;

fn run_historical_replay_test() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_200);
    let action = ActionId::new(0);

    let workflow = build_workflow()?;
    let contract = build_contract(action)?;
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
        &[pack.clone()],
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
        Box::from([contract.clone()]),
    )?;

    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, 0)
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket.clone(), scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = runtime.execute_registered_action(&mut adapter, run, 0, &grant)?;

    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(100));

    // Crash window: real registered effect executed once, durable receipt recorded.
    // Do NOT tick completion. Journal has scheduled PendingAction + historical Allowed.
    // Recovery reconstructs PendingAction without re-executing nondeterministic Do.

    let product_before = runtime.recover_product(run)?;
    match &product_before {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::PendingAction,
                "pending action crash window must recover as resumable pending action"
            );
        }
        other => {
            panic!(
                "expected Resumable with PendingAction boundary, got: {:?}",
                other
            );
        }
    }
    let events_before = vb_storage::read_run_events(&journal, run)?;
    let pending_before = runtime.pending_boundary_snapshot(8);

    let stored_decisions_before = policy_dispatch::read_journal_decision_events(&journal, run)?;
    assert_eq!(stored_decisions_before.len(), 1);

    let denied_digest = PolicyDigest::from_bytes([0x55u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;

    runtime.activate_emergency_deny()?;

    let deny_state = journal.emergency_deny_state()?;
    assert_eq!(deny_state, EmergencyDenyState::Active);

    let product_after = runtime.recover_product(run)?;
    let events_after = vb_storage::read_run_events(&journal, run)?;
    let pending_after = runtime.pending_boundary_snapshot(8);

    assert_eq!(
        product_before, product_after,
        "recovery product must be identical before and after current deny"
    );
    assert_eq!(
        product_before.summary(),
        product_after.summary(),
        "recovery summary must be identical before and after current deny"
    );

    assert_eq!(
        events_before, events_after,
        "full journal must be unchanged after current deny"
    );

    assert_eq!(
        pending_before, pending_after,
        "pending state must be unchanged after current deny"
    );

    assert_eq!(
        adapter.effect_count()?,
        1,
        "effect count must remain 1 after current deny"
    );
    assert_eq!(
        adapter.value(scope())?,
        Some(100),
        "register value must remain 100 after current deny"
    );

    let stored_decisions = policy_dispatch::read_journal_decision_events(&journal, run)?;
    assert_eq!(stored_decisions.len(), 1);
    let stored = stored_decisions
        .first()
        .ok_or("expected exactly one decision")?;
    assert_eq!(
        stored.outcome,
        DecisionCode::Allowed,
        "historical decision must remain Allowed"
    );

    let input_digest = policy_dispatch::compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    let expected = policy_dispatch::make_expected_decision(
        ticket,
        policy_dispatch::admission_pin_from_artifact(artifact.policy_digest.as_bytes()),
        policy_dispatch::current_live_configured(allowed_digest.as_bytes()),
        input_digest,
        DecisionCode::Allowed,
    );
    assert_eq!(
        stored_decisions,
        vec![expected],
        "historical decision record must match independent oracle"
    );

    Ok(())
}

#[test]
fn historical_replay_preserves_state_under_current_deny() -> Result<(), Box<dyn std::error::Error>>
{
    run_historical_replay_test()
}
