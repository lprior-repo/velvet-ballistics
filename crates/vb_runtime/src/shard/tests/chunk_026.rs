
#[test]
fn vb1u88_queue_full_at_capacity_boundary() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
};
    let shard = Shard::new(config)?;
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
    Ok(())
}

#[test]
fn vb1u88_action_ticket_step_idx_boundary() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(9001);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let ticket_min = action_ticket(run, vb_core::ids::StepIdx::ZERO);
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::I64(1),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ticket_min,
            output
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

// ---------------------------------------------------------------------------
// Section 8: Integration Tests — BDD Given-When-Then
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_bdd_clean_shutdown_sequence() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let result = shard.tick();
    assert_eq!(result, Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
    assert_eq!(shard.status().health, super::ShardHealth::ShuttingDown);
    Ok(())
}

#[test]
fn vb1u88_bdd_cancel_non_existent_run_is_idempotent() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let before_failed = shard.counters().snapshot().runs_failed;
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(9999),
        reason: None}),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    assert_eq!(shard.counters().snapshot().runs_failed, before_failed);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn vb1u88_bdd_multiple_ticks_after_shutdown_idempotent() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}

#[test]
fn vb1u88_bdd_cancel_run_removes_from_runs_emits_events() -> Result<(), RuntimeError> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(5001);
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
        "RunCancelled journal event should be present"
    );
    assert_eq!(shard.run_state_get(run), None);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

// =========================================================================
// vb-p5so: Forcefully clear pending suspended timers on drain_for_shutdown
// RED PHASE — These tests compile but fail until pending_timers.clear()
// is added to drain_for_shutdown().
// =========================================================================

#[test]
fn test_drain_for_shutdown_removes_all_pending_timers_and_returns_them() -> Result<(), RuntimeError> {
    // Given: a shard with a run that has a pending Wait timer
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(9001);
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

    // When: drain_for_shutdown processes Shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: pending timers are cleared and shard is shutting down
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}

#[test]
fn test_shutdown_is_processed_successfully_even_when_timer_queue_is_full() -> Result<(), RuntimeError> {
    // Given: a shard with pending timers and a full command queue (no Shutdown)
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(9002);
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

    // Fill the command queue with Inspect commands (no Shutdown)
    for i in 0..config.command_queue_capacity {
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: super::RunId::new(9999),
                correlation: i as u64,
            }),
            Ok(())
        );
    }

    // When: drain_for_shutdown hits capacity before seeing Shutdown
    assert_eq!(
        shard.drain_for_shutdown(),
        Err(RuntimeError::ShutdownInProgress)
    );

    // Then: pending timers are unchanged
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.is_shutting_down(), false);
    Ok(())
}

#[test]
fn test_calling_drain_for_shutdown_repeatedly_is_idempotent() -> Result<(), RuntimeError> {
    // Given: a shard that has already shut down
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert_eq!(shard.is_shutting_down(), true);
    assert_eq!(shard.pending_timers.len(), 0);

    // When: drain_for_shutdown is called again
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: state remains unchanged
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}

#[test]
fn test_drain_for_shutdown_handles_empty_timer_state() -> Result<(), RuntimeError> {
    // Given: a shard with no pending timers
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.pending_timers.len(), 0);

    // When: drain_for_shutdown processes Shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: timers remain empty and shard is shutting down
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}

#[test]
fn test_drain_for_shutdown_handles_timers_without_valid_backing_runs_gracefully() -> Result<(), RuntimeError> {
    // Given: a shard with an orphaned pending timer entry (no corresponding run)
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let orphaned_run = super::RunId::new(9003);
    shard.pending_timer_insert(
        orphaned_run,
        PendingTimer {
            step: vb_core::ids::StepIdx::new(1),
            kind: PendingTimerKind::Wait,
            generation: 1,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.pending_timers.len(), 1);

    // When: drain_for_shutdown processes Shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: orphaned timer is cleared without panic
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}

// =========================================================================
// RQ-W0-12: drain_for_shutdown must journal WaitCancelled/AskCancelled
// before clearing the in-memory timer map. Previously the public
// drain_pending_and_shutdown path bypassed the journaling helper.
// =========================================================================

#[test]
fn test_drain_for_shutdown_journals_wait_cancellation_events() -> Result<(), RuntimeError> {
    // Given: a shard with a run that has a pending Wait timer and a journal
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let config = small_config();
    let mut shard = Shard::new_with_journal(config, shared)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(9101);
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

    // When: drain_for_shutdown is invoked
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: the journal contains a WaitCancelled event for the run
    let events = journal.snapshot().expect("journal snapshot should succeed");
    let wait_cancelled_found = events
        .iter()
        .any(|event| matches!(event, RuntimeJournalEvent::WaitCancelled { run: r, .. } if *r == run));
    assert!(
        wait_cancelled_found,
        "durable WaitCancelled event must be present after drain_for_shutdown"
    );
    Ok(())
}

#[test]
fn test_drain_pending_and_shutdown_journals_timer_cancellations() -> Result<(), RuntimeError> {
    // Given: a shard with a pending Wait timer and a journal
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let config = small_config();
    let mut shard = Shard::new_with_journal(config, shared)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(9102);
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

    // When: drain_pending_and_shutdown is invoked (RQ-W0-12: previously
    // bypassed the cancellation journal)
    assert_eq!(shard.drain_pending_and_shutdown(), Ok(()));

    // Then: pending timers are cleared
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);

    // And: the durable journal contains a WaitCancelled event
    let events = journal.snapshot().expect("journal snapshot should succeed");
    let wait_cancelled_found = events
        .iter()
        .any(|event| matches!(event, RuntimeJournalEvent::WaitCancelled { run: r, .. } if *r == run));
    assert!(
        wait_cancelled_found,
        "durable WaitCancelled event must be journaled by drain_pending_and_shutdown"
    );
    Ok(())
}
