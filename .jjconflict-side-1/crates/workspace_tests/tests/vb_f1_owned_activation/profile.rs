use super::support::{Case, TestResult, assert_pristine, config, runtime};
use std::sync::Arc;
use vb_core::RuntimePolicy;
use vb_runtime::RuntimeError;
use vb_runtime::journal::{
    QueuedStorageRuntimeJournal, StorageRuntimeJournal, VolatileRuntimeJournal,
};

#[test]
fn runtime_strict_policy_does_not_upgrade_a_journaled_adapter() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    assert_eq!(
        adapter.durability_profile(),
        Some(vb_storage::DurabilityProfile::Journaled)
    );
    let runtime = runtime(adapter, config(10, 8, 8))?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_strict_journal"
        })
    ));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert_pristine(&case, &journal)?;
    Ok(())
}

#[test]
fn queued_strict_profile_still_refuses_async_strict_ack() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let queue = Arc::new(vb_storage::JournalWriterQueue::new(
        8,
        4,
        vb_storage::StorageLimits::DEFAULT,
    )?);
    let adapter = QueuedStorageRuntimeJournal::shared_strict(journal.clone(), queue);
    assert_eq!(
        adapter.durability_profile(),
        Some(vb_storage::DurabilityProfile::Strict)
    );
    let runtime = runtime(adapter, config(10, 8, 8))?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::UnsupportedAsyncStrictAck)
    ));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert_pristine(&case, &journal)?;
    Ok(())
}

#[test]
fn unknown_default_profile_has_no_strict_activation_authority() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let adapter = VolatileRuntimeJournal::shared();
    assert_eq!(adapter.durability_profile(), None);
    let runtime = runtime(adapter, config(10, 8, 8))?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_strict_journal"
        })
    ));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    Ok(())
}

#[test]
fn runtime_and_durable_admission_policies_must_both_be_strict() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Journaled)?;
    let journal = case.open()?;
    assert_pristine(&case, &journal)?;
    let runtime = case.runtime(journal.clone(), 10)?;
    let activation = runtime.activate_owned_submission(case.run, case.source.digest);
    assert!(
        matches!(
            &activation,
            Err(RuntimeError::AdmissionArtifactInvalid { .. })
        ),
        "actual activation refusal: {activation:?}"
    );
    assert_pristine(&case, &journal)?;
    let strict = Case::new(false, RuntimePolicy::Strict)?;
    let journal = strict.open()?;
    let mut settings = config(10, 8, 8);
    settings.policy = RuntimePolicy::Relaxed;
    let runtime = super::support::runtime(
        StorageRuntimeJournal::shared_strict(journal.clone()),
        settings,
    )?;
    assert!(matches!(
        runtime.activate_owned_submission(strict.run, strict.source.digest),
        Err(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_strict_only"
        })
    ));
    assert_pristine(&strict, &journal)?;
    Ok(())
}
