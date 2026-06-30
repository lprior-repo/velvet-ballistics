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

#[test]
fn shutdown_forces_syncall_for_journaled_events() -> Result<(), String> {
    // Given a QueuedStorageRuntimeJournal over a temporary Fjall directory,
    // enqueue several journaled events but do NOT call `close` or
    // `persist_strict` explicitly. The runtime shutdown boundary must invoke
    // the durability barrier itself, so when the journal is dropped and a
    // fresh FjallJournal is opened on the same path (the simulated restart),
    // every drained event must be replayable. This pins Master §49
    // Crash-Consistency Rule for the journaled profile path.
    let (dir, journal) = temp_journal()?;
    let journal_path = dir.path().to_path_buf();
    let queue = journal_queue(8, 4)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = RunId::new(2_001);

    for i in 0u16..5u16 {
        assert_eq!(
            adapter.append_sequenced(
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(i),
                },
                EventSeq::new(u64::from(i)),
            ),
            Ok(())
        );
    }

    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 5 && counts.strict == 0
    ));

    let report = adapter
        .drain_for_shutdown()
        .map_err(|error| error.to_string())
        .expect("drain_for_shutdown must succeed");
    assert_eq!(report.drained, 5);
    assert_eq!(report.written, 5);
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    // Drop the in-memory journal references (simulating process shutdown) but
    // keep the directory alive on disk for the simulated restart.
    drop(adapter);
    drop(queue);
    drop(journal);

    // Simulated restart: open a brand-new FjallJournal on the same path
    // without invoking `persist_strict` ourselves. If the shutdown boundary
    // did not force a durability barrier, the WAL fsync would be missing and
    // the events could be unrecoverable after a real power-loss. The reopen
    // exercises the durability end-to-end regardless.
    let reopened = FjallJournal::open(&journal_path, None)
        .map_err(|error| error.to_string())
        .map(Arc::new)
        .expect("reopened journal must open on the same path");
    let events = reopened
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay on reopened journal must succeed");
    assert_eq!(
        events.len(),
        5,
        "all 5 enqueued journaled events must survive the simulated restart after drain_for_shutdown"
    );
    for (i, event) in events.iter().enumerate() {
        let i_u16 = u16::try_from(i).map_err(|error| error.to_string())?;
        let i_u64 = u64::try_from(i).map_err(|error| error.to_string())?;
        assert!(
            matches!(
                event,
                JournalEvent::StepStarted { run: r, seq, step, .. }
                    if *r == run && *seq == EventSeq::new(i_u64) && *step == StepIdx::new(i_u16)
            ),
            "event {i} must replay with the exact run/seq/step that was enqueued; got {event:?}"
        );
    }
    Ok(())
}

#[test]
fn drain_for_shutdown_empties_pending_writes_and_makes_them_durable() -> Result<(), String> {
    // Sibling to the pre-existing `queued_journal_drain_for_shutdown_empties_all_pending_writes`
    // test. The earlier test only asserts queue emptiness; this one adds the
    // durability half of the contract: after the shutdown boundary returns Ok,
    // every drained event must be replayable on a freshly-opened journal.
    // Together the two tests pin the full shutdown contract: queue empty
    // AND events durable.
    let (dir, journal) = temp_journal()?;
    let journal_path = dir.path().to_path_buf();
    let queue = journal_queue(8, 2)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = RunId::new(2_002);

    for i in 0u16..4u16 {
        assert_eq!(
            adapter.append_sequenced(
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(i),
                },
                EventSeq::new(u64::from(i)),
            ),
            Ok(())
        );
    }
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 4 && counts.strict == 0
    ));

    let report = adapter
        .drain_for_shutdown()
        .map_err(|error| error.to_string())
        .expect("drain_for_shutdown must succeed");
    assert_eq!(report.drained, 4);
    assert_eq!(report.written, 4);

    // Queue is empty (the original contract).
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    // And the drained events are durable on disk: a fresh journal opened on
    // the same path sees them.
    drop(adapter);
    drop(queue);
    drop(journal);
    let reopened = FjallJournal::open(&journal_path, None)
        .map_err(|error| error.to_string())
        .map(Arc::new)
        .expect("reopened journal must open on the same path");
    let events = reopened
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay on reopened journal must succeed");
    assert_eq!(events.len(), 4);
    Ok(())
}
