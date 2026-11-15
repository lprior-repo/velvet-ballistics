#[test]
fn timer_preparation_capacity_refusal_restores_checked_out_run_diagnostic() -> Result<(), String> {
    let run = RunId::new(86_030);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let timer = shard.pending_timer_remove(run).ok_or("initial timer")?;
    // Synthetic orphan timer fills the private boundary table while leaving
    // room to restore the real run. This is not public admission or an OOM test.
    let synthetic = RunId::new(86_031);
    shard
        .checked_out_run_insert(synthetic)
        .map_err(|error| error.to_string())?;
    shard
        .pending_timer_insert(synthetic, timer)
        .map_err(|error| error.to_string())?;
    shard.checked_out_run_remove(synthetic);
    shard.max_active_runs = 1;
    let state = shard.run_state_get(run).ok_or("waiting run")?.clone();
    let status = shard.runtime_state_get(run);
    let timers = shard.pending_timer_clone();
    let sequence = shard.journal_seq_get(run);
    let counters = shard.counters().snapshot();
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    let checked_out = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        shard.await_timer(run, checked_out, PendingTimerKind::Wait),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    ));
    assert_eq!(shard.run_state_get(run), Some(&state));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), status);
    assert_eq!(shard.pending_timer_clone(), timers);
    assert_eq!(shard.journal_seq_get(run), sequence);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    Ok(())
}
