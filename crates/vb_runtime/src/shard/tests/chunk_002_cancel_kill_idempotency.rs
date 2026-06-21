// Tests for cancel/kill idempotency (PO-vb-pymh-011).
//
// These tests verify that cancel and kill operations are idempotent:
// - cancel_idempotent_property: Calling cancel twice returns Ok both times
// - kill_idempotent_property: Calling kill twice returns Ok both times

// suspended_workflow() and small_config() are defined in earlier chunks
// wait_workflow() is defined in chunk_dispatch_error_semantics.rs (also
// included in this module)

/// Test cancel_idempotent_property: Calling cancel twice on same run returns Ok both times.
#[test]
fn cancel_idempotent_on_active_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(300);

    // Submit the workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First cancel should succeed
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Second cancel should ALSO succeed (idempotent)
    // because the run is now in terminal_runs
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Counter should only be incremented once
    assert_eq!(shard.counters().snapshot().runs_cancelled, 1);
    Ok(())
}

/// Test kill_idempotent_property: Calling kill twice on same run returns Ok both times.
#[test]
fn kill_idempotent_on_active_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(301);

    // Submit the workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First kill should succeed
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Second kill should ALSO succeed (idempotent)
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Counter should only be incremented once
    assert_eq!(shard.counters().snapshot().runs_killed, 1);
    Ok(())
}

/// Test cancel on a run that was already cancelled via cancel.
#[test]
fn cancel_twice_after_first_cancel_moves_to_terminal() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(302);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First cancel
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Verify run is no longer in active runs
    assert_eq!(shard.run_state_contains(run), false);
    // But is in terminal_runs
    assert_eq!(shard.terminal_runs_contains(run), true);

    // Second cancel - should still return Ok
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

/// Test kill on a run that was already killed via kill.
#[test]
fn kill_twice_after_first_kill_moves_to_terminal() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(303);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First kill
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Verify run is no longer in active runs
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);

    // Second kill
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

/// Test cancel_nonexistent_run_not_in_terminal_error: Run not in runs or terminal_runs → Err(RunNotFound).
#[test]
fn cancel_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Cancel a run that never existed
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

/// Test kill_nonexistent_run_not_in_terminal_error: Kill on unknown run → Err(RunNotFound).
#[test]
fn kill_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Kill a run that never existed
    assert_eq!(
        shard.enqueue(ShardCommand::Kill {
            run: RunId::new(998),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

/// Test cancel_removes_from_runs_and_inserts_terminal.
#[test]
fn cancel_removes_from_runs_inserts_terminal() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(304);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Before cancel: run is in active runs
    assert_eq!(shard.run_state_contains(run), true);
    assert_eq!(shard.terminal_runs_contains(run), false);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // After cancel: run is removed from active, added to terminal
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

/// Test kill_removes_from_runs_and_inserts_terminal.
#[test]
fn kill_removes_from_runs_inserts_terminal() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(305);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Before kill: run is in active runs
    assert_eq!(shard.run_state_contains(run), true);
    assert_eq!(shard.terminal_runs_contains(run), false);

    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // After kill: run is removed from active, added to terminal
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

/// Test cancel_releases_frame_to_pool.
#[test]
fn cancel_releases_frame_to_pool() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(306);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Frame is allocated
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(0)
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Frame is released
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(1)
    );
    Ok(())
}

/// Test kill_releases_frame_to_pool.
#[test]
fn kill_releases_frame_to_pool() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(307);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Frame is allocated
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(0)
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Frame is released
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(1)
    );
    Ok(())
}

// =============================================================================
// RQ-W0-05: Cancel-after-Kill and Kill-after-Cancel cross-case tests.
// These exercise the ordering between cancel and kill on the same run,
// proving that the terminalization guarantees hold across cross-cases.
// =============================================================================

/// Test cancel_after_kill: Cancel issued after Kill must be a typed no-op.
/// The run was already terminalized by Kill, so Cancel must:
/// - return Ok
/// - NOT insert a new RunCancelled journal event
/// - NOT increment the runs_cancelled counter (RQ-W0-17)
/// - leave terminal_runs monotonic (Killed outcome preserved)
#[test]
fn cancel_after_kill_is_typed_noop() -> Result<(), RuntimeError> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(310);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First: Kill the run
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // After kill: run is in terminal_runs, no longer in active runs
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);
    let baseline_cancelled = shard.counters().snapshot().runs_cancelled;

    // Second: Cancel the already-killed run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Counter must NOT increment - terminalization already happened.
    assert_eq!(shard.counters().snapshot().runs_cancelled, baseline_cancelled);

    // No new RunCancelled journal event was appended.
    let events = journal
        .snapshot()
        .map_err(|_| RuntimeError::QueueFull)?;
    let cancelled_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    assert_eq!(
        cancelled_count, 0,
        "cancel-after-kill must not emit RunCancelled journal event: {events:?}"
    );

    // Killed outcome preserved in terminal_runs
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

/// Test kill_after_cancel: Kill issued after Cancel must be a typed no-op.
/// The run was already terminalized by Cancel, so Kill must:
/// - return Ok
/// - NOT insert a new RunKilled journal event
/// - NOT increment the runs_killed counter (RQ-W0-17)
/// - leave terminal_runs monotonic (Cancelled outcome preserved)
#[test]
fn kill_after_cancel_is_typed_noop() -> Result<(), RuntimeError> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(311);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First: Cancel the run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // After cancel: run is in terminal_runs, no longer in active runs
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);
    let baseline_killed = shard.counters().snapshot().runs_killed;

    // Second: Kill the already-cancelled run
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Counter must NOT increment - terminalization already happened.
    assert_eq!(shard.counters().snapshot().runs_killed, baseline_killed);

    // No new RunKilled journal event was appended.
    let events = journal
        .snapshot()
        .map_err(|_| RuntimeError::QueueFull)?;
    let killed_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r, .. } if *r == run))
        .count();
    assert_eq!(
        killed_count, 0,
        "kill-after-cancel must not emit RunKilled journal event: {events:?}"
    );

    // Cancelled outcome preserved in terminal_runs
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

/// Test cancel_after_kill_clears_pending_timer: Cancel after Kill does not
/// re-clear a timer that was already cleared. This guards against any
/// spurious pending_timer_remove during the cross-case.
#[test]
fn cancel_after_kill_does_not_re_clear_timer() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(312);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);

    // Kill removes the pending timer
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);

    // Cancel after kill: no error, no new timer operations
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
    Ok(())
}

/// Test kill_after_cancel_clears_pending_timer: Kill after Cancel does not
/// re-clear a timer that was already cleared.
#[test]
fn kill_after_cancel_does_not_re_clear_timer() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(313);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);

    // Cancel removes the pending timer
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);

    // Kill after cancel: no error, no new timer operations
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
    Ok(())
}

/// Test cancel_kill_alternating: alternating cancel/kill/cancel/kill on the
/// same run must remain a typed no-op after the first terminalization,
/// with exactly one journal event emitted (the first terminalization).
#[test]
fn cancel_kill_alternating_keeps_terminalization_idempotent() -> Result<(), RuntimeError> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(314);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First terminalization: Cancel
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let baseline_cancelled = shard.counters().snapshot().runs_cancelled;
    assert_eq!(baseline_cancelled, 1);

    // Alternate: Kill, Cancel, Kill, Cancel — all no-ops
    for _ in 0..2 {
        assert_eq!(
            shard.enqueue(ShardCommand::Kill { run, reason: None }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run, reason: None }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    // Counter still at 1, not 5 (RQ-W0-17: cancel and kill use distinct counters)
    assert_eq!(shard.counters().snapshot().runs_cancelled, baseline_cancelled);
    assert_eq!(shard.counters().snapshot().runs_killed, 0);

    // Exactly one RunCancelled journal event, zero RunKilled
    let events = journal
        .snapshot()
        .map_err(|_| RuntimeError::QueueFull)?;
    let cancelled_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    let killed_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancelled_count, 1, "events: {events:?}");
    assert_eq!(killed_count, 0, "events: {events:?}");
    Ok(())
}

/// Test cancel_after_kill_releases_frame_only_once: Frame is released exactly
/// once across the cross-case (the first terminalization wins).
#[test]
fn cancel_after_kill_releases_frame_only_once() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(315);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Frame allocated
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(0)
    );

    // Kill releases frame
    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(1)
    );

    // Cancel after kill: frame availability stays at 1 (no double-release)
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(1)
    );
    Ok(())
}
