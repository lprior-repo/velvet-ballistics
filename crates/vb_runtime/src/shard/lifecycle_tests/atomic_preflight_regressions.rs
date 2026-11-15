#[test]
fn single_append_preflights_overflow_and_capacity_before_adapter() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionSingletonJournal::default());
    let mut config = small_config();
    config.max_active_runs = 1;
    let mut shard = Shard::new_with_journal(config, journal.clone()).expect("valid test configuration");
    let run = RunId::new(91_001);
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))
        .map_err(|e| e.to_string())?;
    let event = |run| RuntimeJournalEvent::RunCancelled {
        run,
        reason: None,
        attempt: 1,
    };
    let Err(error) = shard.append_journal_event(event(run)) else {
        return Err("overflow must refuse".into());
    };
    assert!(
        matches!(error, RuntimeError::StorageJournalAppend { source }
        if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow))
    );
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(shard.journal_sequence_for(run).get(), u64::MAX);
    assert_eq!(
        shard.append_journal_event(event(RunId::new(91_002))),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(shard.journal_seq_get(RunId::new(91_002)), None);
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(7))
        .map_err(|e| e.to_string())?;
    shard
        .append_journal_event(event(run))
        .map_err(|e| e.to_string())?;
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(shard.journal_sequence_for(run).get(), 8);
    Ok(())
}

#[test]
fn pre_persisted_refused_current_admission_releases_capacity_one_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let workflow = honest_digest_workflow(two_slot_workflow().ok_or("workflow")?)?;
    let artifact =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)?;
    assert!(artifact.accepted_at_seq.get() > 0);
    let run = RunId::new(91_003);
    let mut batch = journal.batch();
    batch.put_run_header(&vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    })?;
    batch.append_event(&vb_storage::JournalEvent::RunAccepted {
        run,
        seq: vb_storage::EventSeq::ZERO,
        workflow: workflow.digest(),
    })?;
    batch.commit()?;
    let prefix = journal.events_for_run(run)?;
    let queue = std::sync::Arc::new(vb_storage::JournalWriterQueue::new(
        64,
        64,
        vb_storage::StorageLimits::DEFAULT,
    )?);
    let adapter = std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let mut config = small_config();
    config.max_active_runs = 1;
    config.policy = vb_core::RuntimePolicy::Journaled;
    let mut shard = Shard::new_with_journal_and_artifact_store(config, adapter.clone(), crate::admission::StorageArtifactStore::shared(journal.clone())).expect("valid test configuration");
    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(1))?;
    let frame = shard.take_frame_for(run, &workflow)?;
    shard.release_frame(frame);
    assert_eq!(shard.frame_pool_metrics(), (1, 1));
    let command = || ShardCommand::SubmitPrePersisted {
        run,
        workflow: workflow.clone(),
        caps: CapabilitySet::empty(),
    };
    shard.enqueue(command())?;
    (0..64).try_for_each(|offset| {
        queue.enqueue_journaled(vb_storage::JournalEvent::RunCancelled {
            run: RunId::new(92_000 + offset),
            seq: vb_storage::EventSeq::ZERO,
            reason: None,
            attempt: 1,
        })
    })?;
    assert!(
        matches!(shard.tick(), Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
        if matches!(source.as_ref(), vb_storage::JournalError::QueueFull))
    );
    assert_eq!(journal.events_for_run(run)?, prefix);
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    assert_eq!(shard.frame_pool_metrics(), (1, 1));
    assert_eq!(shard.journal_seq_get(run), None);
    assert_eq!(shard.pending_action_get(run), None);
    adapter.flush_batch()?;
    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(1))?;
    shard.enqueue(command())?;
    assert_eq!(shard.tick(), Ok(true));
    adapter.flush_batch()?;
    let events = journal.events_for_run(run)?;
    let expected_admission = vb_storage::JournalEvent::RunAdmissionBudget {
        run,
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: artifact.digest,
        input_digest: vb_storage::RunInputBinding::Digest(vb_storage::inputs::compute_input_digest(&[])?),
        policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest),
        granted_capabilities: CapabilitySet::empty(),
        policy: vb_core::RuntimePolicy::Journaled,
        budget: vb_core::AggregateResourceBudget::from_workflow(&workflow)?,
    };
    let admissions: Vec<_> = events.iter().filter(|event| matches!(event, vb_storage::JournalEvent::RunAdmissionBudget { .. })).collect();
    assert_eq!(admissions, vec![&expected_admission]);
    assert!(events.iter().any(|event| matches!(event, vb_storage::JournalEvent::RunBudgetOriginEvent { started_millis, .. } if *started_millis > 0)));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn single_append_adapter_refusal_preserves_sequence_and_retry_advances_once() -> Result<(), String>
{
    struct RefusingJournal {
        calls: std::sync::atomic::AtomicUsize,
        refuse: std::sync::atomic::AtomicBool,
    }
    impl crate::journal::RuntimeJournal for RefusingJournal {
        fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.refuse.load(std::sync::atomic::Ordering::SeqCst) {
                Err(RuntimeError::from(vb_storage::JournalError::QueueFull))
            } else {
                Ok(())
            }
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }
    let journal = std::sync::Arc::new(RefusingJournal {
        calls: 0.into(),
        refuse: true.into(),
    });
    let mut shard = Shard::new_with_journal(small_config(), journal.clone()).expect("valid test configuration");
    let run = RunId::new(91_004);
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(7))
        .map_err(|e| e.to_string())?;
    let event = || RuntimeJournalEvent::RunCancelled {
        run,
        reason: None,
        attempt: 1,
    };
    assert!(
        matches!(shard.append_journal_event(event()), Err(RuntimeError::StorageJournalAppend { source })
        if matches!(source.as_ref(), vb_storage::JournalError::QueueFull))
    );
    assert_eq!(shard.journal_sequence_for(run).get(), 7);
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    journal
        .refuse
        .store(false, std::sync::atomic::Ordering::SeqCst);
    shard
        .append_journal_event(event())
        .map_err(|e| e.to_string())?;
    assert_eq!(shard.journal_sequence_for(run).get(), 8);
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn single_append_new_run_capacity_refuses_before_adapter() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionSingletonJournal::default());
    let mut config = small_config();
    config.max_active_runs = 1;
    let mut shard = Shard::new_with_journal(config, journal.clone()).expect("valid test configuration");
    shard
        .seed_journal_sequence(RunId::new(91_006), vb_storage::EventSeq::new(7))
        .map_err(|e| e.to_string())?;
    let new_run = RunId::new(91_007);
    let result = shard.append_journal_event(RuntimeJournalEvent::RunCancelled {
        run: new_run,
        reason: None,
        attempt: 1,
    });
    assert_eq!(
        result,
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(shard.journal_seq_get(new_run), None);
    Ok(())
}
