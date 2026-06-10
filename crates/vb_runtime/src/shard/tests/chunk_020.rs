
/// Frame pool metrics reflect submissions and completions.
#[test]
fn shard_frame_pool_metrics_after_submit_and_finish() {
    let config = small_config();
    let mut shard = Shard::new(config);

    // Initially no pools
    let (free, total) = shard.frame_pool_metrics();
    assert_eq!(free, 0);
    assert_eq!(total, 0);

    // Submit a finished workflow -> pool created and frame returned
    let Some(wf) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(980),
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    let (free_after, total_after) = shard.frame_pool_metrics();
    assert!(
        free_after >= 1,
        "expected at least 1 free frame, got {free_after}"
    );
    assert!(
        total_after >= 1,
        "expected at least 1 total capacity, got {total_after}"
    );
}

/// Verify that snapshot_run returns Terminal { Completed } after a run finishes
/// via error handler routing.
#[test]
fn shard_snapshot_after_error_handler_finish() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = action_with_error_handler_workflow() else {
        return;
    };
    let run = RunId::new(990);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Fail the action to route to handler, which then completes
    let ticket = action_ticket(run, vb_core::ids::StepIdx::new(1));
    let failure = vb_core::action::ActionFailure {
        code: vb_core::ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // Snapshot should return Terminal { Completed }
    let response = shard.snapshot_run(run, 1);
    assert_eq!(
        response,
        InspectResponse::Terminal {
            run,
            correlation: 1,
            outcome: TerminalOutcome::Completed,
        }
    );
}

/// Capacity boundary: submit, cancel, then new submit in same tick sequence.
#[test]
fn shard_capacity_one_submit_cancel_submit_sequence() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 8,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);

    // Submit + tick -> suspended
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let run1 = RunId::new(1000);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run1,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Cancel + tick
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run: run1, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // New submit should succeed (capacity freed)
    let Some(wf2) = finished_workflow() else {
        return;
    };
    let run2 = RunId::new(1001);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run2,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
}

/// Verify that PendingTimer fields are correct after timed wait submission.
#[test]
fn shard_pending_timer_fields_are_correct() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = RunId::new(1010);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let timer = shard.pending_timer_get(run);
    match timer {
        Some(t) => {
            assert_eq!(t.step, vb_core::ids::StepIdx::new(1)); // WaitUntil is at step 1
            assert_eq!(t.kind, crate::shard::types::PendingTimerKind::Wait);
        }
        None => assert!(false, "expected pending timer"),
    }
}

/// AskAnswer with I64 value completes the ask workflow correctly.
#[test]
fn shard_ask_answered_with_i64_value() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = ask_then_finish_workflow() else {
        return;
    };
    let run = RunId::new(1020);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::new(2),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: vb_core::value::SlotValue::I64(12345),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Run should complete
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(shard.pending_timers.len(), 0);
}

/// ShardConfig::new at the max command queue capacity boundary succeeds.
#[test]
fn shard_config_new_at_max_capacity_boundary() {
    let result = ShardConfig::new(
        MAX_COMMAND_QUEUE_CAPACITY,
        16,
        100,
        4,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(
        result,
        Ok(ShardConfig {
            command_queue_capacity: MAX_COMMAND_QUEUE_CAPACITY,
            trace_capacity: 16,
            step_budget_per_tick: 100,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        })
    );
    assert_eq!(
        result.map(|config| config.command_queue_capacity),
        Ok(MAX_COMMAND_QUEUE_CAPACITY)
    );
}

/// ShardConfig::new at the minimum valid capacity (1) succeeds.
#[test]
fn shard_config_new_at_minimum_capacity() {
    let result = ShardConfig::new(1, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Ok(ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        })
    );
}
