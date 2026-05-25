
// ---------------------------------------------------------------------------
// handle_action_completion: full ActionCompleted (not legacy)
// ---------------------------------------------------------------------------

#[test]
fn shard_action_completed_full_writes_slot_and_advances() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(730);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Complete the action using the full ActionCompleted command (not legacy).
    let ticket = action_ticket(run, vb_core::ids::StepIdx::ZERO);
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // The trace ring should contain ActionCompleted and SlotWritten
    let events = shard.trace_ring_mut().drain();
    let found_action = events.iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }
    });
    let found_slot = events.iter().any(|e| {
        matches!(e,
            TraceEvent::SlotWritten { run: r, slot, .. }
            if *r == run && *slot == SlotIdx::new(0)
        )
    });
    assert_eq!(found_action, true);
    assert_eq!(found_slot, true);
}

#[test]
fn shard_action_completed_full_with_wrong_step_returns_invalid_completion() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(731);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Complete with wrong step index (step 99 does not exist or is not running)
    let ticket = vb_core::action::ActionTicket {
        run,
        step: vb_core::ids::StepIdx::new(99),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(1),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 8,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
}

// ---------------------------------------------------------------------------
// handle_action_failure: retryable failure triggers retry
// ---------------------------------------------------------------------------

#[test]
fn shard_action_failure_retryable_with_retry_check_retries_action() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = do_with_retry_workflow() else {
        return;
    };
    let run = super::RunId::new(740);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run is suspended on the Do action at step 1

    // When failing with a retryable failure and retry metadata exists
    let ticket = action_ticket(run, vb_core::ids::StepIdx::new(1));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Then the run re-enters suspension on the same Do action (retry)
    let events = shard.trace_ring_mut().drain();
    let found_action_failed = events.iter().any(|e| {
        *e == TraceEvent::ActionFailed {
            run,
            step: vb_core::ids::StepIdx::new(1),
            code: ActionFailureCode::Timeout,
        }
    });
    assert_eq!(found_action_failed, true);

    // The run is still in the runs map (re-suspended on Do)
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
        other => assert_eq!(other, None),
    }
}

#[test]
fn shard_action_failure_retryable_exhaustion_fails_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = do_with_retry_workflow() else {
        return;
    };
    let run = super::RunId::new(741);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First retryable failure: retries (attempt counter goes to 2)
    let ticket1 = action_ticket(run, vb_core::ids::StepIdx::new(1));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Second retryable failure: retries (attempt counter goes to 2, then tries add to 2, max=2, returns false => exhausts)
    let ticket2 = vb_core::action::ActionTicket {
        attempt: 2,
        capacity: 2,
        ..action_ticket(run, vb_core::ids::StepIdx::new(1))
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // The retry policy max_attempts is 2, so after recording attempt 2 the policy is exhausted.
    // With no error handler, the run should fail.
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
}

#[test]
fn shard_action_failure_non_retryable_without_handler_fails_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(742);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let ticket = action_ticket(run, vb_core::ids::StepIdx::ZERO);
    let failure = vb_core::action::ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: vb_core::action::RetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}
