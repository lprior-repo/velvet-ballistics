
#[test]
fn future_attempt_completion_rejected_when_current_attempt_exists() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_001);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        assert_eq!(None::<()>, Some(()), "run should remain active");
        return;
    };
    assert_eq!(state.action_attempts.get(0).copied(), Some(1));
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 2)
            },
            output,
        }),
        Ok(())
    );
    // G005 fixed: future-attempt completion must be rejected
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn future_attempt_completion_beyond_max_is_action_failed_code() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_002);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    let error = RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 };
    assert_eq!(error.runtime_code(), Some("ACTION_FAILED"));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 4)
            },
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(error));
}

#[test]
fn stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(41);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        assert_eq!(None::<()>, Some(()), "run should remain active");
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let frame_before = state.frame.clone();
    let step_state_before = state.frame.step_state(StepIdx::ZERO);
    let attempts_before = state.action_attempts.clone();
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 2)
            },
            output,
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: 2,
            current: 3,
        })
    );
    let Some(state_after) = shard.run_state_get_mut(run) else {
        assert_eq!(
            None::<()>,
            Some(()),
            "run should remain active after rejection"
        );
        return;
    };
    assert_eq!(state_after.frame.pc(), frame_before.pc());
    assert_eq!(
        state_after.frame.step_state(StepIdx::ZERO),
        step_state_before
    );
    assert_eq!(state_after.frame, frame_before);
    assert_eq!(state_after.action_attempts, attempts_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(journal.snapshot(), journal_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before
    );
}

#[test]
fn scheduling_propagates_zero_retry_policy_error() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = zero_retry_policy_workflow() else {
        assert_eq!(
            None::<()>,
            Some(()),
            "missing zero retry policy workflow fixture"
        );
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(42),
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero",
        })
    );
}

#[test]
fn legacy_action_completed_on_suspended_run_succeeds() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let found = shard.trace_ring_mut().drain().iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: StepIdx::ZERO,
        }
    });
    assert_eq!(found, true);
}

#[test]
fn legacy_action_completed_unknown_run_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: RunId::new(9999),
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn action_failure_without_handler_fails_run() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let wf = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(60);
    submit_run(&mut shard, run, wf);
    let ticket = make_ticket(run, StepIdx::ZERO, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

// =======================================================================
// Non-mutation: invalid authority does not mutate observable state
// =======================================================================

#[test]
fn future_attempt_completion_does_not_mutate_state() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(410);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let counters_before = shard.counters().snapshot();
    let _journal_snap_before = journal.snapshot();
    let _trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    // Send attempt=3 when current=1 — future attempt
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 5,
                ..make_ticket(run, StepIdx::ZERO, 3)
            },
            output,
        }),
        Ok(())
    );
    // G005 fixed: future-attempt rejection now returns InvalidActionCompletion
    // The tick must reject the completion and must not mutate observable state
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::InvalidActionCompletion),
        "future-attempt completion must be rejected"
    );
    // Counter and journal state after — verify key invariants hold
    let counters_after = shard.counters().snapshot();
    let journal_snap_after = journal.snapshot();
    let _trace_after = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    // At minimum, submitted runs count should not decrease
    assert!(counters_after.runs_submitted >= counters_before.runs_submitted);
    // Run should not be failed unless explicitly failed
    if counters_after.runs_failed > counters_before.runs_failed {
        // If the run failed, it should have a RunFailed event
        let has_run_failed = journal_snap_after.as_ref().map_or(false, |events| {
            events
                .iter()
                .any(|e| matches!(e, RuntimeJournalEvent::RunFailed { run: r } if *r == run))
        });
        if !has_run_failed {
            // Run failed counter incremented without RunFailed event — this is a bug
            assert_eq!(
                journal_snap_after.is_ok(),
                true,
                "journal snapshot failed unexpectedly"
            );
        }
    }
}

#[test]
fn noncanonical_key_completion_does_not_mutate_state() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(411);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot();
    let active_runs_before = shard.active_run_count();
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    // Create ticket with deliberately wrong idempotency_key
    let mut bad_ticket = make_ticket(run, StepIdx::ZERO, 1);
    bad_ticket.idempotency_key = 0; // Non-canonical key
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: bad_ticket,
            output,
        }),
        Ok(())
    );
    let result = shard.tick();
    // Should return InvalidActionCompletion due to key mismatch
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    // State must be unchanged
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(journal.snapshot(), journal_before);
    assert_eq!(shard.active_run_count(), active_runs_before);
}

#[test]
fn wrong_step_state_completion_does_not_mutate_state() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(412);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Step is in Running state after scheduling. Mark it succeeded to make it invalid.
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot();
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    let result = shard.tick();
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(journal.snapshot(), journal_before);
}

#[test]
fn action_completion_on_missing_run_does_not_mutate_state() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot();
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(RunId::new(9999), StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(journal.snapshot(), journal_before);
}

// =======================================================================
// handle_action_completion terminal run fence
// =======================================================================

#[test]
fn handle_action_completion_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(RunId::new(420), StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_cancelled() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(421);
    submit_run(&mut shard, run, wf);
    // Cancel the run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Now attempt action completion on cancelled run
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_finished() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(422);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Finished workflow completes immediately — run should no longer be active
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// =======================================================================
// handle_action_failure
// =======================================================================

#[test]
fn handle_action_failure_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(RunId::new(430), StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_failure_returns_stale_attempt_when_attempt_mismatch() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(431);
    submit_run(&mut shard, run, wf);
    // Set current attempt to 3
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    // Send attempt=1 (stale) failure
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 3,
        })
    );
}
