fn review_low_wait_workflow(
    queue_depth: u32,
) -> Result<vb_core::CompiledWorkflow, Box<dyn std::error::Error>> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock")?;
    let mut parts = owned_waiting_workflow(now.checked_add(60_000).ok_or("deadline")?)?.to_parts();
    parts.resource_contract.max_queue_depth = queue_depth;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
}

#[test]
fn review_raw_recovery_cannot_bypass_active_queue_cap() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = review_low_wait_workflow(1)?;
    let run = RunId::new(961);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = crate::shard::Shard::new_with_journal(runtime_config(), journal.clone());
    shard.enqueue(crate::shard::ShardCommand::Submit {
        run,
        workflow: workflow.clone(),
        caps: CapabilitySet::empty(),
    })?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.command_queue_pending_for(run), 0);
    let state = shard.run_state_get(run).ok_or("active run")?;
    let admission = state.admission.as_ref().ok_or("admission")?;
    assert_eq!(admission.budget().ok_or("budget")?.max_queue_depth, 1);
    let command = crate::shard::types::RecoverRunCommand::new(
        run,
        state.frame.clone(),
        admission.artifact_digest(),
        workflow.digest(),
        vb_storage::EventSeq::new(0),
        state.collect_states.clone(),
        crate::recovery::RecoveredRunBoundary::none(),
    );
    let before = journal.snapshot()?;
    shard.enqueue(crate::shard::ShardCommand::Recover {
        command: Box::new(command.clone()),
    })?;
    assert_eq!(
        shard.enqueue(crate::shard::ShardCommand::Recover {
            command: Box::new(command)
        }),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        })
    );
    assert_eq!(shard.command_queue_pending_for(run), 1);
    assert_eq!(shard.command_queue_len(), 1);
    assert_eq!(journal.snapshot()?, before);
    Ok(())
}

fn review_owned_cap_journal() -> Result<
    (tempfile::TempDir, Arc<vb_storage::FjallJournal>, RunId, WorkflowDigest),
    Box<dyn std::error::Error>,
> {
    review_owned_cap_journal_with_depth(1)
}

fn review_owned_cap_journal_with_depth(
    queue_depth: u32,
) -> Result<
    (tempfile::TempDir, Arc<vb_storage::FjallJournal>, RunId, WorkflowDigest),
    Box<dyn std::error::Error>,
> {
    let (run, source, submission) = review_owned_submission(queue_depth)?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let events = journal.events_for_run_full(run)?;
    assert!(events.iter().any(|event| matches!(event,
        vb_storage::JournalEvent::RunAdmissionBudget { budget, .. }
            if budget.max_queue_depth == queue_depth)));
    Ok((directory, journal, run, source))
}

fn review_owned_submission(
    queue_depth: u32,
) -> Result<
    (RunId, WorkflowDigest, vb_storage::RunSubmission),
    Box<dyn std::error::Error>,
> {
    let workflow = review_low_wait_workflow(queue_depth)?;
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock")?;
    let run = RunId::new(962);
    let source_bytes = b"review owned queue cap".to_vec();
    let source = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let bundle = review_owned_bundle(&workflow)?;
    let inputs =
        vb_storage::OwnedRunInputsRecordV1::new(run, artifact.digest, source, &bundle, &workflow)?;
    let submission = vb_storage::RunSubmission::owned(
        vb_storage::WorkflowSourceRecord {
            digest: source,
            source: source_bytes,
        },
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: now,
        },
    )?;
    Ok((run, source, submission))
}

fn review_owned_bundle(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::root_bundle::OwnedRootBundle, Box<dyn std::error::Error>> {
    let store =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    Ok(vb_core::root_bundle::OwnedRootBundle::new(
        Box::default(),
        store.checkpoint_v1(),
        workflow,
    )?)
}

fn review_owned_cap_runtime() -> Result<
    (
        tempfile::TempDir,
        Arc<vb_storage::FjallJournal>,
        Runtime,
        RunId,
        WorkflowDigest,
    ),
    Box<dyn std::error::Error>,
> {
    let (directory, journal, run, source) = review_owned_cap_journal()?;
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    Ok((directory, journal, runtime, run, source))
}

#[test]
fn review_pristine_owned_activation_obeys_retained_queue_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, journal, runtime, run, source) = review_owned_cap_runtime()?;
    let before = journal.events_for_run_full(run)?;
    runtime.activate_owned_submission(run, source)?;
    assert_eq!(
        runtime.activate_owned_submission(run, source),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        })
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(journal.events_for_run_full(run)?, before);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert!(matches!(runtime.inspect_run_state(run), Err(RuntimeError::RunNotFound)));
    Ok(())
}

#[test]
fn review_pending_recovery_cap_governs_following_cancel() -> Result<(), Box<dyn std::error::Error>>
{
    let (_directory, journal, runtime, run, _source) = review_owned_cap_runtime()?;
    let before = journal.events_for_run_full(run)?;
    runtime.recover_and_resume(run)?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    assert_eq!(
        runtime.cancel_run(run),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1,
        })
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(journal.events_for_run_full(run)?, before);
    Ok(())
}

#[test]
fn review_raw_owned_activation_uses_verified_cap_before_queue_push()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, journal, run, source) = review_owned_cap_journal()?;
    let adapter = crate::journal::StorageRuntimeJournal::shared_strict(journal.clone());
    let shard = crate::shard::Shard::new_with_journal(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..runtime_config()
        },
        adapter,
    );
    let before = journal.events_for_run_full(run)?;
    let command = || crate::shard::ShardCommand::ActivateOwnedSubmission {
        run,
        expected_source_digest: source,
    };
    shard.enqueue(command())?;
    assert_eq!(
        shard.enqueue(command()),
        Err(RuntimeError::PerRunQueueDepthExceeded { run, pending: 1, limit: 1 })
    );
    assert_eq!(shard.command_queue_pending_for(run), 1);
    assert_eq!(shard.command_queue_len(), 1);
    assert_eq!(journal.events_for_run_full(run)?, before);
    Ok(())
}

#[test]
fn review_raw_owned_activation_rejects_unknown_source_before_queue_push()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, journal, run, _source) = review_owned_cap_journal()?;
    let adapter = crate::journal::StorageRuntimeJournal::shared_strict(journal.clone());
    let shard = crate::shard::Shard::new_with_journal(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..runtime_config()
        },
        adapter,
    );
    let before = journal.events_for_run_full(run)?;
    assert_eq!(
        shard.enqueue(crate::shard::ShardCommand::ActivateOwnedSubmission {
            run,
            expected_source_digest: WorkflowDigest::from_bytes([7; 32]),
        }),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(shard.command_queue_pending_for(run), 0);
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(journal.events_for_run_full(run)?, before);
    Ok(())
}

#[test]
fn review_raw_owned_activation_keeps_nondefault_cap_for_generic_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, journal, run, source) = review_owned_cap_journal_with_depth(2)?;
    let adapter = crate::journal::StorageRuntimeJournal::shared_strict(journal.clone());
    let shard = crate::shard::Shard::new_with_journal(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..runtime_config()
        },
        adapter,
    );
    let activation = crate::shard::ShardCommand::ActivateOwnedSubmission {
        run,
        expected_source_digest: source,
    };
    let before = journal.events_for_run_full(run)?;
    shard.enqueue(activation)?;
    shard.enqueue(crate::shard::ShardCommand::Cancel { run, reason: None })?;
    assert_eq!(
        shard.enqueue(crate::shard::ShardCommand::Cancel { run, reason: None }),
        Err(RuntimeError::PerRunQueueDepthExceeded { run, pending: 2, limit: 2 })
    );
    assert_eq!(shard.command_queue_pending_for(run), 2);
    assert_eq!(shard.command_queue_len(), 2);
    assert_eq!(journal.events_for_run_full(run)?, before);
    Ok(())
}
