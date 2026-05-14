
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
        let Some(state) = shard.runs.get_mut(&run) else {
            assert_eq!(None::<()>, Some(()), "run should remain active");
            return;
        };
        assert_eq!(state.action_attempts.get(0).copied(), Some(1));
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 0,
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
        assert_eq!(shard.tick(), Ok(true));
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
            encoded_len: 0,
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
        let Some(state) = shard.runs.get_mut(&run) else {
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
            encoded_len: 0,
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
        let Some(state_after) = shard.runs.get(&run) else {
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
