
#[test]
fn shard_action_failure_non_retryable_with_handler_routes_to_handler() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = action_with_error_handler_workflow() else {
        return;
    };
    let run = super::RunId::new(743);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Fail the action at step 1 (inside the error handler body)
    let ticket = action_ticket(run, vb_core::ids::StepIdx::new(1));
    let failure = vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
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
    // The error handler at step 2 runs and the workflow finishes successfully
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

// ---------------------------------------------------------------------------
// handle_action_failure: failure with wrong run in ticket
// ---------------------------------------------------------------------------

#[test]
fn shard_action_failure_with_wrong_run_in_ticket_returns_run_not_found() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
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
    assert_eq!(shard.tick(), Ok(true));

    // Fail an action with a ticket that references a different run
    let ticket = action_ticket(super::RunId::new(999), vb_core::ids::StepIdx::ZERO);
    let failure = timeout_failure();
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// ---------------------------------------------------------------------------
// handle_ask_answer: valid answer completes the ask workflow
// ---------------------------------------------------------------------------

#[test]
fn shard_ask_answer_completes_ask_workflow() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = ask_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(750);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run is now waiting on an ask with a pending timer
    assert_eq!(shard.pending_timers.len(), 1);

    // When answering the ask
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::new(2),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: vb_core::value::SlotValue::I64(99),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Then the run completes
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    // Pending timer was cleaned up by the answer
    assert_eq!(shard.pending_timers.len(), 0);
}

#[test]
fn shard_ask_answer_produces_ask_answered_trace_event() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = ask_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(751);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
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
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let events = shard.trace_ring_mut().drain();
    let found_ask_answered = events.iter().any(|e| {
        *e == TraceEvent::AskAnswered {
            run,
            step: vb_core::ids::StepIdx::new(2),
            slot: SlotIdx::new(2),
        }
    });
    assert_eq!(found_ask_answered, true);
}

#[test]
fn shard_ask_answer_for_wrong_ask_step_returns_run_not_found() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = ask_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(752);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Answer with a wrong ask_step that doesn't match the suspended state
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::new(99),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// ---------------------------------------------------------------------------
// handle_timer: wait timer fires and completes
// ---------------------------------------------------------------------------

#[test]
fn shard_timer_fire_for_wait_produces_wait_resolved_journal() {
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(760);
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

    assert_eq!(timer_command(&shard, run).map(|command| shard.enqueue(command)), Some(Ok(())));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // Journal should contain WaitResolved
    assert!(
        matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::WaitResolved { run, step: vb_core::ids::StepIdx::new(1) }))
    );
}

#[test]
fn shard_timer_fire_for_ask_timeout_fails_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_ask_without_answer_workflow() else {
        return;
    };
    let run = super::RunId::new(761);
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

    assert_eq!(timer_command(&shard, run).map(|command| shard.enqueue(command)), Some(Ok(())));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
}

// ---------------------------------------------------------------------------
// handle_cancel: cancel cleans up pending ask timer
// ---------------------------------------------------------------------------

#[test]
fn shard_cancel_removes_pending_ask_timer() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_ask_without_answer_workflow() else {
        return;
    };
    let run = super::RunId::new(770);
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
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}
