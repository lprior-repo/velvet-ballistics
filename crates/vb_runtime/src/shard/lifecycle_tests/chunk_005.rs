
    #[test]
    fn action_failure_without_handler_emits_action_failed_before_run_failed() -> Result<(), String>
    {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
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
            attempt: 1},
            RuntimeJournalEvent::RunFailed { run },
        );
        Ok(())
    }

    #[test]
    fn action_failure_routes_to_error_handler() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
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
    fn action_failure_routed_to_handler_emits_action_failed_before_handler_step()
    -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
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
            attempt: 1},
            RuntimeJournalEvent::StepStarted {
                run,
                step: StepIdx::new(2),
            },
        );
        Ok(())
    }

    #[test]
    fn action_failure_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn retry_exhaustion_emits_single_action_failed() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let run = RunId::new(620);
        submit_run(&mut shard, run, retry_workflow()?);
        enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
        enqueue_action_failure(&mut shard, run, StepIdx::new(1), 2);
        let events = require_snapshot(&journal)?;
        assert_retry_exhaustion_journal(&events, run);
        Ok(())
    }

    #[test]
    fn ask_answer_completes_ask_workflow() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = ask_workflow() else {
            return;
        };
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
    }

    #[test]
    fn ask_answer_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
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
    }

    #[test]
    fn timer_fire_advances_wait_to_completion() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
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
    }

    #[test]
    fn timer_fire_for_non_timer_run_returns_invalid_timer_fire() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
    }

    #[test]
    fn timer_fire_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(invalid_timer_command(RunId::new(9999))),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    }

    #[test]
    fn cancel_removes_active_run_and_increments_failed() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn cancel_nonexistent_run_succeeds_without_counter_change() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel {
                run: RunId::new(9999),
            reason: None}),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    // =======================================================================
    // finish_run terminal fence
    // =======================================================================

    #[test]
    fn finish_run_appends_run_finished_event_and_inserts_terminal_run() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let Some(wf) = finished_workflow() else {
            return Ok(());
        };
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
        assert!(shard.terminal_runs.contains(&run));
        // Journal should contain RunFinished event
        let events = require_snapshot(&journal)?;
        let has_run_finished = events.iter().any(|e| {
            matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run)
        });
        assert!(has_run_finished, "RunFinished event not found in journal: {events:?}");
        Ok(())
    }

    // =======================================================================
    // retry_is_available and apply_action_failure_to_state
    // =======================================================================

    #[test]
    fn retry_remaining_advances_attempt_and_resumes_drive() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let run = RunId::new(501);
        submit_run(&mut shard, run, retry_workflow()?);
        // After submission, the run should be suspended on action step 1
        // with attempt 1 and capacity 2 from the retry policy
        let state = shard.runs.get(&run).ok_or("run should exist")?;
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
        let state = shard.runs.get(&run).ok_or("run should exist after retry")?;
        assert_eq!(state.action_attempts.get(1).copied(), Some(2));
        // Run is still active
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    #[test]
    fn retry_exhausted_fails_run_when_no_more_attempts() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
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
        let mut shard = Shard::new(small_config());
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
        let mut shard = Shard::new(small_config());
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
    fn handle_action_failure_rejects_when_step_out_of_bounds() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
    }
