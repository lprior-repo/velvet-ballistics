#[test]
fn review_partial_shutdown_keeps_failed_later_batch_and_retries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(3, 1, StorageLimits::DEFAULT)?;
    journal.append_journaled(&event(1))?;
    for seq in 0..3 { queue.enqueue_journaled(event(seq))?; }
    journal.fail_next_persist_for_test();
    assert!(matches!(queue.shutdown_durable(&journal), Err(JournalError::StrictDurabilityFailed)));
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert!(matches!(queue.enqueue_journaled(event(3)), Err(JournalError::QueueShutdown)));
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![event(0),event(1)]);
    let retry = queue.shutdown_durable(&journal)?;
    assert_eq!((retry.drained,retry.written),(2,2));
    assert_eq!(queue.pending_profile_counts()?.journaled,0);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![event(0),event(1),event(2)]);
    assert_eq!(queue.shutdown_durable(&journal)?.drained,0);
    drop(journal);
    let reopened=FjallJournal::open(dir.path(),None)?;
    assert_eq!(reopened.events_for_run(RunId::new(900))?,vec![event(0),event(1),event(2)]);
    Ok(())
}
