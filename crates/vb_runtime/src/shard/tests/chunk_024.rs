
// =========================================================================
// vb-1u88: Graceful Shutdown and Cancellation Edges — RED PHASE TESTS
// These tests compile but fail assertions until implementation matches spec.
// =========================================================================

// ---------------------------------------------------------------------------
// Section 2: Tick and Shutdown — Postconditions
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_tick_multiple_times_after_shutdown_all_false() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
}

#[test]
fn vb1u88_drain_for_shutdown_processes_submit_then_shutdown() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert_eq!(shard.is_shutting_down(), true);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn vb1u88_drain_for_shutdown_on_already_shutting_down() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
}

#[test]
fn vb1u88_drain_for_shutdown_empty_queue_returns_shutdown_in_progress() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.drain_for_shutdown(),
        Err(RuntimeError::ShutdownInProgress)
    );
}

// ---------------------------------------------------------------------------
// Section 3: Handle Cancel — Postconditions
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_cancel_unknown_run_returns_ok() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(9999),
        reason: None}),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn vb1u88_cancel_emits_run_cancelled_journal_event() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(1001);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let events = journal.snapshot().expect("journal snapshot should succeed");
    assert!(
        events.contains(&RuntimeJournalEvent::RunCancelled { run, reason: None}),
        "journal should contain RunCancelled event"
    );
}

#[test]
fn vb1u88_cancel_emits_run_cancelled_trace_event() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(1002);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let events = shard.trace_ring_mut().drain();
    assert!(
        events.contains(&TraceEvent::RunCancelled { run }),
        "trace should contain RunCancelled event"
    );
}

#[test]
fn vb1u88_cancel_unknown_run_does_not_emit_events() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let before = journal.snapshot().expect("journal snapshot should succeed");
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(8888),
        reason: None}),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    let after = journal.snapshot().expect("journal snapshot should succeed");
    assert_eq!(
        before, after,
        "no journal events should be emitted for unknown run cancel"
    );
    let trace_events = shard.trace_ring_mut().drain();
    let unknown_run = super::RunId::new(8888);
    assert!(
        !trace_events
            .iter()
            .any(|e| matches!(e, TraceEvent::RunCancelled { run } if *run == unknown_run)),
        "no trace RunCancelled event for unknown run"
    );
}

#[test]
fn vb1u88_cancel_removes_run_and_releases_frame() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(1003);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(0)
    );
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.run_state_get(run), None);
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(|p| p.available()),
        Some(1)
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn vb1u88_cancel_removes_pending_timer() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(1004);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 0);
}

// ---------------------------------------------------------------------------
// Section 4: Status and Health Reporting
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_status_running_when_not_shutting_down() {
    let config = small_config();
    let shard = Shard::new(config);
    let status = shard.status();
    assert_eq!(status.health, super::ShardHealth::Running);
    assert_eq!(status.running, true);
    assert_eq!(status.shutting_down, false);
}

#[test]
fn vb1u88_status_shutting_down_after_shutdown_tick() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    let status = shard.status();
    assert_eq!(status.health, super::ShardHealth::ShuttingDown);
    assert_eq!(status.running, false);
    assert_eq!(status.shutting_down, true);
}

#[test]
fn vb1u88_status_command_queue_depth_correct() {
    let config = small_config();
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let status = shard.status();
    assert_eq!(status.command_queue_depth, 2);
    assert_eq!(status.command_queue_capacity, 16);
}

#[test]
fn vb1u88_status_immutable_during_shutdown() {
    let config = small_config();
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let status1 = shard.status();
    let status2 = shard.status();
    assert_eq!(status1.command_queue_depth, status2.command_queue_depth);
    assert_eq!(status1.active_runs, status2.active_runs);
}

#[test]
fn vb1u88_is_shutting_down_false_on_new_shard() {
    let config = small_config();
    let shard = Shard::new(config);
    assert_eq!(shard.is_shutting_down(), false);
}

#[test]
fn vb1u88_is_shutting_down_true_after_shutdown() {
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
}
