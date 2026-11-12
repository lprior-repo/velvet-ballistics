fn timer_due_regression_fixture(
    run: RunId,
) -> Result<
    (
        Shard,
        std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    ),
    String,
> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    timer_due_regression_submit_wait(&mut shard, run)?;
    Ok((shard, journal))
}

fn timer_due_regression_submit_wait(shard: &mut Shard, run: RunId) -> Result<(), String> {
    let workflow = wait_workflow().ok_or("wait workflow fixture must compile")?;
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
    assert!(shard.pending_timer_contains(run));
    Ok(())
}

fn timer_due_regression_set_deadline(
    shard: &mut Shard,
    run: RunId,
    deadline: std::time::Instant,
) -> Result<crate::shard::types::PendingTimer, String> {
    let mut timer = shard
        .pending_timer_get(run)
        .ok_or("owned timer must exist")?;
    timer.deadline = deadline;
    let previous = shard
        .pending_timer_insert(run, timer)
        .map_err(|error| format!("{error:?}"))?;
    assert!(previous.is_some());
    Ok(timer)
}

#[test]
fn owned_timer_fired_before_deadline_preserves_all_run_authority() -> Result<(), String> {
    let run = RunId::new(84_001);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or("future Instant must be representable")?;
    let timer = timer_due_regression_set_deadline(&mut shard, run, deadline)?;
    let before = shard.run_state_get(run).ok_or("run must exist")?.clone();
    let sequence = shard.journal_seq_get(run);
    let events = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    let completed = shard.counters().snapshot().runs_completed;
    assert_eq!(
        before.frame.step_state(timer.step),
        Ok(vb_core::frame::StepState::Waiting)
    );
    let entry = shard.timer_entry(run).ok_or("captured timer must exist")?;
    shard
        .enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.run_state_get(run), Some(&before));
    assert_eq!(shard.pending_timer_get(run), Some(timer));
    assert_eq!(shard.journal_seq_get(run), sequence);
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        events
    );
    assert_eq!(shard.counters().snapshot().runs_completed, completed);
    assert!(!shard.terminal_runs_contains(run));
    Ok(())
}

struct IdleDueFixture {
    shard: Shard,
    journal: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    future_run: RunId,
    due_run: RunId,
    future_deadline: std::time::Instant,
    future_timer: crate::shard::types::PendingTimer,
    future_state: crate::shard::RunState,
    future_sequence: Option<vb_storage::EventSeq>,
}

fn schedule_due_wait(
    shard: &mut Shard,
    due_run: RunId,
) -> Result<crate::shard::types::PendingTimer, String> {
    timer_due_regression_submit_wait(shard, due_run)?;
    timer_due_regression_set_deadline(shard, due_run, std::time::Instant::now())
}

fn capture_future_state(
    shard: &Shard,
    future_run: RunId,
) -> Result<(crate::shard::RunState, Option<vb_storage::EventSeq>), String> {
    let state = shard
        .run_state_get(future_run)
        .ok_or("future run must exist")?
        .clone();
    Ok((state, shard.journal_seq_get(future_run)))
}

fn setup_idle_due_fixture() -> Result<IdleDueFixture, String> {
    let future_run = RunId::new(84_002);
    let due_run = RunId::new(84_003);
    let (mut shard, journal) = timer_due_regression_fixture(future_run)?;
    let future_deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or("future Instant must be representable")?;
    let future_timer = timer_due_regression_set_deadline(&mut shard, future_run, future_deadline)?;
    let due_timer = schedule_due_wait(&mut shard, due_run)?;
    let (future_state, future_sequence) = capture_future_state(&shard, future_run)?;
    let unowned = shard.pending_timer_insert(RunId::new(84_004), due_timer);
    assert_eq!(unowned, Err(RuntimeError::RunNotFound));
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.pending_timer_count(), 2);
    Ok(IdleDueFixture {
        shard,
        journal,
        future_run,
        due_run,
        future_deadline,
        future_timer,
        future_state,
        future_sequence,
    })
}

fn assert_due_run_finished(shard: &Shard, due_run: RunId) {
    assert!(shard.terminal_runs_contains(due_run));
    assert!(!shard.run_state_contains(due_run));
    assert_eq!(shard.pending_timer_get(due_run), None);
}

fn assert_future_timer_retained(fixture: &IdleDueFixture) {
    let shard = &fixture.shard;
    let future_run = fixture.future_run;
    let future_before = &fixture.future_state;
    let future_timer = fixture.future_timer;
    let future_sequence = fixture.future_sequence;
    let future_deadline = fixture.future_deadline;
    assert_eq!(shard.run_state_get(future_run), Some(future_before));
    assert_eq!(shard.pending_timer_get(future_run), Some(future_timer));
    assert_eq!(shard.journal_seq_get(future_run), future_sequence);
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(
        shard.run_aggregate.pending_timer_next_deadline(),
        Some(future_deadline)
    );
}

fn assert_due_resolution_once(after: &[RuntimeJournalEvent], due_run: RunId, future_run: RunId) {
    assert_eq!(
        after
            .iter()
            .filter(|event| matches!(
                event,
                RuntimeJournalEvent::WaitResolved { run, .. } if *run == due_run
            ))
            .count(),
        1
    );
    assert!(!after.iter().any(|event| matches!(
        event,
        RuntimeJournalEvent::WaitResolved { run, .. } if *run == future_run
    )));
}

fn assert_second_idle_tick_unchanged(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    after: Vec<RuntimeJournalEvent>,
) -> Result<(), String> {
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        after
    );
    Ok(())
}

fn assert_due_resolution_then_idle_tick(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    due_run: RunId,
    future_run: RunId,
) -> Result<(), String> {
    let after = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_due_resolution_once(&after, due_run, future_run);
    assert_second_idle_tick_unchanged(shard, journal, after)
}

#[test]
fn idle_tick_resolves_only_due_owned_timer_once() -> Result<(), String> {
    let mut fixture = setup_idle_due_fixture()?;
    assert_eq!(fixture.shard.tick(), Ok(true));
    assert_due_run_finished(&fixture.shard, fixture.due_run);
    assert_future_timer_retained(&fixture);
    let IdleDueFixture {
        mut shard,
        journal,
        future_run,
        due_run,
        future_timer,
        future_state: future_before,
        ..
    } = fixture;
    assert_due_resolution_then_idle_tick(&mut shard, &journal, due_run, future_run)?;
    assert_eq!(shard.pending_timer_get(future_run), Some(future_timer));
    assert_eq!(shard.run_state_get(future_run), Some(&future_before));
    Ok(())
}
