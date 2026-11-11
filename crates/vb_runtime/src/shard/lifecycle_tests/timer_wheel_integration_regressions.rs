use std::time::Duration;

fn collect_wait_resolved_runs(
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
) -> Result<Vec<RunId>, String> {
    let events = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    Ok(events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::WaitResolved { run, .. } => Some(*run),
            _ => None,
        })
        .collect())
}

fn assert_idle_tick_keeps_journal(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    before: &Vec<crate::journal::RuntimeJournalEvent>,
) -> Result<(), String> {
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        *before
    );
    Ok(())
}

fn assert_timer_unchanged(
    shard: &Shard,
    run: RunId,
    expected: &crate::shard::types::PendingTimer,
) -> Result<(), String> {
    let actual = shard
        .pending_timer_get(run)
        .ok_or("timer must remain owned")?;
    assert_eq!(&actual, expected);
    Ok(())
}

fn setup_due_high_low(
    high_run: RunId,
    low_run: RunId,
) -> Result<
    (
        Shard,
        std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
        crate::shard::types::PendingTimer,
    ),
    String,
> {
    let (mut shard, journal) = timer_due_regression_fixture(high_run)?;
    timer_due_regression_submit_wait(&mut shard, low_run)?;
    let due = std::time::Instant::now();
    let high_timer = timer_due_regression_set_deadline(&mut shard, high_run, due)?;
    timer_due_regression_set_deadline(&mut shard, low_run, due)?;
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.pending_timer_count(), 2);
    Ok((shard, journal, high_timer))
}

fn future_timer_replacement(
    old: crate::shard::types::PendingTimer,
) -> Result<crate::shard::types::PendingTimer, String> {
    let deadline = old
        .deadline
        .checked_add(Duration::from_secs(86_400))
        .ok_or("deadline must be representable")?;
    let generation = old
        .generation
        .checked_add(1)
        .ok_or("generation must increment")?;
    let mut replacement = old.clone();
    replacement.deadline = deadline;
    replacement.generation = generation;
    Ok(replacement)
}

fn insert_replacement(
    shard: &mut Shard,
    run: RunId,
    old: crate::shard::types::PendingTimer,
    replacement: crate::shard::types::PendingTimer,
) -> Result<crate::shard::timer_wheel::TimerEntry, String> {
    let previous = shard
        .pending_timer_insert(run, replacement.clone())
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(previous, Some(old));
    let expected = crate::shard::timer_wheel::TimerEntry {
        run,
        generation: replacement.generation,
        deadline: replacement.deadline,
        kind: replacement.kind,
    };
    assert_eq!(shard.timer_entry(run), Some(expected));
    assert_eq!(shard.pending_timer_count(), 1);
    Ok(expected)
}

fn assert_replacement_unchanged(
    shard: &Shard,
    run: RunId,
    timer: &crate::shard::types::PendingTimer,
    entry: crate::shard::timer_wheel::TimerEntry,
) -> Result<(), String> {
    assert_timer_unchanged(shard, run, timer)?;
    assert_eq!(shard.timer_entry(run), Some(entry));
    assert_eq!(shard.pending_timer_count(), 1);
    Ok(())
}

fn cancel_and_check_survivor(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    survivor: RunId,
    cancelled: RunId,
    cancelled_token: crate::shard::timer_wheel::TimerEntry,
    timer: &crate::shard::types::PendingTimer,
) -> Result<(), String> {
    let state = shard
        .run_state_get(survivor)
        .ok_or("survivor must exist")?
        .clone();
    shard
        .enqueue(ShardCommand::Cancel {
            run: cancelled,
            reason: None,
        })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.run_state_get(survivor), Some(&state));
    assert_cancelled_timer_removed(shard, survivor, cancelled, timer)?;
    assert_eq!(collect_wait_resolved_runs(journal)?, Vec::<RunId>::new());
    assert_stale_cancelled_timer_rejected(shard, journal, survivor, cancelled_token)?;
    Ok(())
}

fn assert_cancelled_timer_removed(
    shard: &Shard,
    survivor: RunId,
    cancelled: RunId,
    timer: &crate::shard::types::PendingTimer,
) -> Result<(), String> {
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.pending_timer_get(cancelled), None);
    assert_eq!(shard.timer_entry(cancelled), None);
    assert_timer_unchanged(shard, survivor, timer)?;
    let snapshot = shard.pending_boundary_snapshot(0, 2);
    assert_eq!(snapshot.pending_timer_count(), 1);
    assert_eq!(snapshot.pending_timers().len(), 1);
    let entry = snapshot
        .pending_timers()
        .first()
        .ok_or("survivor snapshot missing")?;
    assert_eq!(entry.run_id(), survivor);
    assert_eq!(entry.step(), timer.step);
    assert_eq!(entry.kind(), timer.kind);
    assert_eq!(entry.generation(), timer.generation);
    assert_eq!(entry.deadline(), timer.deadline);
    let next_deadline = shard.run_aggregate.pending_timer_next_deadline();
    assert_eq!(next_deadline, Some(timer.deadline));
    Ok(())
}

#[test]
fn timer_wheel_shard_same_deadline_fires_lower_run_first() -> Result<(), String> {
    let high = RunId::new(90_002);
    let low = RunId::new(90_001);
    let (mut shard, journal, high_timer) = setup_due_high_low(high, low)?;
    assert_eq!(collect_wait_resolved_runs(&journal)?, Vec::<RunId>::new());
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(low));
    assert!(shard.run_state_contains(high));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_timer_unchanged(&shard, high, &high_timer)?;
    assert_eq!(collect_wait_resolved_runs(&journal)?, vec![low]);
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(high));
    assert_eq!(shard.pending_timer_count(), 0);
    let resolved = collect_wait_resolved_runs(&journal)?;
    assert_eq!(resolved, vec![low, high]);
    let after = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_idle_tick_keeps_journal(&mut shard, &journal, &after)?;
    assert_eq!(collect_wait_resolved_runs(&journal)?, resolved);
    Ok(())
}

#[test]
fn timer_wheel_shard_same_run_replacement_uses_new_deadline_and_generation() -> Result<(), String> {
    let run = RunId::new(90_010);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let old_deadline = std::time::Instant::now();
    let old = timer_due_regression_set_deadline(&mut shard, run, old_deadline)?;
    let replacement = future_timer_replacement(old.clone())?;
    let entry = insert_replacement(&mut shard, run, old, replacement.clone())?;
    let journal_before = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    let state_before = shard
        .run_state_get(run)
        .ok_or("run must remain live")?
        .clone();
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.terminal_runs_contains(run));
    assert_eq!(shard.run_state_get(run), Some(&state_before));
    assert_replacement_unchanged(&shard, run, &replacement, entry)?;
    assert_eq!(collect_wait_resolved_runs(&journal)?, Vec::<RunId>::new());
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        journal_before
    );
    Ok(())
}

#[test]
fn timer_wheel_shard_cancel_removes_one_same_time_entry_and_prevents_resolution()
-> Result<(), String> {
    let survivor = RunId::new(90_020);
    let cancelled = RunId::new(90_021);
    let (mut shard, journal) = timer_due_regression_fixture(survivor)?;
    timer_due_regression_submit_wait(&mut shard, cancelled)?;
    let due = std::time::Instant::now();
    let survivor_timer = timer_due_regression_set_deadline(&mut shard, survivor, due)?;
    timer_due_regression_set_deadline(&mut shard, cancelled, due)?;
    let cancelled_token = shard
        .timer_entry(cancelled)
        .ok_or("cancelled timer token must exist")?;
    assert_eq!(shard.pending_timer_count(), 2);
    cancel_and_check_survivor(
        &mut shard,
        &journal,
        survivor,
        cancelled,
        cancelled_token,
        &survivor_timer,
    )?;
    assert_survivor_resolves_once(&mut shard, &journal, survivor)?;
    Ok(())
}

fn assert_survivor_resolves_once(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    survivor: RunId,
) -> Result<(), String> {
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(survivor));
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.pending_timer_get(survivor), None);
    assert_eq!(shard.run_aggregate.pending_timer_next_deadline(), None);
    let resolved = collect_wait_resolved_runs(journal)?;
    assert_eq!(resolved, vec![survivor]);
    let after = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_idle_tick_keeps_journal(shard, journal, &after)?;
    assert_eq!(collect_wait_resolved_runs(journal)?, resolved);
    Ok(())
}

#[test]
fn timer_wheel_shard_insert_non_live_run_fails_without_mutation() -> Result<(), String> {
    let live = RunId::new(90_030);
    let (mut shard, journal) = timer_due_regression_fixture(live)?;
    let captured = shard
        .pending_timer_get(live)
        .ok_or("fixture timer must exist")?;
    let entry_before = shard.timer_entry(live).ok_or("timer entry must exist")?;
    let pending_before = shard.pending_timer_clone();
    let count_before = shard.pending_timer_count();
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    let result = shard.pending_timer_insert(RunId::new(90_099), captured.clone());
    assert_eq!(result, Err(RuntimeError::RunNotFound));
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.pending_timer_clone(), pending_before);
    assert_eq!(shard.pending_timer_count(), count_before);
    assert_timer_unchanged(&shard, live, &captured)?;
    assert_eq!(shard.timer_entry(live), Some(entry_before));
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        journal_before
    );
    Ok(())
}
