#[test]
fn queued_storage_runtime_journal_drain_all_flushes_past_batch_size() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(8, 2)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = RunId::new(48);
    let workflow = WorkflowDigest::from_bytes([11; 32]);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunCancelled { run, reason: None },
            EventSeq::new(1),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(RuntimeJournalEvent::RunFailed { run }, EventSeq::new(2),),
        Ok(())
    );
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 3 && counts.strict == 0
    ));

    assert!(matches!(
        adapter.drain_all(),
        Ok(report) if report.drained == 3 && report.written == 3
    ));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 3));
    Ok(())
}

#[test]
fn queued_storage_runtime_journal_rejects_unsequenced_append() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(4, 2)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = RunId::new(50);

    assert!(matches!(
        adapter.append(RuntimeJournalEvent::RunCancelled { run, reason: None }),
        Err(crate::RuntimeError::UnsupportedOperation {
            operation: "unsequenced_storage_journal_append"
        })
    ));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    Ok(())
}

#[test]
fn runtime_shutdown_graceful_drains_owned_queued_journal() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(4, 1)?;
    let runtime_journal = Arc::new(QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let run = RunId::new(49);
    let workflow = WorkflowDigest::from_bytes([12; 32]);
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| "invalid shard count".to_owned())?;
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    let runtime = Runtime::new(shard_count, config, runtime_journal);

    let compiled = single_finish_workflow(workflow)?;
    assert_eq!(runtime.submit_direct(run, compiled), Ok(()));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    let mut runtime = runtime;
    assert_eq!(runtime.tick_all(), Ok(true));
    // The run header is already drained before acknowledgement; shutdown drains
    // only post-admission execution evidence for this single Finish step.
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(ref c) if c.journaled >= 2 && c.strict == 0
    ));
    assert_eq!(runtime.shutdown_graceful(), Ok(()));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    // At minimum RunSubmitted + RunAdmission + StepSucceeded + RunFinished are stored.
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() >= 4));
    Ok(())
}

#[test]
fn queued_storage_runtime_journal_maps_queue_full_to_runtime_error() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(1, 1)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(46);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunCancelled { run, reason: None },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert!(matches!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunFailed { run },
            EventSeq::new(1),
        ),
        Err(crate::RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );
    assert_eq!(
        adapter.append_sequenced(RuntimeJournalEvent::RunFailed { run }, EventSeq::new(1),),
        Ok(())
    );
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        events,
        vec![
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(0),
                attempt: 1,
                reason: None,
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
            },
        ]
    );
    Ok(())
}

#[test]
fn storage_runtime_journal_probe_delegates_to_fjall_health() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let adapter = StorageRuntimeJournal::journaled(journal);

    assert_eq!(adapter.probe(), Ok(()));
    Ok(())
}

#[test]
fn queued_storage_runtime_journal_probe_rejects_full_queue() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(1, 1)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal, queue);
    let run = RunId::new(47);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunCancelled { run, reason: None },
            EventSeq::new(0),
        ),
        Ok(())
    );

    assert!(matches!(
        adapter.probe(),
        Err(crate::RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    Ok(())
}

#[test]
fn volatile_runtime_journal_accepts_appends_until_configured_capacity() -> Result<(), String> {
    let capacity = NonZeroUsize::new(2).ok_or_else(|| "invalid journal capacity".to_owned())?;
    let journal = VolatileRuntimeJournal::with_capacity(capacity);
    let run = RunId::new(51);
    let workflow = WorkflowDigest::from_bytes([13; 32]);
    let first = RuntimeJournalEvent::RunSubmitted { run, workflow };
    let second = RuntimeJournalEvent::RunFinished {
        run,
        result: SlotIdx::new(0),
    };

    assert_eq!(journal.append(first.clone()), Ok(()));
    assert_eq!(journal.append(second.clone()), Ok(()));

    assert_eq!(journal.snapshot(), Ok(vec![first, second]));
    Ok(())
}

#[test]
fn volatile_runtime_journal_returns_journal_full_and_preserves_entries_when_capacity_is_reached()
-> Result<(), String> {
    let capacity = NonZeroUsize::new(1).ok_or_else(|| "invalid journal capacity".to_owned())?;
    let journal = VolatileRuntimeJournal::with_capacity(capacity);
    let run = RunId::new(52);
    let workflow = WorkflowDigest::from_bytes([14; 32]);
    let kept = RuntimeJournalEvent::RunSubmitted { run, workflow };
    let rejected = RuntimeJournalEvent::RunFailed { run };

    assert_eq!(journal.append(kept.clone()), Ok(()));
    assert_eq!(
        journal.append(rejected),
        Err(crate::RuntimeError::JournalFull { capacity: 1 })
    );

    assert_eq!(journal.snapshot(), Ok(vec![kept]));
    Ok(())
}

#[test]
fn volatile_runtime_journal_snapshots_remain_stable_after_full_append_rejection()
-> Result<(), String> {
    let capacity = NonZeroUsize::new(2).ok_or_else(|| "invalid journal capacity".to_owned())?;
    let journal = VolatileRuntimeJournal::with_capacity(capacity);
    let run = RunId::new(53);
    let first = RuntimeJournalEvent::RunCancelled { run, reason: None };
    let second = RuntimeJournalEvent::RunFailed { run };
    let rejected = RuntimeJournalEvent::WaitScheduled {
        run,
        step: StepIdx::new(1),
    };

    assert_eq!(journal.append(first.clone()), Ok(()));
    assert_eq!(journal.append(second.clone()), Ok(()));
    assert_eq!(
        journal.append(rejected),
        Err(crate::RuntimeError::JournalFull { capacity: 2 })
    );

    let expected = Ok(vec![first, second]);
    assert_eq!(journal.snapshot(), expected.clone());
    assert_eq!(journal.snapshot(), expected);
    Ok(())
}
