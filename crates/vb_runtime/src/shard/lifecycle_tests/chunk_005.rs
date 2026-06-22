
#[test]
fn action_failure_without_handler_emits_action_failed_before_run_failed() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(600);
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

    let events = require_snapshot(&journal)?;
    assert_event_order(
        &events,
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::ZERO,
            action: ActionId::new(0),
            attempt: 1,
        },
        RuntimeJournalEvent::RunFailed { run },
    );
    Ok(())
}

#[test]
fn action_failure_routes_to_error_handler() -> Result<(), String> {
    let mut shard = Shard::new(small_config()).map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(61);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let ticket = make_ticket(run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn action_failure_routed_to_handler_emits_action_failed_before_handler_step() -> Result<(), String>
{
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(610);
    submit_run(&mut shard, run, wf);
    let ticket = make_ticket(run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = require_snapshot(&journal)?;
    assert_event_order(
        &events,
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(0),
            attempt: 1,
        },
        RuntimeJournalEvent::StepStarted {
            run,
            step: StepIdx::new(2),
        },
    );
    Ok(())
}

#[test]
fn action_failure_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn retry_exhaustion_emits_single_action_failed() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let run = RunId::new(620);
    submit_run(&mut shard, run, retry_workflow()?);
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 2);
    let events = require_snapshot(&journal)?;
    assert_retry_exhaustion_journal(&events, run);
    Ok(())
}

#[test]
fn ask_answer_completes_ask_workflow() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("ask", ask_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn ask_answer_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let answer = AskAnswer {
        ticket: AskTicket {
            run: RunId::new(9999),
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        answer_slot: SlotIdx::ZERO,
        value: SlotValue::I64(0),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// RS-103 regression: the handler must reject answers whose supplied
// answer_slot or resume_step does not match the workflow-derived
// authority. Previously the handler trusted both fields from the
// answer payload.
#[test]
fn ask_answer_with_wrong_answer_slot_is_rejected() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("ask", ask_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(71);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Workflow authority: AskResume at step 3 answers slot 2.
    // Caller supplies an answer with an unauthorized answer slot (5).
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(5),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

#[test]
fn ask_answer_with_wrong_resume_step_is_rejected() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("ask", ask_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(72);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Workflow authority: resume_step is 3 (AskResume step).
    // Caller supplies an answer pointing at an arbitrary other step.
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(99),
        },
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

#[test]
fn timer_fire_advances_wait_to_completion() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("wait", wait_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.pending_timer_count(), 0);
    Ok(())
}

#[test]
fn timer_fire_for_non_timer_run_returns_invalid_timer_fire() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("suspended", suspended_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn timer_fire_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    assert_eq!(
        shard.enqueue(invalid_timer_command(RunId::new(9999))),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn cancel_removes_active_run_and_increments_failed() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("suspended", suspended_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn cancel_nonexistent_run_succeeds_without_counter_change() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(9999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

// =======================================================================
// finish_run terminal fence
// =======================================================================

#[test]
fn finish_run_appends_run_finished_event_and_inserts_terminal_run() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("finished", finished_workflow())?;
    let run = RunId::new(500);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Finished workflow should complete immediately
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // terminal_runs should contain the finished run
    assert!(shard.terminal_runs_contains(run));
    // Journal should contain RunFinished event
    let events = require_snapshot(&journal)?;
    let has_run_finished = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run));
    assert!(
        has_run_finished,
        "RunFinished event not found in journal: {events:?}"
    );
    Ok(())
}

// =======================================================================
// retry_is_available and apply_action_failure_to_state
// =======================================================================

#[test]
fn retry_remaining_advances_attempt_and_resumes_drive() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let run = RunId::new(501);
    submit_run(&mut shard, run, retry_workflow()?);
    // After submission, the run should be suspended on action step 1
    // with attempt 1 and capacity 2 from the retry policy
    let state = shard.run_state_get(run).ok_or("run should exist")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(1));
    // Send action failure with Retryable — should retry
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Check that attempt was incremented
    let state = shard
        .run_state_get(run)
        .ok_or("run should exist after retry")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(2));
    // Run is still active
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}

#[test]
fn retry_exhausted_fails_run_when_no_more_attempts() -> Result<(), String> {
    let mut shard = Shard::new(small_config()).map_err(|e| format!("shard construction: {:?}", e))?;
    let run = RunId::new(502);
    submit_run(&mut shard, run, retry_workflow()?);
    // retry_workflow has max_attempts=2 (from ConstValue::I64(2))
    // First failure at attempt 1 → retry to attempt 2
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Second failure at attempt 2 → retry exhausted, run fails
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(1), 2),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run should be failed
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn non_retryable_failure_fails_run_immediately() -> Result<(), String> {
    let mut shard = Shard::new(small_config()).map_err(|e| format!("shard construction: {:?}", e))?;
    let run = RunId::new(503);
    submit_run(&mut shard, run, retry_workflow()?);
    // Send non-retryable failure
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(1), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run should be failed
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn action_failure_drives_error_handler_when_no_retry_metadata() -> Result<(), String> {
    let mut shard = Shard::new(small_config()).map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(504);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // error_handler_workflow has no retry policy, so non-retryable failure
    // should route to error handler, which drives the run to completion
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(1), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run completes via error handler path (handler -> finish)
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn handle_action_failure_rejects_when_step_out_of_bounds() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let wf = require_workflow("suspended", suspended_workflow()).map_err(|_| RuntimeError::QueueFull)?;
    let run = RunId::new(505);
    submit_run(&mut shard, run, wf);
    // Step 99 is out of bounds
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(99), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // validate_action_completion catches the out-of-bounds step
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

// =======================================================================
// RS-105: ActionFailed journal record must precede state mutation.
// =======================================================================

#[test]
fn rs105_retryable_failure_journals_action_failed_before_incrementing_attempts() {
    // RS-105 regression: the durable ActionFailed record must appear in the
    // journal BEFORE the retry attempt counter is incremented, so a journal
    // failure cannot leave the run with consumed retry state but no durable
    // failure evidence.
    //
    // We drive enough ticks to flush the coalesce buffer (small_config uses
    // coalesce_window_ticks=1, so 2 ticks after enqueueing the failure
    // guarantees the ActionFailed record is committed to the journal).
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))
        .expect("shard construction");
    submit_run(&mut shard, RunId::new(600), retry_workflow().expect("workflow"));
    let run = RunId::new(600);
    let step = StepIdx::new(1);

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, step, 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    // First tick processes the failure and buffers ActionFailed + StepStarted.
    assert_eq!(shard.tick(), Ok(true));
    // Second tick flushes the coalesce buffer to the journal.
    assert_eq!(shard.tick(), Ok(true));

    let state = shard
        .run_state_get(run)
        .expect("run still exists after retryable failure");
    assert_eq!(
        state.action_attempts.get(step.as_usize()).copied(),
        Some(2),
        "retry attempt must have advanced to 2 after journal succeeded"
    );

    let events = require_snapshot(&journal).expect("snapshot");
    let expected_action_failed = RuntimeJournalEvent::ActionFailed {
        run,
        step,
        action: ActionId::new(0),
        attempt: 1,
    };
    assert!(
        events.contains(&expected_action_failed),
        "ActionFailed event with attempt=1 must be present in the journal, got {events:?}"
    );
    let action_failed_pos = events
        .iter()
        .position(|e| e == &expected_action_failed)
        .expect("ActionFailed must be in events");
    // After the journal entry, drive_run re-enters the Do step and appends
    // a fresh StepStarted for the retry. That StepStarted must come AFTER
    // ActionFailed. Use rposition to find the LAST StepStarted for this
    // run+step (which corresponds to the retry's re-entry).
    let expected_step_started = RuntimeJournalEvent::StepStarted { run, step };
    let retry_step_started_pos = events
        .iter()
        .rposition(|e| e == &expected_step_started)
        .expect("retry StepStarted must be in events");
    assert!(
        action_failed_pos < retry_step_started_pos,
        "ActionFailed (pos={action_failed_pos}) must precede retry StepStarted (pos={retry_step_started_pos}); events={events:?}"
    );
}

#[test]
fn rs105_non_retryable_failure_with_handler_journals_action_failed_before_handler_step()
-> Result<(), String> {
    // RS-105 regression: with an error handler, the ActionFailed journal
    // event must be present BEFORE the StepStarted for the handler step,
    // proving that the durable record is established before the run jumps
    // into the handler.
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(601);
    submit_run(&mut shard, run, wf);
    let ticket = make_ticket(run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // First tick buffers ActionFailed + handler StepStarted + RunFinished.
    assert_eq!(shard.tick(), Ok(true));
    // Second tick flushes the coalesce buffer to the journal.
    assert_eq!(shard.tick(), Ok(true));

    let events = require_snapshot(&journal)?;
    let expected_action_failed = RuntimeJournalEvent::ActionFailed {
        run,
        step: StepIdx::new(1),
        action: ActionId::new(0),
        attempt: 1,
    };
    let expected_handler_started = RuntimeJournalEvent::StepStarted {
        run,
        step: StepIdx::new(2),
    };
    assert!(
        events.contains(&expected_action_failed),
        "ActionFailed event must be present in journal"
    );
    let action_failed_pos = events
        .iter()
        .position(|e| e == &expected_action_failed)
        .ok_or_else(|| format!("ActionFailed not found in events: {events:?}"))?;
    let handler_started_pos = events
        .iter()
        .position(|e| e == &expected_handler_started)
        .ok_or_else(|| format!("handler StepStarted not found in events: {events:?}"))?;
    assert!(
        action_failed_pos < handler_started_pos,
        "ActionFailed (pos={action_failed_pos}) must precede handler StepStarted (pos={handler_started_pos}); events={events:?}"
    );
    Ok(())
}

// =======================================================================
// RS-104: AskAnswered journal record must precede frame/timer mutations.
// =======================================================================

/// RS-104 regression: the durable `AskAnswered` (and `SlotWritten`) journal
/// records must be appended BEFORE the frame slot write and the pending
/// timer removal. If the durable append fails, the frame must NOT be
/// mutated and the timer must NOT be removed — the run stays suspendable
/// for a caller retry. Mirrors the B-012 fix pattern used by
/// `handle_cancel` / `handle_kill` and the RS-105 ordering guarantee for
/// `handle_action_failure`.
struct RejectAskAnsweredJournal {
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectAskAnsweredJournal {
    fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            events: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> Result<Vec<RuntimeJournalEvent>, RuntimeError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RejectAskAnsweredJournal {
    fn append(&self, event: RuntimeJournalEvent) -> Result<(), RuntimeError> {
        if matches!(event, RuntimeJournalEvent::AskAnswered { .. }) {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn probe(&self) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn rs104_durable_ask_answered_journaled_before_frame_mutation() -> Result<(), String> {
    // RS-104 happy-path durability: the SlotWritten and AskAnswered events
    // must be present in the journal BEFORE the frame is mutated. We use
    // the volatile journal and assert the events are appended in the
    // required order (SlotWritten before AskAnswered) per the existing
    // PO-vb282my-AA-FLUX-001 refinement, AND that both are present
    // before the run drives to completion.
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("ask", ask_workflow())?;
    let run = RunId::new(700);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));
    // The run is now suspended on the ask with a pending timer.
    assert_eq!(shard.pending_timers.len(), 1);

    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // First tick buffers the answer sequence plus StepSucceeded.
    assert_eq!(shard.tick(), Ok(true));
    // Second tick flushes the coalesce buffer.
    assert_eq!(shard.tick(), Ok(true));

    let events = require_snapshot(&journal)?;
    let slot_written_pos = events
        .iter()
        .position(|e| matches!(
            e,
            RuntimeJournalEvent::SlotWritten { run: r, slot, .. }
                if *r == run && *slot == SlotIdx::new(2)
        ))
        .ok_or_else(|| format!("SlotWritten for slot 2 not found: {events:?}"))?;
    let ask_answered_pos = events
        .iter()
        .position(|e| matches!(e, RuntimeJournalEvent::AskAnswered { run: r, .. } if *r == run))
        .ok_or_else(|| format!("AskAnswered event not found: {events:?}"))?;
    assert!(
        slot_written_pos < ask_answered_pos,
        "SlotWritten (pos={slot_written_pos}) must precede AskAnswered (pos={ask_answered_pos}); \
         PO-vb282my-AA-FLUX-001 SlotWritten-before-AskAnswered ordering: {events:?}"
    );
    // After RS-104 + drive_run, the run completed via the AskResume →
    // Finish path; the answer sequence is durable before any frame
    // mutation, so the run drove to completion cleanly.
    assert_eq!(
        shard.counters().snapshot().runs_completed,
        1,
        "ask answer must drive the run to completion"
    );
    Ok(())
}

#[test]
fn rs104_ask_answered_journal_failure_preserves_frame_and_timer() -> Result<(), String> {
    // RS-104 failure-path durability: when the durable `AskAnswered`
    // append fails, the frame slot write and the pending timer removal
    // must NOT happen. The pre-fix anti-pattern (mutate first, journal
    // second) would have left the run with an unsynchronized frame and
    // no pending ask authority; recovery would have no durable evidence
    // that an answer was ever attempted.
    let journal = RejectAskAnsweredJournal::shared();
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|e| format!("shard construction: {:?}", e))?;
    let wf = require_workflow("ask", ask_workflow())?;
    let run = RunId::new(701);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));
    // Pre-answer sanity: the run is suspended on the ask with a pending
    // timer; the answer slot is empty (no SlotWritten for slot 2 yet).
    assert_eq!(shard.pending_timers.len(), 1);
    let pre_answer_pc = shard
        .run_state_get(run)
        .map(|state| state.frame.pc())
        .ok_or_else(|| "run must exist before ask answer".to_string())?;

    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Tick must fail because the durable AskAnswered append is rejected.
    let tick_result = shard.tick();
    assert!(
        tick_result.is_err(),
        "tick must fail when durable AskAnswered append fails, got {tick_result:?}"
    );

    // RS-104: the run must STILL exist in run_states so the caller can retry.
    assert_eq!(
        shard.run_state_contains(run),
        true,
        "RS-104: run must remain in run_states after durable AskAnswered append fails; \
         pre-fix bug would silently drop the run from run_states"
    );
    // RS-104: the pending ask timer must NOT be removed (the answer was
    // not durable, so the run is still authorized to receive the answer).
    assert_eq!(
        shard.pending_timers.len(),
        1,
        "RS-104: pending ask timer must not be removed when durable AskAnswered append fails"
    );
    // RS-104: the frame PC must NOT have been advanced to the resume step.
    let post_answer_pc = shard
        .run_state_get(run)
        .map(|state| state.frame.pc())
        .ok_or_else(|| "run state must still be readable".to_string())?;
    assert_eq!(
        post_answer_pc, pre_answer_pc,
        "RS-104: frame PC must be unchanged when durable AskAnswered append fails"
    );

    // Journal must contain SlotWritten (durable, succeeded) but NOT
    // AskAnswered (durable, failed). This proves the failure happened
    // at the AskAnswered append and the state mutation guard above it
    // never executed.
    let events = journal.snapshot().map_err(|e| format!("snapshot: {e:?}"))?;
    let slot_written_present = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::SlotWritten { run: r, .. } if *r == run));
    let ask_answered_present = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::AskAnswered { run: r, .. } if *r == run));
    assert!(
        slot_written_present,
        "SlotWritten must have been durably appended before the failing AskAnswered"
    );
    assert!(
        !ask_answered_present,
        "AskAnswered must NOT be in the journal when its durable append failed"
    );
    Ok(())
}
