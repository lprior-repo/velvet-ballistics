fn step_succeeded_carries_retry_attempt_number() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(12);

    // Submit and drive to first action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Fail ticket 1 (retryable)
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

    // Fail ticket 2 (retryable)
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Complete ticket 3 (matching attempt=3)
    let ticket3 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 3")?;
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ticket3,
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Check attempt=3 was persisted in the durable action completion envelope.
    let events = journal.snapshot()?;
    let completion_with_attempt_3: bool = events.iter().any(|e| {
        matches!(
            e,
            RuntimeJournalEvent::ActionCompletedEnvelope { ticket, .. }
                if ticket.run == run && ticket.step == StepIdx::new(1) && ticket.attempt == 3
        )
    });
    assert!(
        completion_with_attempt_3,
        "ActionCompletedEnvelope ticket should carry attempt=3"
    );

}