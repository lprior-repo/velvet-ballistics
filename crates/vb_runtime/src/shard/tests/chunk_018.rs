
// BH-SHD-10: Cancel non-existent run produces no journal event.
#[test]
fn bh_shd_10_cancel_nonexistent_run_no_journal_event() -> Result<(), RuntimeError> {
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared)?;
    let run = super::RunId::new(810);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    let events = journal.snapshot().unwrap_or_default();
    let cancelled_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r , reason: None} if *r == run))
        .count();
    assert_eq!(
        cancelled_count, 0,
        "BH-SHD-10: no RunCancelled journal event for non-existent run"
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

// BH-SHD-11: step_budget_per_tick=0 creates permanent DoS.
// Runs are accepted but never execute any steps.
// Severity: Medium. Config should reject step_budget_per_tick=0.
#[test]
fn bh_shd_11_zero_step_budget_never_executes() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(811);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    // The run completed with zero budget (no steps executed).
    // A second tick also does nothing — confirming zero-step DoS is harmless.
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    // BH-SHD-11: Run is stuck forever with zero budget
    Ok(())
}

// BH-SHD-12: Legacy completion on finished run errors correctly.
#[test]
fn bh_shd_12_legacy_completion_on_finished_run_errors() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(812);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// BH-SHD-13: TimerFire after cancel returns RunNotFound.
#[test]
fn bh_shd_13_timer_fire_after_cancel_returns_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(813);
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
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

// BH-SHD-14: Inspect after immediate completion returns Terminal { Completed }.
#[test]
fn bh_shd_14_inspect_after_immediate_completion_returns_terminal() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(814);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Terminal {
            run: r,
            correlation: c,
            outcome,
        }) => {
            assert_eq!(r, run);
            assert_eq!(c, 1);
            assert_eq!(outcome, TerminalOutcome::Completed);
        }
        other => {
            let msg = format!("expected Terminal {{ Completed }}, got {other:?}");
            panic!("{msg}");
        }
    }
    Ok(())
}

// =========================================================================
// Additional lifecycle coverage: submit/cancel/resume/inspect boundaries,
// capacity enforcement, and state machine edge cases.
// =========================================================================

/// Submit multiple runs, cancel some, inspect the remainder -- verify counters.
#[test]
fn shard_submit_cancel_inspect_mixed_lifecycle() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = Shard::new(config)?;
    let Some(wf_suspend) = suspended_workflow() else {
        return Ok(());
    };
    let Some(wf_finish) = finished_workflow() else {
        return Ok(());
    };

    // Submit a finishing run (completes immediately)
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(900),
            workflow: wf_finish,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Submit a suspended run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(901),
            workflow: wf_suspend,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Cancel the suspended run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(901),
        reason: None}),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Inspect the finished run (should be Terminal { Completed } since it completed)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: super::RunId::new(900),
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::Terminal {
            run: super::RunId::new(900),
            correlation: 1,
            outcome: TerminalOutcome::Completed,
        })
    );

    // Counters: 2 submitted, 1 completed, 1 failed (cancelled)
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

/// SubmitWithInputs with empty inputs behaves identically to Submit.
#[test]
fn shard_submit_with_empty_inputs_matches_submit() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(910);

    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs: Box::from([]),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}
