fn stale_attempt_completion_rejected_before_journal_write() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(20);

    // Submit and drive to first action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Drive failure to advance to ticket 2
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Capture real ticket 2 (will become stale)
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    let ticket2_attempt = ticket2.attempt;
    assert_eq!(ticket2_attempt, 2, "ticket2 should have attempt=2");

    // Drive failure to advance to ticket 3
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Now current attempt is 3, but we'll try to complete with the saved ticket 2 attempt
    let ticket3 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 3")?;
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");

    // Capture state before stale completion attempt
    let journal_before = journal.snapshot()?;
    let counters_before = shard.counters().snapshot();

    // Attempt stale completion with saved ticket 2 attempt number
    let stale_ticket = ticket2;
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: stale_ticket,
            output,
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: ticket2_attempt,
            current: ticket3.attempt,
        })
    );

    // Verify NO new journal events were written
    let journal_after = journal.snapshot()?;
    assert_eq!(
        journal_before, journal_after,
        "Journal must not be mutated when stale attempt is rejected"
    );

    // Verify counters unchanged
    let counters_after = shard.counters().snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "runs_completed counter must not change on stale rejection"
    );
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "runs_failed counter must not change on stale rejection"
    );

}