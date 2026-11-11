#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthoritySnapshot {
    run_state: crate::shard::RunState,
    pending_timers: indexmap::IndexMap<RunId, crate::shard::PendingTimer>,
    boundary: crate::shard::ShardPendingBoundarySnapshot,
    counters: crate::counters::CounterSnapshot,
    journal_events: Vec<crate::journal::RuntimeJournalEvent>,
}

fn authority_snapshot(
    shard: &Shard,
    run: RunId,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
) -> Result<AuthoritySnapshot, String> {
    let run_state = shard
        .run_state_get(run)
        .ok_or_else(|| String::from("run state must exist"))?
        .clone();
    let journal_events = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    Ok(AuthoritySnapshot {
        run_state,
        pending_timers: shard.pending_timer_clone(),
        boundary: shard.pending_boundary_snapshot(0, 2),
        counters: shard.counters().snapshot(),
        journal_events,
    })
}

fn assert_authority_unchanged(
    shard: &Shard,
    run: RunId,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    before: &AuthoritySnapshot,
) -> Result<(), String> {
    assert_eq!(authority_snapshot(shard, run, journal)?, *before);
    Ok(())
}

fn authority_due_deadlines() -> Result<(std::time::Instant, std::time::Instant), String> {
    let now = std::time::Instant::now();
    let due = now
        .checked_sub(std::time::Duration::from_secs(30))
        .ok_or_else(|| String::from("due deadline must be representable"))?;
    Ok((now, due))
}

fn authority_timer_entry(
    shard: &Shard,
    run: RunId,
) -> Result<crate::shard::timer_wheel::TimerEntry, String> {
    shard
        .timer_entry(run)
        .ok_or_else(|| String::from("captured timer entry must exist"))
}

fn assert_step_only_authority_replacement(shard: &mut Shard, run: RunId) -> Result<(), String> {
    let original = shard
        .pending_timer_get(run)
        .ok_or_else(|| String::from("owned timer must exist"))?;
    let old_token = authority_timer_entry(shard, run)?;
    let old = crate::shard::PendingTimer {
        step: StepIdx::ZERO,
        ..original
    };
    assert_eq!(shard.pending_timer_insert(run, old), Ok(Some(original)));
    assert_eq!(shard.pending_timer_get(run), Some(old));
    assert_eq!(authority_timer_entry(shard, run)?, old_token);
    let replacement = crate::shard::PendingTimer {
        step: StepIdx::new(1),
        ..old
    };
    assert_eq!(shard.pending_timer_insert(run, replacement), Ok(Some(old)));
    assert_eq!(shard.pending_timer_get(run), Some(replacement));
    assert_eq!(authority_timer_entry(shard, run)?, old_token);
    Ok(())
}

fn authority_next_replacement(
    old_timer: crate::shard::PendingTimer,
    deadline: std::time::Instant,
) -> Result<crate::shard::PendingTimer, String> {
    let generation = old_timer
        .generation
        .checked_add(1)
        .ok_or_else(|| String::from("timer generation must advance"))?;
    Ok(crate::shard::PendingTimer {
        generation,
        deadline,
        ..old_timer
    })
}

fn insert_authority_replacement(
    shard: &mut Shard,
    run: RunId,
    old_timer: crate::shard::PendingTimer,
    replacement: crate::shard::PendingTimer,
) -> Result<crate::shard::timer_wheel::TimerEntry, String> {
    assert_eq!(
        shard.pending_timer_insert(run, replacement),
        Ok(Some(old_timer))
    );
    assert_eq!(shard.pending_timer_get(run), Some(replacement));
    let expected = crate::shard::timer_wheel::TimerEntry {
        run,
        generation: replacement.generation,
        deadline: replacement.deadline,
        kind: replacement.kind,
    };
    let captured = authority_timer_entry(shard, run)?;
    assert_eq!(captured, expected);
    Ok(captured)
}

fn install_due_replacement_and_capture_old_token(
    shard: &mut Shard,
    run: RunId,
    due_deadline: std::time::Instant,
) -> Result<
    (
        crate::shard::timer_wheel::TimerEntry,
        crate::shard::timer_wheel::TimerEntry,
    ),
    String,
> {
    let old_timer = timer_due_regression_set_deadline(shard, run, due_deadline)?;
    let old_token = authority_timer_entry(shard, run)?;
    let replacement = authority_next_replacement(old_timer, due_deadline)?;
    let current_token = insert_authority_replacement(shard, run, old_timer, replacement)?;
    Ok((old_token, current_token))
}

fn enqueue_authority_token(
    shard: &mut Shard,
    token: crate::shard::timer_wheel::TimerEntry,
) -> Result<(), String> {
    shard
        .enqueue(ShardCommand::TimerFired {
            run: token.run,
            generation: token.generation,
            deadline: token.deadline,
            kind: token.kind,
        })
        .map_err(|error| format!("{error:?}"))
}

fn wait_resolution_count(events: &[crate::journal::RuntimeJournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, crate::journal::RuntimeJournalEvent::WaitResolved { run: event_run, .. } if *event_run == run))
        .count()
}

fn assert_idle_tick_preserves_wait_journal(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
    expected: &[crate::journal::RuntimeJournalEvent],
) -> Result<(), String> {
    assert_eq!(shard.tick(), Ok(true));
    let actual = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_eq!(actual, expected);
    assert_eq!(wait_resolution_count(&actual, run), 1);
    Ok(())
}

fn authority_set_maximum_timer_generation(
    shard: &mut Shard,
    run: RunId,
) -> Result<(StepIdx, crate::shard::PendingTimerKind), String> {
    let original = shard
        .pending_timer_get(run)
        .ok_or_else(|| String::from("owned timer must exist"))?;
    let maximum = crate::shard::PendingTimer {
        generation: u64::MAX,
        ..original
    };
    assert_eq!(shard.pending_timer_insert(run, maximum), Ok(Some(original)));
    assert_eq!(shard.pending_timer_get(run), Some(maximum));
    Ok((maximum.step, maximum.kind))
}

#[test]
fn timer_wheel_authority_step_replacement_preserves_old_token() -> Result<(), String> {
    let run = RunId::new(84_010);
    let (mut shard, _journal) = timer_due_regression_fixture(run)?;
    assert_step_only_authority_replacement(&mut shard, run)?;
    Ok(())
}

#[test]
fn timer_wheel_authority_stale_token_preserves_whole_shard_state() -> Result<(), String> {
    let run = RunId::new(84_011);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let (now, due_deadline) = authority_due_deadlines()?;
    assert!(due_deadline < now);
    let (old_token, current_token) =
        install_due_replacement_and_capture_old_token(&mut shard, run, due_deadline)?;
    let next_generation = old_token
        .generation
        .checked_add(1)
        .ok_or_else(|| String::from("timer generation must advance"))?;
    assert_eq!(current_token.generation, next_generation);
    assert_eq!(current_token.deadline, old_token.deadline);
    assert_eq!(current_token.kind, old_token.kind);
    let before = authority_snapshot(&shard, run, &journal)?;
    enqueue_authority_token(&mut shard, old_token)?;
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_authority_unchanged(&shard, run, &journal, &before)?;
    Ok(())
}

#[test]
fn timer_wheel_authority_current_due_token_resolves_wait() -> Result<(), String> {
    let run = RunId::new(84_013);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let (_, deadline) = authority_due_deadlines()?;
    let timer = timer_due_regression_set_deadline(&mut shard, run, deadline)?;
    let token = authority_timer_entry(&shard, run)?;
    assert_eq!(
        token,
        crate::shard::timer_wheel::TimerEntry {
            run,
            generation: timer.generation,
            deadline,
            kind: timer.kind,
        }
    );
    enqueue_authority_token(&mut shard, token)?;
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(run));
    assert_eq!(shard.pending_timer_get(run), None);
    let events = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_eq!(wait_resolution_count(&events, run), 1);
    assert_idle_tick_preserves_wait_journal(&mut shard, &journal, run, &events)?;
    Ok(())
}

#[test]
fn timer_wheel_authority_max_generation_blocks_recovery_atomically() -> Result<(), String> {
    let run = RunId::new(84_012);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let (step, kind) = authority_set_maximum_timer_generation(&mut shard, run)?;
    let before = authority_snapshot(&shard, run, &journal)?;
    assert_eq!(shard.next_pending_timer_generation(run), None);
    assert_eq!(
        shard.restore_recovered_timer(run, step, kind),
        Err(RuntimeError::InvalidTimerFire)
    );
    assert_authority_unchanged(&shard, run, &journal, &before)?;
    Ok(())
}
