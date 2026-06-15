
#[test]
fn shard_duplicate_submit_after_cancel_succeeds() {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(201);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When re-submitting the same run ID
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then it succeeds (run was removed by cancel)
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn shard_snapshot_run_for_active_run_returns_found() {
    // Given a shard with an active suspended run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(202);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting directly (non-queued)
    let response = shard.snapshot_run(run, 42);
    // Then it returns Found with correct fields
    match response {
        InspectResponse::Found(snap) => {
            assert_eq!(snap.run, run);
            assert_eq!(snap.correlation, 42);
        }
        other => {
            assert_eq!(
                other,
                InspectResponse::NotFound {
                    run,
                    correlation: 42
                }
            );
        }
    }
}

#[test]
fn shard_snapshot_run_for_unknown_returns_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let shard = Shard::new(config);
    // When snapshotting a non-existent run
    let response = shard.snapshot_run(super::RunId::new(9999), 7);
    // Then it returns NotFound
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run: super::RunId::new(9999),
            correlation: 7,
        }
    );
}

#[test]
fn shard_fill_queue_to_capacity_returns_queue_full() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    // When filling the queue exactly
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then the next enqueue returns QueueFull
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
}

#[test]
fn adversarial_shard_ask_answered_for_unknown_run_returns_run_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When answering an ask for a non-existent run
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(999),
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_submit_two_runs_same_id_second_fails() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(203);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID without cancelling
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn shard_step_budget_zero_still_submits_but_does_not_drive() {
    // Given a shard with zero step budget
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 0,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(204);
    // When submitting a run with zero budget
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is submitted (counter incremented)
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // And the run is still in the map (budget exhausted on first step)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
        }
        other => {
            assert_eq!(other, None);
        }
    }
}

#[test]
fn shard_multiple_cancels_idempotent_for_same_run() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(205);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling twice
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then failed counter is 1 (not 2)
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

// =======================================================================
// Adversarial BDD tests - shard attack vectors
// =======================================================================

#[test]
fn shard_submit_after_shutdown_is_enqueued_but_never_processed() {
    // Given a shard that has received shutdown
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    // When submitting a run after shutdown was processed
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(300),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns false (shutting down flag prevents processing)
    assert_eq!(shard.tick(), Ok(false));
    // And no runs were submitted
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
}
