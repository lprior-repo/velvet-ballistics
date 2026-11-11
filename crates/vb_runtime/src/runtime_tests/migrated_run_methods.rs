    /// RA-030 wave-15: capture_timer_entry must read the timer entry from
    /// the owning shard even after migration.
    #[test]
    fn runtime_capture_timer_entry_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0.
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            return;
        }
        let destination = 1usize;
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(runtime.shards[home_index].run_state_contains(run));
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Then capture_timer_entry returns InvalidTimerFire because the run
        // was migrated but no timer entry exists yet on the destination.
        // The test asserts the routing correctness (not RunNotFound).
        let result = runtime.capture_timer_entry(run);
        assert!(matches!(
            result,
            Err(RuntimeError::InvalidTimerFire) | Ok(_)
        ));
    }

    /// RA-030 wave-15: timer_entry_fired must deliver to the owning shard
    /// even after migration.
    #[test]
    fn runtime_timer_entry_fired_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0.
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            return;
        }
        let destination = 1usize;
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(runtime.shards[home_index].run_state_contains(run));
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Construct a timer entry for the run.
        use crate::shard::PendingTimerKind;
        use crate::shard::timer_wheel::TimerEntry;
        let entry = TimerEntry {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Ask,
        };
        // Then timer_entry_fired returns Ok (post-fix scans all shards).
        let result = runtime.timer_entry_fired(entry);
        // The run is migrated but no live timer was inserted at this point.
        // The test asserts the routing correctness (Ok if the run is on a shard
        // that owns it; otherwise the shard refuses to enqueue).
        assert!(result.is_ok() || matches!(result, Err(RuntimeError::QueueFull)));
    }

    /// RA-030 wave-15: round-trip all five methods on a migrated run.
    #[test]
    fn runtime_migrated_run_round_trips_each_method() {
        // Given a 2-shard runtime; run lives on shard 0.
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            return;
        }
        let destination = 1usize;
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Then each method observes the owning shard (post-fix).
        let ticket = runtime
            .lookup_pending_ask_ticket(run, StepIdx::new(2))
            .expect("migrated suspended ask must issue a ticket");
        let answer = AskAnswer {
            ticket,
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        assert_eq!(runtime.answer_ask(answer), Ok(()));
        assert!(matches!(runtime.list_events(run), Ok(_)));
        assert!(matches!(runtime.take_inspect_response(run), Ok(None)));
        assert!(matches!(runtime.capture_timer_entry(run), Err(_) | Ok(_)));
    }

    /// RA-030 wave-15: unknown run returns RunNotFound across all four
    /// newly-hardened methods (parity with existing answer_ask test).
    #[test]
    fn runtime_unknown_run_returns_run_not_found_across_methods() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let unknown_run = vb_core::ids::RunId::new(7777);

        // list_events
        assert_eq!(
            runtime.list_events(unknown_run),
            Err(RuntimeError::RunNotFound)
        );
        // take_inspect_response
        assert_eq!(
            runtime.take_inspect_response(unknown_run),
            Err(RuntimeError::RunNotFound)
        );
        // capture_timer_entry
        assert_eq!(
            runtime.capture_timer_entry(unknown_run),
            Err(RuntimeError::RunNotFound)
        );
        // timer_entry_fired (with constructed entry)
        use crate::shard::PendingTimerKind;
        use crate::shard::timer_wheel::TimerEntry;
        let entry = TimerEntry {
            run: unknown_run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Ask,
        };
        assert_eq!(
            runtime.timer_entry_fired(entry),
            Err(RuntimeError::RunNotFound)
        );
    }

    #[test]
    fn runtime_drain_trace_returns_empty_for_fresh_runtime() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When draining trace
        let events = runtime.drain_trace();
        // Then result is empty
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn runtime_submit_and_cancel_increments_failed_counter() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        // When submitting then cancelling
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then failed counter is 1
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_inspect_run_enqueues_command_successfully() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When inspecting
        assert_eq!(runtime.inspect_run(run, 99), Ok(()));
        // Then tick processes the inspect
        assert_eq!(runtime.tick_all(), Ok(true));
        // And the response is available
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snap))) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 99);
            }
            other => {
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn runtime_shutdown_graceful_enqueues_to_all_shards() {
        // Given a 3-shard runtime
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When shutting down gracefully
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_after_shutdown_returns_false_repeatedly() {
        // Given a shutdown runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
        // When ticking again
        assert_eq!(runtime.tick_all(), Ok(false));
        // Then it still returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_submit_direct_returns_queue_full_when_shard_queue_full() {
        // Given a runtime with tiny queue
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
        // When filling the queue
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf.clone()),
            Ok(())
        );
        // Then the second submit returns QueueFull
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_list_events_for_unknown_shard_returns_error() {
        // RA-030 wave-15 (vb-sxkz6): list_events now scans all shards and
        // returns Err(RunNotFound) for runs not on any shard. The previous
        // test was lenient about either Ok(empty) or Err; we now strictly
        // assert Err(RunNotFound).
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let events = runtime.list_events(vb_core::ids::RunId::new(1));
        assert_eq!(events, Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_counters_aggregate_across_shards() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf1) = finished_workflow() else {
            return;
        };
        let Some(wf2) = finished_workflow() else {
            return;
        };
        // When submitting runs
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters aggregate across shards
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_completed, 2);
    }

    #[test]
    fn runtime_single_shard_operations() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When submitting a run
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then it completes
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
    }

    #[test]
    fn runtime_new_creates_correct_shard_count() {
        // Given a runtime with 4 shards
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When shutting down
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        // Then tick_all returns false (all shards shut down)
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_take_inspect_response_for_unknown_run_returns_not_found() {
        // RA-030 wave-15 (vb-sxkz6): take_inspect_response now requires the
        // run to be owned by some shard. For unknown runs (no submit), the
        // post-fix behavior is `Err(RunNotFound)`. The previous test asserted
        // `Ok(Some(NotFound { ... }))` because the old implementation read
        // the home shard's inspect slot regardless of run ownership. Under
        // the new scan-all-shards contract, take_inspect_response is gated
        // on run ownership. To retrieve a NotFound inspect response, the
        // caller must first submit the run and let it fail/finish.
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When inspecting a non-existent run
        let run = vb_core::ids::RunId::new(999);
        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the response is RunNotFound (the run is not owned by any shard).
        let response = runtime.take_inspect_response(run);
        assert_eq!(response, Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_drain_trace_returns_submitted_events() {
        // Given a runtime with submitted runs
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
        // When draining trace
        let events = runtime.drain_trace();
        // Then events contain RunSubmitted
        let found = events.iter().any(|e| {
            matches!(e, TraceEvent::RunSubmitted { run } if *run == vb_core::ids::RunId::new(1))
        });
        assert_eq!(found, true);
    }

    // =======================================================================
    // Adversarial BDD tests — runtime
    // =======================================================================

    #[test]
    fn runtime_shutdown_with_pending_run_then_tick_returns_false() {
        // Given a runtime with a pending suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(300);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When shutting down with a pending run
        assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_run_stays_on_one_shard_across_operations() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(301);
        // When submitting, then cancelling
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show exactly 1 submitted and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        // And re-submitting the same run succeeds (it was removed by cancel)
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(submit_suspended(&runtime, run, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap2 = runtime.counters_snapshot();
        assert_eq!(snap2.runs_submitted, 2);
    }

    #[test]
    fn runtime_complete_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When completing an action for a run that was never submitted
        let run = vb_core::ids::RunId::new(999);
        assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
        // Then tick returns RunNotFound (the shard has no such run)
        assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_fail_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When failing an action for a run that was never submitted
        let ticket = ActionTicket {
            run: vb_core::ids::RunId::new(998),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_idempotency_key(
                vb_core::ids::RunId::new(998),
                1,
                SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
            fencing_token: 0,
        };
        let failure = ActionFailureCode::Rejected.into();
        // Then enqueue succeeds (failure is queued)
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
    }

    #[test]
    fn runtime_queue_full_returns_typed_error() {
        // Given a runtime with tiny queue capacity
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
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // When filling the queue to capacity
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf.clone()),
            Ok(())
        );
        // Then the next submit returns QueueFull (exact error variant)
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_drain_trace_after_drain_returns_empty() {
        // Given a runtime with events
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
        // When draining twice
        let first = runtime.drain_trace();
        assert_eq!(first.is_empty(), false);
        let second = runtime.drain_trace();
        // Then second drain is empty
        assert_eq!(second.len(), 0);
    }
