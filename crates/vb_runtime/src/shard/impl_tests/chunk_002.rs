
    // =======================================================================
    // snapshot_run (direct, non-queued)
    // =======================================================================

    #[test]
    fn snapshot_run_returns_not_found_for_missing_run() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config());
        let response = shard.snapshot_run(RunId::new(999), 42);
        match response {
            super::InspectResponse::NotFound { run, correlation } => {
                assert_eq!(run, RunId::new(999));
                assert_eq!(correlation, 42);
            }
            other => {
                assert_eq!(
                    other,
                    super::InspectResponse::NotFound {
                        run: RunId::new(999),
                        correlation: 42,
                    }
                );
            }
        }
        Ok(())
    }

    fn submit_finished_run(shard: &mut Shard, run: RunId) {
        let Some(wf) = finished_workflow() else {
            assert_eq!(None::<()>, Some(()));
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    // =======================================================================
    // Frame pool metrics
    // =======================================================================

    #[test]
    fn frame_pool_metrics_zero_initially() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config());
        let (free, total) = shard.frame_pool_metrics();
        assert_eq!(free, 0);
        assert_eq!(total, 0);
        Ok(())
    }

    // =======================================================================
    // Boundary conditions
    // =======================================================================

    #[test]
    fn shard_with_run_id_zero() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(0),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        Ok(())
    }

    #[test]
    fn shard_with_max_run_id() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(u64::MAX),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        Ok(())
    }

    #[test]
    fn shard_handles_multiple_sequential_finished_runs() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        submit_finished_run(&mut shard, RunId::new(0));
        submit_finished_run(&mut shard, RunId::new(1));
        submit_finished_run(&mut shard, RunId::new(2));
        submit_finished_run(&mut shard, RunId::new(3));
        assert_eq!(shard.counters().snapshot().runs_completed, 4);
        assert_eq!(shard.counters().snapshot().runs_submitted, 4);
        Ok(())
    }

    #[test]
    fn take_inspect_response_returns_none_when_none_pending() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.take_inspect_response(), None);
        Ok(())
    }

    #[test]
    fn status_reports_shard_health_and_capacity_without_mutation() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        let before_len = shard.command_queue_len();

        let status = shard.status();

        assert_eq!(status.health, ShardHealth::Running);
        assert_eq!(status.running, true);
        assert_eq!(status.shutting_down, false);
        assert_eq!(status.command_queue_depth, 1);
        assert_eq!(status.command_queue_capacity, 16);
        assert_eq!(status.active_runs, 0);
        assert_eq!(status.max_active_runs, 4);
        assert_eq!(status.trace_capacity, 16);
        assert_eq!(status.trace_dropped, 0);
        assert_eq!(status.step_budget_per_tick, 4);
        assert_eq!(
            status.runtime_policy,
            vb_core::policy::RuntimePolicy::Relaxed
        );
        assert_eq!(shard.command_queue_len(), before_len);
        Ok(())
    }

    #[test]
    fn status_reports_shutting_down_after_shutdown_tick() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));

        let status = shard.status();

        assert_eq!(status.health, ShardHealth::ShuttingDown);
        assert_eq!(status.running, false);
        assert_eq!(status.shutting_down, true);
        Ok(())
    }
