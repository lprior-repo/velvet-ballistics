#[test]
fn queued_storage_runtime_journal_drain_all_flushes_past_batch_size() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(8, 2), "journal queue opens") else {
        return;
    };
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
}

#[test]
fn queued_storage_runtime_journal_rejects_unsequenced_append() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(4, 2), "journal queue opens") else {
        return;
    };
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
}

#[test]
fn runtime_shutdown_graceful_drains_owned_queued_journal() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(4, 1), "journal queue opens") else {
        return;
    };
    let runtime_journal = Arc::new(QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let run = RunId::new(49);
    let workflow = WorkflowDigest::from_bytes([12; 32]);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        assert!(false, "invalid shard count");
        return;
    };
    let runtime = Runtime::new_with_journal(shard_count, ShardConfig::default(), runtime_journal);

    let Some(compiled) = require_ok(single_finish_workflow(workflow), "workflow compiles") else {
        return;
    };
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
}

#[test]
fn queued_storage_runtime_journal_maps_queue_full_to_runtime_error() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(1, 1), "journal queue opens") else {
        return;
    };
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

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "queue-full events read",
    ) else {
        return;
    };
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
}
