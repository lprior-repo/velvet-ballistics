#![forbid(unsafe_code)]
mod artifact_evidence_support;
use artifact_evidence_support::*;
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::{
    ActionContract, ActionId, ActionTicket, Capability, RunId, RuntimePolicy, SeqNo, StepIdx,
};
use vb_storage::executor::{ExecutorCatalog, ExecutorError, FjallRegister};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}
fn external_contract() -> Result<ActionContract, Box<dyn std::error::Error>> {
    let mut contract = contract()?;
    contract.required_capabilities =
        Box::new([Capability::new("velvet.register.write".into(), contract.id)]);
    Ok(contract)
}
fn ticket(seq: u64) -> ActionTicket {
    let run = RunId::new(100);
    let seq = SeqNo::new(seq);
    let action = ActionId::new(0);
    ActionTicket {
        run,
        step: StepIdx::new(1),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
    }
}

#[test]
fn registered_suite_executes_duplicate_conflicts_and_reopen() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let report = catalog.conformance().report();
    assert_eq!(report.first, report.duplicate);
    assert_eq!(report.first, report.reopened);
    assert_eq!(
        (report.effects_before_reopen, report.effects_after_reopen),
        (1, 1)
    );
    assert_eq!(report.target_after_reopen, 42);
    assert_eq!(report.checked_cases.len(), 4);
    let audit = postcard::to_allocvec(report)?;
    let restored: vb_storage::executor::ConformanceReport = postcard::from_bytes(&audit)?;
    assert_eq!(&restored, report);
    // No constructor accepts this deserialized report to mint ExecutedConformance.
    let fresh = ExecutorCatalog::run_registered_conformance()?;
    assert_eq!(fresh.conformance().digest(), catalog.conformance().digest());
    Ok(())
}

#[test]
fn strict_pack_requires_registered_context_and_raw_metadata_cannot_self_certify() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract()?;
    let workflow = action_workflow()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    assert!(matches!(
        vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[contract.clone()]
        ),
        Err(vb_storage::JournalError::MissingRequiredProofFlag {
            flag: "executor_conformance"
        })
    ));
    let artifact =
        catalog.prepare_artifact(&workflow, RuntimePolicy::Strict, &[contract], &[pack])?;
    assert!(matches!(
        vb_storage::admission::verify_accepted_artifact(&artifact),
        Err(vb_storage::JournalError::ArtifactMalformed)
    ));
    assert_eq!(
        catalog.verify_artifact(&artifact, RuntimePolicy::Strict)?,
        workflow
    );
    let mut forged = artifact;
    let vb_core::artifact::ArtifactEvidence::Packed(evidence) = &mut forged.verification.evidence
    else {
        return Err("missing packs".into());
    };
    let first = evidence.packs.first_mut().ok_or("missing pack")?;
    first.duplicate_ticket_test = vb_core::action_pack::ConformanceReportDigest::from_digest(
        vb_core::WorkflowDigest::from_bytes([9; 32]),
    );
    rebind(&mut forged)?;
    assert!(matches!(
        catalog.verify_artifact(&forged, RuntimePolicy::Strict),
        Err(ExecutorError::PackMismatch)
    ));
    Ok(())
}

#[test]
fn duplicate_and_reopen_return_one_actual_effect_and_capacity_never_evicts() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract()?;
    let workflow = action_workflow()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let artifact =
        catalog.prepare_artifact(&workflow, RuntimePolicy::Strict, &[contract], &[pack])?;
    let grant = catalog.issue_register_grant(&artifact, ticket(1), scope())?;
    let dir = tempfile::tempdir()?;
    let first = {
        let mut adapter = FjallRegister::open(dir.path(), 2)?;
        let receipt = adapter.execute(&grant, ticket(1), 42)?;
        assert_eq!(adapter.execute(&grant, ticket(1), 42)?, receipt);
        assert!(matches!(
            adapter.execute(&grant, ticket(1), 43),
            Err(ExecutorError::ReceiptConflict)
        ));
        assert_eq!(adapter.effect_count()?, 1);
        receipt
    };
    let mut adapter = FjallRegister::open(dir.path(), 2)?;
    assert_eq!(adapter.reconcile(&grant, ticket(1), 42)?, first);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    let second = catalog.issue_register_grant(&artifact, ticket(2), scope())?;
    assert!(matches!(
        adapter.reconcile(&second, ticket(2), 43),
        Err(ExecutorError::ReceiptMissing)
    ));
    adapter.execute(&second, ticket(2), 43)?;
    let third = catalog.issue_register_grant(&artifact, ticket(3), scope())?;
    assert!(matches!(
        adapter.execute(&third, ticket(3), 44),
        Err(ExecutorError::Capacity)
    ));
    assert_eq!(adapter.reconcile(&grant, ticket(1), 42)?, first);
    assert_eq!(adapter.effect_count()?, 2);
    assert_eq!(adapter.value(scope())?, Some(43));
    Ok(())
}

#[test]
fn recovery_axis_defaults_blocked_and_unsupported_classes_reject() -> TestResult {
    assert_eq!(RecoveryClass::default(), RecoveryClass::Blocked);
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract()?;
    for recovery in [
        RecoveryClass::NoRecoveryNeeded,
        RecoveryClass::RetrySameKey,
        RecoveryClass::Compensate,
        RecoveryClass::ManualGate,
    ] {
        assert!(matches!(
            catalog.register_pack(&contract, scope(), recovery),
            Err(ExecutorError::PackMismatch)
        ));
    }
    let blocked = catalog.register_pack(&contract, scope(), RecoveryClass::default())?;
    assert_eq!(blocked.recovery, RecoveryClass::Blocked);
    assert_eq!(
        blocked.idempotency,
        vb_core::action_pack::IdempotencyClass::KeyEnforcedByExecutor
    );
    Ok(())
}

#[test]
fn stored_pack_reopens_with_fresh_executed_catalog_and_exact_classification() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract()?;
    let workflow = action_workflow()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let dir = tempfile::tempdir()?;
    let digest = {
        let journal = vb_storage::FjallJournal::open(dir.path(), None)?;
        catalog
            .submit_artifact(
                &journal,
                &workflow,
                RuntimePolicy::Strict,
                &[contract],
                &[pack.clone()],
            )?
            .digest
    };
    let reopened = vb_storage::FjallJournal::open(dir.path(), None)?;
    let record = reopened.compiled_ir(digest)?.ok_or("artifact absent")?;
    let restored = postcard::from_bytes(&record.ir)?;
    let fresh = ExecutorCatalog::run_registered_conformance()?;
    assert_eq!(
        fresh.verify_artifact(&restored, RuntimePolicy::Strict)?,
        workflow
    );
    let vb_core::artifact::ArtifactEvidence::Packed(packed) = restored.verification.evidence else {
        return Err("missing packs".into());
    };
    assert_eq!(packed.packs.as_ref(), &[pack]);
    Ok(())
}
