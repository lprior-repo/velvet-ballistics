fn review_low_wait_workflow() -> Result<vb_core::CompiledWorkflow, Box<dyn std::error::Error>> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock")?;
    let mut parts = owned_waiting_workflow(now.checked_add(60_000).ok_or("deadline")?)?.to_parts();
    parts.resource_contract.max_queue_depth = 1;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
}

#[test]
fn review_raw_recovery_cannot_bypass_active_queue_cap() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = review_low_wait_workflow()?;
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
    assert_eq!(journal.snapshot()?, before);
    Ok(())
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
    let workflow = review_low_wait_workflow()?;
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock")?;
    let run = RunId::new(962);
    let source_bytes = b"review owned queue cap".to_vec();
    let source = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let store =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::default(),
        store.checkpoint_v1(),
        &workflow,
    )?;
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
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let before = journal.events_for_run_full(run)?;
    assert!(before.iter().any(|event| matches!(event, vb_storage::JournalEvent::RunAdmissionBudget { budget, .. } if budget.max_queue_depth == 1)));
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
    assert_eq!(journal.events_for_run_full(run)?, before);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
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
    assert_eq!(journal.events_for_run_full(run)?, before);
    Ok(())
}
