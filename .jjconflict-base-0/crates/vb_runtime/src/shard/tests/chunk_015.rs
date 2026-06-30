#[test]
fn shard_action_failure_non_retryable_with_handler_routes_to_handler() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(
        action_with_error_handler_workflow(),
        "action_with_error_handler",
    )?;
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
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_action_failure: failure with wrong run in ticket
// ---------------------------------------------------------------------------

#[test]
fn shard_action_failure_with_wrong_run_in_ticket_returns_run_not_found() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(suspended_workflow(), "suspended_workflow")?;
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
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_ask_answer: valid answer completes the ask workflow
// ---------------------------------------------------------------------------

#[test]
fn shard_ask_answer_completes_ask_workflow() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(ask_then_finish_workflow(), "ask_then_finish_workflow")?;
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
    Ok(())
}

#[test]
fn shard_ask_answer_produces_ask_answered_trace_event() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(ask_then_finish_workflow(), "ask_then_finish_workflow")?;
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
    Ok(())
}

#[test]
fn shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(ask_then_finish_workflow(), "ask_then_finish_workflow")?;
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

    // Answer with a wrong ask_step that doesn't match the suspended state.
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
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_timer: wait timer fires and completes
// ---------------------------------------------------------------------------

#[test]
fn shard_timer_fire_for_wait_produces_wait_resolved_journal() -> Result<(), String> {
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);
    let workflow = workflow_fixture(timed_wait_then_finish_workflow(), "timed_wait_then_finish")?;
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

    assert_eq!(
        timer_command(&shard, run).map(|command| shard.enqueue(command)),
        Some(Ok(()))
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // Journal should contain WaitResolved
    assert!(
        matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::WaitResolved { run, step: vb_core::ids::StepIdx::new(1) }))
    );
    Ok(())
}

#[test]
fn shard_timer_fire_for_ask_timeout_fails_run() -> Result<(), String> {
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);
    let workflow = timed_ask_without_answer_workflow()
        .ok_or_else(|| "timed ask workflow fixture must build".to_owned())?;
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

    assert_eq!(
        timer_command(&shard, run).map(|command| shard.enqueue(command)),
        Some(Ok(()))
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    assert!(
        matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::AskTimedOut { run, step: vb_core::ids::StepIdx::new(2) }))
    );
    Ok(())
}

#[derive(Debug)]
struct CancelAppendFailsJournal;

impl crate::journal::RuntimeJournal for CancelAppendFailsJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.append_sequenced(event, vb_storage::EventSeq::ZERO)
    }

    fn append_sequenced(
        &self,
        event: RuntimeJournalEvent,
        _seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        if matches!(event, RuntimeJournalEvent::RunCancelled { .. }) {
            return Err(RuntimeError::JournalPoisoned);
        }
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn shard_cancel_append_failure_preserves_run_state_and_pending_timer() -> Result<(), String> {
    let shared: SharedRuntimeJournal = std::sync::Arc::new(CancelAppendFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let workflow = timed_ask_without_answer_workflow()
        .ok_or_else(|| "timed ask workflow fixture must build".to_owned())?;
    let run = super::RunId::new(762);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_contains(run), true);
    assert_eq!(shard.run_state_contains(run), true);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::JournalPoisoned));

    assert_eq!(shard.pending_timer_contains(run), true);
    assert_eq!(shard.run_state_contains(run), true);
    assert_eq!(shard.terminal_runs_contains(run), false);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_cancel: cancel cleans up pending ask timer
// ---------------------------------------------------------------------------

#[test]
fn shard_cancel_removes_pending_ask_timer() -> Result<(), String> {
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(
        timed_ask_without_answer_workflow(),
        "timed_ask_without_answer",
    )?;
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

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}
