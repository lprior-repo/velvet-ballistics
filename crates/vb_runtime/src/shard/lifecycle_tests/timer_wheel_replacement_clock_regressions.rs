fn drive_due_timer_at(shard: &mut Shard, now: std::time::Instant) -> Result<(), String> {
    shard
        .drive_one_due_timer_at(now)
        .map_err(|error| format!("{error:?}"))
}

fn assert_old_deadline_preserved(
    shard: &Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
    replacement: &crate::shard::types::PendingTimer,
    entry: crate::shard::timer_wheel::TimerEntry,
    state_before: &crate::shard::RunState,
    journal_before: &Vec<RuntimeJournalEvent>,
) -> Result<(), String> {
    assert!(!shard.terminal_runs_contains(run));
    assert_eq!(shard.run_state_get(run), Some(state_before));
    assert_replacement_unchanged(shard, run, replacement, entry)?;
    assert_eq!(
        shard.run_aggregate.pending_timer_next_deadline(),
        Some(replacement.deadline)
    );
    assert_eq!(collect_wait_resolved_runs(journal)?, Vec::<RunId>::new());
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        *journal_before
    );
    Ok(())
}

fn assert_exact_wait_resolution(
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    expected: RuntimeJournalEvent,
) -> Result<(), String> {
    let resolved = journal
        .snapshot()
        .map_err(|error| format!("{error:?}"))?
        .into_iter()
        .filter(|event| matches!(event, RuntimeJournalEvent::WaitResolved { .. }))
        .collect::<Vec<_>>();
    assert_eq!(resolved, vec![expected]);
    Ok(())
}

fn assert_replacement_fired(
    shard: &Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
    replacement: &crate::shard::types::PendingTimer,
) -> Result<(), String> {
    assert_eq!(shard.run_state_get(run), None);
    assert!(shard.terminal_runs_contains(run));
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.pending_timer_get(run), None);
    assert_eq!(shard.timer_entry(run), None);
    assert_eq!(shard.run_aggregate.pending_timer_next_deadline(), None);
    assert_exact_wait_resolution(
        journal,
        RuntimeJournalEvent::WaitResolved {
            run,
            step: replacement.step,
        },
    )?;
    Ok(())
}

fn observe_old_replacement_deadline(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
    old_deadline: std::time::Instant,
    replacement: &crate::shard::types::PendingTimer,
    entry: crate::shard::timer_wheel::TimerEntry,
) -> Result<(), String> {
    let state_before = shard
        .run_state_get(run)
        .ok_or("run must remain live")?
        .clone();
    let journal_before = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    drive_due_timer_at(shard, old_deadline)?;
    assert_old_deadline_preserved(
        shard,
        journal,
        run,
        replacement,
        entry,
        &state_before,
        &journal_before,
    )
}

fn assert_idle_after_replacement_fire(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    run: RunId,
    instant: std::time::Instant,
) -> Result<(), String> {
    let state_after_fire = shard.run_state_get(run).cloned();
    let journal_after_fire = journal.snapshot().map_err(|error| format!("{error:?}"))?;
    drive_due_timer_at(shard, instant)?;
    assert_eq!(
        journal.snapshot().map_err(|error| format!("{error:?}"))?,
        journal_after_fire
    );
    assert_eq!(shard.run_state_get(run).cloned(), state_after_fire);
    assert!(shard.terminal_runs_contains(run));
    Ok(())
}

#[test]
fn timer_wheel_shard_replacement_preserves_both_journal_boundaries() -> Result<(), String> {
    let run = RunId::new(90_011);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let old_deadline = std::time::Instant::now();
    let old = timer_due_regression_set_deadline(&mut shard, run, old_deadline)?;
    let replacement = future_timer_replacement(old.clone())?;
    let entry = insert_replacement(&mut shard, run, old, replacement.clone())?;
    assert!(replacement.deadline > old_deadline);
    observe_old_replacement_deadline(&mut shard, &journal, run, old_deadline, &replacement, entry)?;
    drive_due_timer_at(&mut shard, replacement.deadline)?;
    assert_replacement_fired(&shard, &journal, run, &replacement)?;
    assert_idle_after_replacement_fire(&mut shard, &journal, run, replacement.deadline)?;
    Ok(())
}
