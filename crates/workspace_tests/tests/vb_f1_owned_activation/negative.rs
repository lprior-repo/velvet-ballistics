use super::support::{Case, TestResult, assert_pristine, config, runtime};
use vb_core::{RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::shard::ShardCommand;

#[test]
fn wrong_source_run_and_header_are_rejected_without_publication() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let runtime = case.runtime(journal.clone(), 10)?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, WorkflowDigest::from_bytes([7; 32])),
        Err(RuntimeError::RunNotFound)
    ));
    assert!(matches!(
        runtime.activate_owned_submission(RunId::new(18), case.source.digest),
        Err(RuntimeError::RunNotFound)
    ));
    assert_pristine(&case, &journal)?;
    let mut header = super::support::header(case.run, &case.workflow);
    header.compiled_digest = WorkflowDigest::from_bytes([8; 32]);
    journal.put_run_header(&header)?;
    assert!(
        matches!(runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::StorageJournalAppend { source })
        if matches!(*source, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::IdentityMismatch)))
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert!(matches!(
        runtime.inspect_run_state(case.run),
        Err(RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn duplicate_queued_activation_cannot_replace_live_actor_state() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let mut runtime = case.runtime(journal.clone(), 10)?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert!(runtime.tick_all()?);
    let before = runtime.inspect_run_state(case.run)?.clone();
    let events = journal.events_for_run_full(case.run)?;
    assert!(matches!(
        runtime.tick_all(),
        Err(RuntimeError::RunAlreadyExists)
    ));
    assert_eq!(runtime.inspect_run_state(case.run)?, &before);
    assert_eq!(journal.events_for_run_full(case.run)?, events);
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::RunAlreadyExists)
    ));
    Ok(())
}

#[test]
fn source_advancement_between_enqueue_and_shard_processing_is_revalidated() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let mut runtime = case.runtime(journal.clone(), 10)?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    journal.append_strict(&vb_storage::JournalEvent::StepStarted {
        run: case.run,
        seq: vb_storage::EventSeq::new(2),
        step: case.workflow.entry(),
        attempt: 1,
    })?;
    let before = journal.events_for_run_full(case.run)?;
    assert!(
        matches!(runtime.tick_all(), Err(RuntimeError::StorageJournalAppend { source })
        if matches!(*source, vb_storage::JournalError::TooManyEvents { .. }))
    );
    assert_eq!(journal.events_for_run_full(case.run)?, before);
    assert!(matches!(
        runtime.inspect_run_state(case.run),
        Err(RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn queue_full_leaves_pristine_submission_retryable() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let adapter = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = runtime(adapter, config(12, 1, 8))?;
    runtime.inspect_run(RunId::new(999), 1)?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::QueueFull)
    ));
    assert_pristine(&case, &journal)?;
    assert!(runtime.tick_all()?);
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert!(runtime.tick_all()?);
    super::evidence::terminal_output(&case, &journal, vb_core::SlotValue::List(case.root))?;
    Ok(())
}

#[test]
fn zero_active_capacity_refuses_without_sequence_or_frame_publication() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let adapter = StorageRuntimeJournal::shared_strict(journal.clone());
    let runtime = runtime(adapter, config(10, 8, 0))?;
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 0 })
    ));
    assert_pristine(&case, &journal)?;
    assert_eq!(runtime.pending_boundary_snapshot(8).active_run_count(), 0);
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    Ok(())
}

#[test]
fn forged_public_command_must_revalidate_the_same_owned_authority() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let adapter = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut shard = vb_runtime::shard::Shard::new_with_journal(config(10, 8, 8), adapter);
    shard.enqueue(ShardCommand::ActivateOwnedSubmission {
        run: case.run,
        expected_source_digest: WorkflowDigest::from_bytes([7; 32]),
    })?;
    assert!(matches!(shard.tick(), Err(RuntimeError::RunNotFound)));
    assert_eq!(shard.active_run_count(), 0);
    assert_pristine(&case, &journal)?;
    Ok(())
}

#[test]
fn actual_full_capacity_is_rechecked_at_processing_then_submission_can_retry() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let second = RunId::new(18);
    super::support::additional_run(&case, &journal, second)?;
    let adapter = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = runtime(adapter, config(10, 8, 1))?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    runtime.activate_owned_submission(second, case.source.digest)?;
    assert!(runtime.tick_all()?);
    let pending = journal.events_for_run_full(second)?;
    assert_eq!(pending.len(), 2);
    assert!(matches!(
        runtime.tick_all(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    ));
    assert_eq!(journal.events_for_run_full(second)?, pending);
    assert!(matches!(
        runtime.inspect_run_state(second),
        Err(RuntimeError::RunNotFound)
    ));
    runtime.kill_run(case.run)?;
    assert!(runtime.tick_all()?);
    runtime.activate_owned_submission(second, case.source.digest)?;
    assert!(runtime.tick_all()?);
    let state = runtime.inspect_run_state(second)?;
    assert_eq!(state.frame.run_id(), second);
    assert_eq!(
        state.frame.read_slot(case.input)?,
        &vb_core::SlotValue::List(case.root)
    );
    assert_eq!(state.store.list(case.root)?, super::arenas::VALUES);
    Ok(())
}
