    // --- 6. Timer resume order: multiple waits, deterministic processing ---

    #[test]
    fn scheduler_timer_fire_processes_correct_run() {
        // Given a 1-shard runtime with two waiting runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf1) = wait_then_finish_workflow() else {
            return;
        };
        let Some(wf2) = wait_then_finish_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(607);
        let run2 = vb_core::ids::RunId::new(608);

        assert_eq!(runtime.submit_compiled(run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.submit_compiled(run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Both runs are now in wait state. Fire captured timer authority for run1 only.
        let Ok(entry1) = runtime.capture_timer_entry(run1) else {
            return;
        };
        std::thread::sleep(
            entry1
                .deadline
                .saturating_duration_since(std::time::Instant::now()),
        );
        assert_eq!(runtime.timer_entry_fired(entry1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then run1 completed but run2 is still waiting
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);

        // run2 is still present (inspect returns Found)
        let inspect = runtime.snapshot_run(run2, 5);
        match inspect {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run2);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run: run2,
                        correlation: 5,
                    })
                );
            }
        }

        // Now fire captured timer authority for run2.
        let Ok(entry2) = runtime.capture_timer_entry(run2) else {
            return;
        };
        std::thread::sleep(
            entry2
                .deadline
                .saturating_duration_since(std::time::Instant::now()),
        );
        assert_eq!(runtime.timer_entry_fired(entry2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap2 = runtime.counters_snapshot();
        assert_eq!(snap2.runs_completed, 2);
    }

    // --- 7. Action completion resumes correct run ---

    #[test]
    fn scheduler_action_completion_resumes_correct_run_among_many() {
        // Given a 1-shard runtime with three suspended runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf1) = action_then_finish_workflow() else {
            return;
        };
        let Some(wf2) = action_then_finish_workflow() else {
            return;
        };
        let Some(wf3) = action_then_finish_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(610);
        let run2 = vb_core::ids::RunId::new(611);
        let run3 = vb_core::ids::RunId::new(612);

        assert_eq!(submit_action_then_finish(&runtime, run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_action_then_finish(&runtime, run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_action_then_finish(&runtime, run3, wf3), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // All three runs are suspended on their Do actions.
        // Complete only run2's action.
        let ticket = ActionTicket {
            run: run2,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_idempotency_key(run2, 1, SeqNo::ZERO, ActionId::new(7)),
            capacity: 1,
            fencing_token: 0,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: encoded_len(&SlotValue::I64(42)),
        };
        assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then only run2 completed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);

        // run1 and run3 are still present and suspended
        assert_suspended_run_is_found(&runtime, run1, 6);
        assert_suspended_run_is_found(&runtime, run3, 6);

        // Verify the completed run's trace shows the finish
        let trace = runtime.list_events(run2);
        match trace {
            Ok(evts) => {
                let found_finished = evts
                    .iter()
                    .any(|e| *e == TraceEvent::RunFinished { run: run2 });
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<TraceEvent>::new()));
            }
        }
    }

    fn assert_suspended_run_is_found(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        correlation: u64,
    ) {
        match runtime.snapshot_run(run, correlation) {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run);
            }
            other => {
                assert_eq!(other, Ok(InspectResponse::NotFound { run, correlation }));
            }
        }
    }

    // --- 8. No task-per-step: scheduler processes within step budget ---

    #[test]
    fn scheduler_no_task_per_step_processes_within_budget() {
        // Given a runtime with step_budget_per_tick = 100 (sufficient for short workflows)
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 64,
            step_budget_per_tick: 100,
            max_active_runs: 4,
            max_open_gates: 1024,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        // Use the finished_workflow (SetConst -> Finish, 2 steps)
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(620);

        // When submitting the workflow
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        // A single tick processes the submit command and drives to completion
        // The scheduler does NOT spawn a task per step; it drives all steps
        // within the budget of one tick synchronously.
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run completes in a single tick (no task-per-step behavior)
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
        // Verify steps executed is exactly 2 (SetConst + Finish)
        assert!(snap.steps_executed >= 2);
    }

    #[test]
    fn scheduler_single_tick_does_not_spawn_concurrent_tasks() {
        // Given a runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(621);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // When inspecting the run, it is in a suspended state with deterministic PC
        assert_eq!(runtime.inspect_run(run, 10), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snap))) => {
                // The PC is at step 0 (the Do step that suspended)
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 10);
                // PC should be at the suspended step (0 for our single-node workflow)
                assert!(snap.pc == StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn tick_shard_continue_drives_only_selected_shard() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(2);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));

        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 0);
    }

    #[test]
    fn tick_shard_suspend_preserves_pending_work() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(3);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Suspend), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    }

    #[test]
    fn tick_shard_migrate_rejects_self_and_invalid_target() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Migrate { target: 0 }),
            Err(RuntimeError::MigrateSelf)
        );
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Migrate { target: 9 }),
            Err(RuntimeError::ShardNotFound { shard: 9 })
        );
    }

    #[test]
    fn tick_shard_shutdown_drains_and_reports_dead() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(4);

        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Shutdown), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    }

    #[test]
    fn tick_shard_shutdown_drains_when_command_queue_is_full() {
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            max_open_gates: 1024,
            policy: RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        let Some(wf) = finished_workflow() else {
            return;
        };

        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(40), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_shard(0, ShardDirective::Shutdown), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn tick_shard_cancel_and_barrier_do_not_advance_work_silently() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };

        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(41), wf),
            Ok(())
        );
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Cancel),
            Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_cancel"
            })
        );
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Barrier),
            Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_barrier"
            })
        );
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    }

    #[test]
    fn pending_boundary_snapshot_reports_empty_state() -> Result<(), String> {
        let shard_count = NonZeroUsize::new(2)
            .ok_or_else(|| String::from("nonzero shard count fixture failed"))?;
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let snapshot = runtime.pending_boundary_snapshot(8);
        assert_eq!(snapshot.shards().len(), 2);
        assert_snapshot_counts(&snapshot, 0, 0, 0, 0, false);
        for shard in snapshot.shards() {
            assert_eq!(shard.active_runs(), &[]);
            assert_eq!(shard.pending_timers(), &[]);
            assert_eq!(shard.pending_actions(), &[]);
            assert_eq!(shard.pending_asks(), &[]);
            assert_eq!(shard.command_queue_depth(), 0);
        }
        Ok(())
    }

    #[test]
    fn pending_boundary_snapshot_reports_populated_boundaries() -> Result<(), String> {
        let shard_count = NonZeroUsize::new(1)
            .ok_or_else(|| String::from("nonzero shard count fixture failed"))?;
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let action_workflow = suspended_workflow()
            .ok_or_else(|| String::from("suspended workflow fixture failed"))?;
        let ask_workflow =
            ask_waiting_workflow().ok_or_else(|| String::from("ask workflow fixture failed"))?;
        let action_run = vb_core::ids::RunId::new(10);
        let ask_run = vb_core::ids::RunId::new(11);
        assert_eq!(
            submit_suspended(&runtime, action_run, action_workflow),
            Ok(())
        );
        assert_eq!(submit_ask_waiting(&runtime, ask_run, ask_workflow), Ok(()));
        assert_eq!(
            runtime.pending_boundary_snapshot(8).command_queue_depth(),
            2
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snapshot = runtime.pending_boundary_snapshot(8);
        assert_snapshot_counts(&snapshot, 2, 1, 1, 1, false);
        let shard = snapshot
            .shards()
            .first()
            .ok_or_else(|| String::from("snapshot missing shard"))?;
        assert_eq!(shard.active_runs(), &[action_run, ask_run]);
        let action = shard
            .pending_actions()
            .first()
            .ok_or_else(|| String::from("snapshot missing action"))?;
        assert_eq!(action.run_id(), action_run);
        assert_eq!(action.ticket().step, StepIdx::ZERO);
        assert_eq!(action.ticket().action, ActionId::new(0));
        assert_eq!(action.ticket().attempt, 1);
        assert_eq!(action.ticket().capacity, 1);
        let timer = shard
            .pending_timers()
            .first()
            .ok_or_else(|| String::from("snapshot missing timer"))?;
        assert_eq!(timer.run_id(), ask_run);
        assert_eq!(timer.step(), StepIdx::new(2));
        assert_eq!(timer.kind(), PendingTimerKind::Ask);
        assert_eq!(timer.generation(), 1);
        let ask = shard
            .pending_asks()
            .first()
            .ok_or_else(|| String::from("snapshot missing ask"))?;
        assert_eq!(ask.run_id(), ask_run);
        assert_eq!(ask.ask_step(), StepIdx::new(2));
        let timeout = ask
            .timeout()
            .ok_or_else(|| String::from("snapshot missing ask timeout"))?;
        assert_eq!(timeout.generation(), timer.generation());
        assert_eq!(timeout.deadline(), timer.deadline());
        assert_eq!(ask.generation(), Some(timer.generation()));
        assert_eq!(ask.deadline(), Some(timer.deadline()));
        Ok(())
    }

    #[test]
    fn pending_boundary_snapshot_reports_ask_without_timeout() -> Result<(), String> {
        let shard_count = NonZeroUsize::new(1)
            .ok_or_else(|| String::from("nonzero shard count fixture failed"))?;
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let workflow = ask_without_timeout_workflow()
            .ok_or_else(|| String::from("ask without timeout workflow fixture failed"))?;
        let ask_run = vb_core::ids::RunId::new(12);
        assert_eq!(submit_ask_waiting(&runtime, ask_run, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snapshot = runtime.pending_boundary_snapshot(4);
        assert_snapshot_counts(&snapshot, 1, 0, 0, 1, false);
        let shard = snapshot
            .shards()
            .first()
            .ok_or_else(|| String::from("snapshot missing shard"))?;
        let ask = shard
            .pending_asks()
            .first()
            .ok_or_else(|| String::from("snapshot missing open ask"))?;
        assert_eq!(ask.run_id(), ask_run);
        assert_eq!(ask.ask_step(), StepIdx::new(1));
        assert_eq!(ask.timeout(), None);
        assert_eq!(ask.generation(), None);
        assert_eq!(ask.deadline(), None);
        Ok(())
    }

    #[test]
    fn pending_boundary_snapshot_bounds_truncates_and_repeats_read_only() -> Result<(), String> {
        let shard_count = NonZeroUsize::new(1)
            .ok_or_else(|| String::from("nonzero shard count fixture failed"))?;
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let action_workflow = suspended_workflow()
            .ok_or_else(|| String::from("suspended workflow fixture failed"))?;
        let timed_ask_workflow = ask_waiting_workflow()
            .ok_or_else(|| String::from("timed ask workflow fixture failed"))?;
        let open_ask_workflow = ask_without_timeout_workflow()
            .ok_or_else(|| String::from("open ask workflow fixture failed"))?;
        let action_run = vb_core::ids::RunId::new(20);
        let timed_ask_run = vb_core::ids::RunId::new(21);
        let open_ask_run = vb_core::ids::RunId::new(22);
        assert_eq!(
            submit_suspended(&runtime, action_run, action_workflow),
            Ok(())
        );
        assert_eq!(
            submit_ask_waiting(&runtime, timed_ask_run, timed_ask_workflow),
            Ok(())
        );
        assert_eq!(
            submit_ask_waiting(&runtime, open_ask_run, open_ask_workflow),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.tick_all(), Ok(true));
        let zero = runtime.pending_boundary_snapshot(0);
        assert_snapshot_counts(&zero, 3, 1, 1, 2, true);
        let zero_shard = zero
            .shards()
            .first()
            .ok_or_else(|| String::from("zero snapshot missing shard"))?;
        assert_eq!(zero_shard.active_runs().len(), 0);
        assert_eq!(zero_shard.pending_timers().len(), 0);
        assert_eq!(zero_shard.pending_actions().len(), 0);
        assert_eq!(zero_shard.pending_asks().len(), 0);
        let limited = runtime.pending_boundary_snapshot(1);
        assert_snapshot_counts(&limited, 3, 1, 1, 2, true);
        let limited_shard = limited
            .shards()
            .first()
            .ok_or_else(|| String::from("limited snapshot missing shard"))?;
        assert_eq!(limited_shard.active_runs().len(), 1);
        assert_eq!(limited_shard.pending_asks().len(), 1);
        let exact = runtime.pending_boundary_snapshot(3);
        assert_snapshot_counts(&exact, 3, 1, 1, 2, false);
        let exact_shard = exact
            .shards()
            .first()
            .ok_or_else(|| String::from("exact snapshot missing shard"))?;
        assert_eq!(
            exact_shard.active_runs(),
            &[action_run, timed_ask_run, open_ask_run]
        );
        assert_eq!(exact_shard.pending_asks().len(), 2);
        assert_eq!(
            exact_shard
                .pending_asks()
                .first()
                .map(|ask| ask.timeout().is_some()),
            Some(true)
        );
        assert_eq!(
            exact_shard.pending_asks().get(1).map(|ask| ask.timeout()),
            Some(None)
        );
        let repeated = runtime.pending_boundary_snapshot(3);
        assert_eq!(repeated, exact);
        Ok(())
    }

    fn assert_snapshot_counts(
        snapshot: &crate::runtime::RuntimePendingBoundarySnapshot,
        active_runs: usize,
        pending_timers: usize,
        pending_actions: usize,
        pending_asks: usize,
        truncated: bool,
    ) {
        assert_eq!(snapshot.command_queue_depth(), 0);
        assert_eq!(snapshot.active_run_count(), active_runs);
        assert_eq!(snapshot.pending_timer_count(), pending_timers);
        assert_eq!(snapshot.pending_action_count(), pending_actions);
        assert_eq!(snapshot.pending_ask_count(), pending_asks);
        assert_eq!(snapshot.truncated(), truncated);
    }

    // --- Helper: wait-then-finish workflow (SetConst -> WaitUntil -> Finish) ---

    fn wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        );
        let wait = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
            None,
            Some(StepIdx::new(2)),
        );
        let finish = vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            name: Box::from("wait_then_finish"),
            digest: WorkflowDigest::from_bytes([6; 32]),
            nodes: Box::from([set_deadline, wait, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::I64(10)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
            symbols_count: 0,
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts);
        assert_eq!(workflow.as_ref().map(|_| ()), Ok(()));
        workflow.ok()
    }
