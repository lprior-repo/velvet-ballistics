
#[test]
fn shard_timer_rejects_run_without_pending_timer() -> Result<(), RuntimeError> {
    // Given a shard with an action-suspended run, not a timed wait/ask
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(60);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When timer fires for the run
    assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
    // Then tick rejects it because no timer was registered
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn shard_wait_suspension_registers_pending_timer() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(61);

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
    assert_eq!(
        shard.pending_timer_get(run).map(|timer| timer.step),
        Some(vb_core::ids::StepIdx::new(1))
    );
    Ok(())
}

#[test]
fn shard_timer_fired_advances_timed_wait_to_finish() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(62);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(timer_command(&shard, run).map(|command| shard.enqueue(command)), Some(Ok(())));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_timer_returns_error_for_unknown_run() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When timer fires for a non-existent run
    assert_eq!(
        shard.enqueue(invalid_timer_command(super::RunId::new(777))),
        Ok(())
    );
    // Then tick rejects missing timer authority.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn shard_cancel_removes_run_from_runs_map() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then inspect returns NotFound (run removed from map)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 5,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::Terminal {
            run,
            correlation: 5,
            outcome: TerminalOutcome::Cancelled,
        })
    );
    Ok(())
}

#[test]
fn shard_cancel_records_run_cancelled_trace_event() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(71);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains a RunCancelled event
    let events = shard.trace_ring_mut().drain();
    let found = events
        .iter()
        .any(|e| *e == TraceEvent::RunCancelled { run });
    assert_eq!(found, true);
    Ok(())
}

#[test]
fn shard_cancel_emits_cancelled_journal_and_preserves_counter_semantics() -> Result<(), RuntimeError> {
    // Given a shard with a volatile journal and an active suspended run
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(73);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // When cancelling the active run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Then cancellation is a distinct journal/trace event, while the legacy failed counter
    // still counts the non-successful terminal lifecycle.
    assert!(
        matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::RunCancelled { run, reason: None}))
    );
    assert!(
        shard
            .trace_ring_mut()
            .drain()
            .contains(&TraceEvent::RunCancelled { run })
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn shard_cancel_increments_failed_counter() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(72);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then the failed counter is incremented
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn shard_inspect_captures_current_pc() -> Result<(), RuntimeError> {
    // Given a shard with an active suspended run at step 0
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When inspecting the run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 10,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the snapshot pc matches the expected program counter
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.pc, vb_core::ids::StepIdx::new(0));
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 10);
        }
        other => assert_eq!(other, None),
    }
    Ok(())
}
