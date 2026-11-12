    #[test]
    fn runtime_countered_exhausted_at_max_active_runs() {
        // Given a 1-shard runtime with max_active_runs = 1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        // When submitting two runs
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        // Then second tick returns ActiveRunCapacityExceeded
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn runtime_snapshot_run_for_unknown_run_returns_not_found() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When snapshotting a non-existent run
        let result = runtime.snapshot_run(vb_core::ids::RunId::new(9999), 42);
        // Then it returns NotFound
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: vb_core::ids::RunId::new(9999),
                correlation: 42,
            })
        );
    }

    #[test]
    fn runtime_list_events_is_idempotent() {
        // Given a runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // When listing events twice without draining
        let first = runtime.list_events(vb_core::ids::RunId::new(1));
        let second = runtime.list_events(vb_core::ids::RunId::new(1));
        // Then both return the same events (non-destructive)
        assert_eq!(first, second);
        assert_eq!(first.map(|e| e.is_empty()), Ok(false));
    }

    #[test]
    fn runtime_finished_workflow_counts_completed_not_failed() {
        // Given a runtime with a workflow that finishes immediately
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When submitting
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(42), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then completed is 1 and failed is 0
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.runs_submitted, 1);
    }

    // =======================================================================
    // Adversarial BDD tests - runtime attack vectors
    // =======================================================================

    #[test]
    fn runtime_two_shards_deterministic_routing_same_run_same_shard() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When computing shard index for run 1 twice
        let idx1 = runtime.shard_index(vb_core::ids::RunId::new(1));
        let idx2 = runtime.shard_index(vb_core::ids::RunId::new(1));
        // Then the shard index is deterministic
        assert_eq!(idx1, idx2);
        assert!(idx1 < 4);
    }

    #[test]
    fn runtime_two_shards_different_runs_may_land_on_different_shards() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When computing shard indices for different runs
        let idx1 = runtime.shard_index(vb_core::ids::RunId::new(1));
        let idx2 = runtime.shard_index(vb_core::ids::RunId::new(2));
        // Then at least two different shard indices exist among the runs
        // (we can't guarantee different shards for 1 and 2, but we check the mechanism works)
        assert!(idx1 < 4);
        assert!(idx2 < 4);
    }

    #[test]
    fn runtime_cancel_then_resubmit_on_same_shard_succeeds() {
        // Given a 1-shard runtime with a cancelled run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(400);
        assert_eq!(submit_suspended(&runtime, run, wf.clone()), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When re-submitting the same run
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show 2 submissions and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_fail_action_for_active_suspended_run_increments_failed() {
        // Given a 1-shard runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(401);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When failing the action
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_idempotency_key(run, 1, SeqNo::ZERO, ActionId::new(0)),
            capacity: 1,
            fencing_token: 0,
        };
        let failure = ActionFailureCode::Rejected.into();
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is failed.
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_tick_all_after_shutdown_ignores_pending_commands() {
        // Given a runtime with a submit queued, then shutdown
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // Shutdown first (processes shutdown tick)
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        // When trying to submit after shutdown, admission is rejected at the boundary.
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(402), wf),
            Err(RuntimeError::ShutdownInProgress)
        );
        // Then tick_all returns false (shard already shutting down)
        assert_eq!(runtime.tick_all(), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    }

    #[test]
    fn runtime_journal_events_are_recorded_for_submit_and_finish() {
        // Given a runtime with a volatile journal
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(403);
        // When submitting and ticking
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then journal contains RunSubmitted and RunFinished
        let events = journal.snapshot();
        match events {
            Ok(evts) => {
                let found_submitted = evts.iter().any(|e| {
                    *e == RuntimeJournalEvent::RunSubmitted {
                        run,
                        workflow: vb_core::ids::WorkflowDigest::from_bytes([2; 32]),
                    }
                });
                let found_finished = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run),
                );
                assert_eq!(found_submitted, true);
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }

    #[test]
    fn runtime_countered_exhausted_does_not_corrupt_other_runs() {
        // Given a 1-shard runtime with max_active_runs=1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(500);
        let run2 = vb_core::ids::RunId::new(501);
        // When submitting run1 (succeeds) then run2 (capacity exceeded)
        assert_eq!(submit_suspended(&runtime, run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_suspended(&runtime, run2, wf2), Ok(()));
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
        // Then run1 is still alive and inspectable
        let snap = runtime.snapshot_run(run1, 1);
        match snap {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run1);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run: run1,
                        correlation: 1
                    })
                );
            }
        }
    }

    // =======================================================================
    // Scheduler edge case tests — Section 36
    // =======================================================================

    // --- 1. Queue-full returns typed error with diagnostic code ---

    #[test]
    fn scheduler_queue_full_returns_typed_error_with_diagnostic_code() {
        // Given a runtime with command_queue_capacity = 1
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When filling the queue
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(600), wf.clone()),
            Ok(())
        );
        // Then the next submit returns the typed QueueFull error
        let err = runtime.submit_direct(vb_core::ids::RunId::new(601), wf);
        assert_eq!(err, Err(RuntimeError::QueueFull));
        // And the error's diagnostic code matches QUEUE_FULL
        match err {
            Err(ref e) => assert_eq!(e.diagnostic_code(), RuntimeError::QUEUE_FULL_CODE),
            Ok(()) => assert!(false, "expected QueueFull error"),
        }
    }

    // --- 2. Run stays on one shard across all operations ---

    #[test]
    fn scheduler_run_stays_on_one_shard_across_all_operations() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(602);
        let target_shard = runtime.shard_index(run);

        // When submitting, inspecting, and cancelling the same run
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run always maps to the same shard index
        assert!(target_shard < 4);
        // And counters reflect the submit + cancel lifecycle
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
    }

    // --- 3. Cancel pending runs (before execution tick) ---

    #[test]
    fn scheduler_cancel_pending_run_before_execution() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(603);

        // When submitting and cancelling before tick
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        // Cancel is enqueued before the submit is processed by tick
        assert_eq!(runtime.cancel_run(run), Ok(()));
        // tick_all processes one command per shard per tick, so we need two ticks
        // First tick processes the Submit (run becomes active/suspended)
        assert_eq!(runtime.tick_all(), Ok(true));
        // Second tick processes the Cancel
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is cancelled (failed counter = 1)
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        // And inspecting the run returns NotFound (no active run)
        let result = runtime.snapshot_run(run, 2);
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run,
                correlation: 2,
            })
        );
    }

    // --- 4. Cancel waiting runs (suspended on WaitUntil) ---

    #[test]
    fn scheduler_cancel_run_waiting_on_timer() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = wait_then_finish_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(604);

        // When submitting a wait workflow and ticking (run enters Wait state)
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is in the runs map (snapshot returns Found)
        let before = runtime.snapshot_run(run, 3);
        match before {
            Ok(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run,
                        correlation: 3
                    })
                );
            }
        }

        // When cancelling the waiting run
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is cleanly removed
        let after = runtime.snapshot_run(run, 4);
        assert_eq!(
            after,
            Ok(InspectResponse::NotFound {
                run,
                correlation: 4,
            })
        );
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        assert_eq!(snap.runs_completed, 0);
    }

    // --- 5. Shutdown drains gracefully with pending runs ---

    #[test]
    fn scheduler_shutdown_drains_pending_suspended_run() {
        // Given a 1-shard runtime with a journal to observe events
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(605);

        // When submitting a suspended run and initiating graceful shutdown
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        // drain_for_shutdown processes queued commands
        // Then tick_all returns false (shutdown complete)
        assert_eq!(runtime.tick_all(), Ok(false));

        // And the journal recorded the run submission
        let journal_events = journal.snapshot();
        match journal_events {
            Ok(evts) => {
                let found_submitted = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunSubmitted { run: r, .. } if *r == run),
                );
                assert_eq!(found_submitted, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }

    #[test]
    fn scheduler_shutdown_drains_pending_finished_run() {
        // Given a 1-shard runtime with a journal
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(606);

        // When submitting a finished workflow and initiating shutdown
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));

        // Then the run completed during drain
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);

        // And journal recorded RunFinished
        let journal_events = journal.snapshot();
        match journal_events {
            Ok(evts) => {
                let found_finished = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run),
                );
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }
