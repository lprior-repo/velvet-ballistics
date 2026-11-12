fn action_attempts_monotonically_advances_on_retry() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(31);

    // Submit and drive to action (step 1)
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should be 1 after first scheduling
    {
        let state = shard.run_state_get_mut(run).ok_or("run should exist")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(1),
            "action_attempts[1] should be 1 after first scheduling"
        );
    }

    // Fail with retryable policy - this should trigger a retry
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should now be 2 (retry scheduled)
    {
        let state = shard
            .run_state_get_mut(run)
            .ok_or("run should exist after retry")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(2),
            "action_attempts[1] should be 2 after retry scheduling"
        );
    }

    // Fail again with retry using real issued ticket
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should now be 3
    {
        let state = shard.run_state_get_mut(run).ok_or("run should exist")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(3),
            "action_attempts[1] should be 3 after second retry scheduling"
        );
    }

}