struct AskAnswerCleanupFixture {
    shard: Shard,
    journal: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    ask_run: RunId,
    wait_run: RunId,
    ask_timer: crate::shard::types::PendingTimer,
    wait_timer: crate::shard::types::PendingTimer,
    ask_entry: crate::shard::timer_wheel::TimerEntry,
    ask_ticket: AskTicket,
    wait_state: crate::shard::RunState,
}

fn ask_cleanup_deadline(offset: u64) -> Result<std::time::Instant, String> {
    let duration = std::time::Duration::from_secs(offset);
    std::time::Instant::now()
        .checked_add(duration)
        .ok_or_else(|| String::from("future deadline overflow"))
}

fn setup_ask_run(shard: &mut Shard, run: RunId) -> Result<(), String> {
    let workflow = ask_workflow().ok_or_else(|| String::from("ask workflow must compile"))?;
    shard
        .enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
        })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

fn expected_timer_entry(
    run: RunId,
    timer: crate::shard::types::PendingTimer,
) -> crate::shard::timer_wheel::TimerEntry {
    crate::shard::timer_wheel::TimerEntry {
        run,
        generation: timer.generation,
        deadline: timer.deadline,
        kind: timer.kind,
    }
}

fn capture_ask_entry(
    shard: &Shard,
    run: RunId,
) -> Result<crate::shard::timer_wheel::TimerEntry, String> {
    shard
        .timer_entry(run)
        .ok_or_else(|| String::from("ask timer entry must exist"))
}

fn assert_ask_ticket(ticket: AskTicket, run: RunId) {
    assert_eq!(ticket.run(), run);
    assert_eq!(ticket.ask_step(), StepIdx::new(2));
    assert_eq!(ticket.resume_step(), StepIdx::new(3));
    assert_eq!(ticket.answer_slot(), SlotIdx::new(2));
}

fn setup_ask_authority(
    shard: &mut Shard,
    run: RunId,
) -> Result<
    (
        crate::shard::types::PendingTimer,
        crate::shard::timer_wheel::TimerEntry,
        AskTicket,
    ),
    String,
> {
    setup_ask_run(shard, run)?;
    let deadline = ask_cleanup_deadline(3_600)?;
    let timer = timer_due_regression_set_deadline(shard, run, deadline)?;
    let entry = capture_ask_entry(shard, run)?;
    assert_eq!(entry, expected_timer_entry(run, timer));
    let ticket = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or_else(|| String::from("ask answer ticket must be issued"))?;
    assert_ask_ticket(ticket, run);
    Ok((timer, entry, ticket))
}

fn setup_wait_authority(
    shard: &mut Shard,
) -> Result<
    (
        RunId,
        crate::shard::types::PendingTimer,
        crate::shard::RunState,
    ),
    String,
> {
    let run = RunId::new(55_002);
    timer_due_regression_submit_wait(shard, run)?;
    let deadline = ask_cleanup_deadline(7_200)?;
    let timer = timer_due_regression_set_deadline(shard, run, deadline)?;
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| String::from("wait survivor state must exist"))?
        .clone();
    Ok((run, timer, state))
}

fn ask_answer_cleanup_fixture() -> Result<AskAnswerCleanupFixture, String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let ask_run = RunId::new(55_001);
    let (ask_timer, ask_entry, ask_ticket) = setup_ask_authority(&mut shard, ask_run)?;
    let (wait_run, wait_timer, wait_state) = setup_wait_authority(&mut shard)?;
    assert_eq!(shard.pending_timer_count(), 2);
    Ok(AskAnswerCleanupFixture {
        shard,
        journal,
        ask_run,
        wait_run,
        ask_timer,
        wait_timer,
        ask_entry,
        ask_ticket,
        wait_state,
    })
}

fn answer_ask_and_tick(shard: &mut Shard, ticket: AskTicket) -> Result<(), String> {
    let answer = AskAnswer {
        ticket,
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    shard
        .enqueue(ShardCommand::AskAnswered { answer })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

fn assert_ask_timer_removed(fixture: &AskAnswerCleanupFixture) -> Result<(), String> {
    assert_eq!(
        fixture.ask_entry,
        expected_timer_entry(fixture.ask_run, fixture.ask_timer)
    );
    assert_eq!(fixture.shard.pending_timer_get(fixture.ask_run), None);
    assert_eq!(fixture.shard.timer_entry(fixture.ask_run), None);
    assert_eq!(
        fixture
            .shard
            .lookup_pending_ask_ticket(fixture.ask_run, StepIdx::new(2)),
        None
    );
    assert!(fixture.shard.terminal_runs_contains(fixture.ask_run));
    assert_eq!(fixture.shard.run_state_get(fixture.ask_run), None);
    Ok(())
}

fn assert_survivor_unchanged(fixture: &AskAnswerCleanupFixture) -> Result<(), String> {
    assert_eq!(
        fixture.shard.pending_timer_get(fixture.wait_run),
        Some(fixture.wait_timer)
    );
    assert_eq!(
        fixture.shard.timer_entry(fixture.wait_run),
        Some(expected_timer_entry(fixture.wait_run, fixture.wait_timer))
    );
    assert_eq!(
        fixture.shard.run_state_get(fixture.wait_run),
        Some(&fixture.wait_state)
    );
    Ok(())
}

fn assert_boundary_snapshot_fields(fixture: &AskAnswerCleanupFixture) -> Result<(), String> {
    let boundary = fixture.shard.pending_boundary_snapshot(0, 2);
    assert_eq!(boundary.pending_timer_count(), 1);
    assert_eq!(boundary.pending_timers().len(), 1);
    let timer = boundary
        .pending_timers()
        .first()
        .ok_or_else(|| String::from("survivor timer snapshot must exist"))?;
    assert_eq!(timer.run_id(), fixture.wait_run);
    assert_eq!(timer.step(), fixture.wait_timer.step);
    assert_eq!(timer.kind(), fixture.wait_timer.kind);
    assert_eq!(timer.generation(), fixture.wait_timer.generation);
    assert_eq!(timer.deadline(), fixture.wait_timer.deadline);
    Ok(())
}

fn assert_due_peek(fixture: &AskAnswerCleanupFixture) -> Result<(), String> {
    let wheel = &fixture.shard.run_aggregate;
    let expected = expected_timer_entry(fixture.wait_run, fixture.wait_timer);
    assert_eq!(
        wheel.pending_timer_due_peek(fixture.wait_timer.deadline),
        Some(expected)
    );
    let before = fixture
        .wait_timer
        .deadline
        .checked_sub(std::time::Duration::from_nanos(1))
        .ok_or_else(|| String::from("survivor deadline must allow a prior instant"))?;
    assert_eq!(wheel.pending_timer_due_peek(before), None);
    Ok(())
}

fn assert_boundary_and_due_peek(fixture: &AskAnswerCleanupFixture) -> Result<(), String> {
    assert_boundary_snapshot_fields(fixture)?;
    assert_due_peek(fixture)?;
    Ok(())
}

fn assert_ask_event_counts(
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
) -> Result<(), String> {
    let events = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    let answered = events
        .iter()
        .filter(|event| matches!(event, RuntimeJournalEvent::AskAnswered { run: event_run, .. } if *event_run == run))
        .count();
    let timed_out = events
        .iter()
        .filter(|event| matches!(event, RuntimeJournalEvent::AskTimedOut { run: event_run, .. } if *event_run == run))
        .count();
    assert_eq!(answered, 1);
    assert_eq!(timed_out, 0);
    Ok(())
}

fn enqueue_stale_ask_fire(
    shard: &mut Shard,
    entry: crate::shard::timer_wheel::TimerEntry,
) -> Result<(), String> {
    shard
        .enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        })
        .map_err(|error| format!("{error:?}"))
}

fn assert_stale_rejected(
    fixture: &mut AskAnswerCleanupFixture,
    journal_before: &[RuntimeJournalEvent],
) -> Result<(), String> {
    let counters_before = fixture.shard.counters().snapshot();
    enqueue_stale_ask_fire(&mut fixture.shard, fixture.ask_entry)?;
    assert_eq!(fixture.shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(fixture.shard.counters().snapshot(), counters_before);
    let after = fixture
        .journal
        .snapshot()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(after, journal_before);
    Ok(())
}

#[test]
fn ask_answer_removes_only_ask_timer_and_stale_fire_is_rejected() -> Result<(), String> {
    let mut fixture = ask_answer_cleanup_fixture()?;
    answer_ask_and_tick(&mut fixture.shard, fixture.ask_ticket)?;
    assert_eq!(fixture.shard.pending_timer_count(), 1);
    assert_ask_timer_removed(&fixture)?;
    assert_survivor_unchanged(&fixture)?;
    assert_boundary_and_due_peek(&fixture)?;
    assert_ask_event_counts(&fixture.journal, fixture.ask_run)?;
    let before_stale = fixture
        .journal
        .snapshot()
        .map_err(|error| format!("{error:?}"))?;
    assert_stale_rejected(&mut fixture, &before_stale)?;
    assert_ask_timer_removed(&fixture)?;
    assert_survivor_unchanged(&fixture)?;
    assert_ask_event_counts(&fixture.journal, fixture.ask_run)?;
    Ok(())
}
