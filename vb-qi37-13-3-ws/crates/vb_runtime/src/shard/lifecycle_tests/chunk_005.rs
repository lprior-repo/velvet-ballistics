
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
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
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
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    }

    #[test]
    fn timer_fire_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::TimerFired {
                run: RunId::new(9999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
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
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }
