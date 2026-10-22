use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, StepIdx, WorkflowDigest};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::executor::ExecutorCatalog;
use vb_storage::executor::FjallRegister;
use vb_storage::{JournalEvent, RunInputBinding, RunPolicyPin};

use crate::legacy_admission_test::{append_events, open_journal};
use crate::legacy_workflow::legacy_ask_action_workflow;

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
        policy: vb_core::policy::RuntimePolicy::Strict,
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
        name: ActionName::new("vb_c4hzw_forged_action")?,
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

#[test]
fn forged_policy_pin_rejected_before_recovery_mutation() -> TestResult {
    let action = ActionId::new(0);
    let run = RunId::new(4_421);
    let temp = tempfile::tempdir()?;

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = legacy_ask_action_workflow(action)?;
    let contract = external_contract(action)?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = open_journal(temp.path())?;
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        vb_core::policy::RuntimePolicy::Strict,
        &[contract],
        &[pack],
    )?;
    drop(journal);

    // Compute the genuine policy digest and construct a distinct wrong pin.
    let real_policy_digest = artifact.policy_digest;
    let wrong_pin = WorkflowDigest::from_bytes([0xFFu8; 32]);
    assert_ne!(real_policy_digest, wrong_pin);

    // Write the legacy prefix with a FORGED policy pin that does not match
    // the accepted artifact's actual policy digest.
    let journal = open_journal(temp.path())?;
    let forged_events = [
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
            policy_pin: RunPolicyPin::Pinned(wrong_pin),
            granted_capabilities: action_grants(action),
            policy: vb_core::policy::RuntimePolicy::Strict,
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
    append_events(&journal, &forged_events)?;

    // Capture full journal state before recovery.
    let journal_before = vb_storage::read_run_events(&journal, run)?;
    assert_eq!(
        journal_before.len(),
        4,
        "forged journal has exactly the legacy prefix events"
    );

    drop(journal);

    // Reopen for recovery.
    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let runtime = Runtime::new(shard_count(1)?, strict_config(), shared);

    // Capture full pending state before recovery.
    let pending_before = runtime.pending_boundary_snapshot(8);

    // Open a real FjallRegister to observe zero effects.
    let register = FjallRegister::open(&temp.path().join("register"), 8)?;

    // Verify register is clean before any recovery attempt.
    assert_eq!(
        register.effect_count()?,
        0,
        "register must be clean before recovery"
    );
    assert_eq!(
        register.value(scope())?,
        None,
        "register must have no value before recovery"
    );

    // Recover product must reject the forged pin.
    assert_eq!(
        runtime.recover_product(run),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_policy_pin_mismatch")
        })
    );

    // Verify full journal is unchanged after first rejected call.
    let journal_after_product = vb_storage::read_run_events(&journal, run)?;
    assert_eq!(
        journal_before, journal_after_product,
        "recover_product with forged policy pin must leave journal unmodified"
    );

    // Verify full pending state is unchanged after first rejected call.
    let pending_after_product = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_before, pending_after_product,
        "recover_product with forged policy pin must leave pending state unmodified"
    );

    // Verify register has no effects after first rejected call.
    assert_eq!(
        register.effect_count()?,
        0,
        "recover_product with forged pin must produce zero register effects"
    );
    assert_eq!(
        register.value(scope())?,
        None,
        "recover_product with forged pin must not set scope value"
    );

    // Recover and resume must also reject the forged pin.
    assert_eq!(
        runtime.recover_and_resume(run),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_policy_pin_mismatch")
        })
    );

    // Verify full journal is unchanged after second rejected call.
    let journal_after_resume = vb_storage::read_run_events(&journal, run)?;
    assert_eq!(
        journal_before, journal_after_resume,
        "recover_and_resume with forged policy pin must leave journal unmodified"
    );

    // Verify full pending state is unchanged after second rejected call.
    let pending_after_resume = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_before, pending_after_resume,
        "recover_and_resume with forged policy pin must leave pending state unmodified"
    );

    // Verify register has no effects after second rejected call.
    assert_eq!(
        register.effect_count()?,
        0,
        "recover_and_resume with forged pin must produce zero register effects"
    );
    assert_eq!(
        register.value(scope())?,
        None,
        "recover_and_resume with forged pin must not set scope value"
    );

    drop(register);
    drop(runtime);
    drop(journal);
    Ok(())
}
