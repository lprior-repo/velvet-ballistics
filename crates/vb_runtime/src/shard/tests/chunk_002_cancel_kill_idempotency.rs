// Tests for cancel/kill idempotency (PO-vb-pymh-011).
//
// These tests verify that cancel and kill operations are idempotent:
// - cancel_idempotent_property: Calling cancel twice returns Ok both times
// - kill_idempotent_property: Calling kill twice returns Ok both times

// suspended_workflow() and small_config() are defined in earlier chunks

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
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
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
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
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
