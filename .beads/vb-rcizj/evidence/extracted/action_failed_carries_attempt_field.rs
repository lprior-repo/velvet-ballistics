fn action_failed_carries_attempt_field() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = suspended_workflow() else {
        return Err("missing workflow fixture".into());
    };
    let run = RunId::new(11);

    // Submit and drive to action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Fail action with real issued ticket
    let ticket = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket")?;
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Check journal event carries attempt
    let events = journal.snapshot()?;
    let action_failed_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::ActionFailed {
                run: r,
                step: s,
                action: _,
                attempt: a, // POST-003: attempt field should exist
            } if *r == run && *s == StepIdx::ZERO => Some(*a),
            _ => None,
        })
        .collect();

    assert!(
        !action_failed_events.is_empty(),
        "ActionFailed event should be in journal"
    );
    assert_eq!(
        action_failed_events[0], 1,
        "ActionFailed should carry attempt=1 from ticket"
    );

}