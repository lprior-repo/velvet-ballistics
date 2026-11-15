#[test]
fn answered_ask_cancels_only_own_timer_and_stale_fire_preserves_authority() -> Result<(), String> {
    let ask = RunId::new(84_030);
    let wait = RunId::new(84_031);
    let (mut shard, journal) = timer_due_regression_fixture(wait)?;
    let future = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or("future deadline")?;
    let wait_timer = timer_due_regression_set_deadline(&mut shard, wait, future)?;
    shard
        .enqueue(ShardCommand::SubmitWithContracts {
            run: ask,
            workflow: ask_workflow().ok_or("ask workflow")?,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.tick(), Ok(true));
    timer_due_regression_set_deadline(&mut shard, ask, future)?;
    let stale = shard.timer_entry(ask).ok_or("ask timer")?;
    let ticket = shard
        .lookup_pending_ask_ticket(ask, StepIdx::new(2))
        .ok_or("ask ticket")?;
    let wait_state = shard.run_state_get(wait).ok_or("wait state")?.clone();
    let wait_seq = shard.journal_seq_get(wait);
    let wait_status = shard.runtime_state_get(wait);
    assert_eq!(shard.pending_timer_count(), 2);
    shard
        .enqueue(ShardCommand::AskAnswered {
            answer: AskAnswer {
                ticket,
                answer_slot: SlotIdx::new(2),
                value: SlotValue::I64(42),
                taint: Taint::Clean,
                encoded_len: 0,
            },
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.pending_timer_get(ask), None);
    assert_eq!(shard.timer_entry(ask), None);
    assert_eq!(shard.lookup_pending_ask_ticket(ask, StepIdx::new(2)), None);
    assert!(shard.terminal_runs_contains(ask));
    assert_eq!(shard.run_state_get(ask), None);
    assert_eq!(shard.pending_timer_get(wait), Some(wait_timer));
    assert_eq!(shard.run_state_get(wait), Some(&wait_state));
    assert_eq!(shard.journal_seq_get(wait), wait_seq);
    assert_eq!(shard.runtime_state_get(wait), wait_status);
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
        RuntimeJournalEvent::AskAnswered { run, .. } if *run == ask))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
        RuntimeJournalEvent::AskTimedOut { run, .. } if *run == ask))
            .count(),
        0
    );
    let counters = shard.counters().snapshot();
    let ask_seq = shard.journal_seq_get(ask);
    let ask_status = shard.runtime_state_get(ask);
    shard
        .enqueue(ShardCommand::TimerFired {
            run: stale.run,
            generation: stale.generation,
            deadline: stale.deadline,
            kind: stale.kind,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.pending_timer_get(wait), Some(wait_timer));
    assert_eq!(shard.run_state_get(wait), Some(&wait_state));
    assert_eq!(shard.journal_seq_get(wait), wait_seq);
    assert_eq!(shard.runtime_state_get(wait), wait_status);
    assert_eq!(shard.journal_seq_get(ask), ask_seq);
    assert_eq!(shard.runtime_state_get(ask), ask_status);
    assert_eq!(shard.run_state_get(ask), None);
    assert!(shard.terminal_runs_contains(ask));
    assert_eq!(shard.pending_timer_get(ask), None);
    assert_eq!(shard.lookup_pending_ask_ticket(ask, StepIdx::new(2)), None);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    Ok(())
}
