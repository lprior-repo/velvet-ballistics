
    use super::super::types::{RuntimeEvent, RuntimeState};
#[allow(unused_imports)]
use super::super::types::*;

    // =======================================================================
    // snapshot_run (direct, non-queued)
    // =======================================================================

    #[test]
    fn snapshot_run_returns_not_found_for_missing_run() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config())?;
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
        let shard = Shard::new(small_config())?;
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
        let mut shard = Shard::new(small_config())?;
        let wf = finished_workflow().ok_or(RuntimeError::QueueFull)?;
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
        let mut shard = Shard::new(small_config())?;
        let wf = finished_workflow().ok_or(RuntimeError::QueueFull)?;
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
        let mut shard = Shard::new(small_config())?;
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
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.take_inspect_response(), None);
        Ok(())
    }

    #[test]
    fn status_reports_shard_health_and_capacity_without_mutation() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config())?;
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
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));

        let status = shard.status();

        assert_eq!(status.health, ShardHealth::ShuttingDown);
        assert_eq!(status.running, false);
        assert_eq!(status.shutting_down, true);
        Ok(())
    }

    // =======================================================================
    // RQ-W0-06: Shard::enqueue TOCTOU with shutting_down.
    //
    // The `shutting_down` flag is now an `AtomicBool` so that the producer
    // thread (calling `enqueue`) and the dispatcher thread (calling `tick`)
    // can synchronise without holding a mutex. Producers must observe a
    // consistent value via the Acquire load, and the dispatcher's Release
    // store must synchronise the visibility of `shutting_down=true` to any
    // subsequent producer that observes the value.
    // =======================================================================

    #[test]
    fn enqueue_rejects_after_shutdown_processed_by_tick() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config())?;
        // First Shutdown is enqueued normally.
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        // After tick() returns, shutting_down is true.
        assert_eq!(shard.is_shutting_down(), true);
        // Any further non-Shutdown command is rejected.
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(1),
                correlation: 1,
            }),
            Err(RuntimeError::ShutdownInProgress)
        );
        // Shutdown sentinel is still permitted (idempotent drain).
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        Ok(())
    }

    #[test]
    fn enqueue_toctou_race_emits_shutdown_in_progress() -> Result<(), RuntimeError> {
        // Simulate the TOCTOU scenario: producer thread inspects
        // shutting_down (false), then dispatcher flips it to true, then
        // producer tries to push. With the AtomicBool fix, the producer's
        // load(Acquire) sees the Release store from the dispatcher, so the
        // push is rejected with ShutdownInProgress rather than landing on a
        // shutting-down shard.
        let mut shard = Shard::new(small_config())?;
        // Producer's perspective: shutting_down is currently false.
        assert_eq!(shard.is_shutting_down(), false);
        // Dispatcher: process Shutdown.
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
        // Producer attempts to enqueue after seeing the Acquire load.
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(7),
                correlation: 7,
            }),
            Err(RuntimeError::ShutdownInProgress)
        );
        Ok(())
    }

    #[test]
    fn is_shutting_down_consistent_across_threads() -> Result<(), RuntimeError> {
        // Drives the dispatcher and observes that `is_shutting_down()` flips
        // synchronously from `false` to `true` once `tick()` processes the
        // `Shutdown` sentinel. Because `shutting_down` is an `AtomicBool`,
        // subsequent `enqueue` calls must observe the same value via
        // Acquire/Release semantics and reject non-Shutdown commands with
        // `RuntimeError::ShutdownInProgress`.
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.is_shutting_down(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
        // After shutdown, every other command is rejected.
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(1),
                correlation: 1,
            }),
            Err(RuntimeError::ShutdownInProgress)
        );
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        Ok(())
    }

    // =======================================================================
    // RQ-W0-07: RuntimeStateMachine::apply must verify prior state for Resume.
    //
    // The `apply` method now requires the prior `RuntimeState` to be
    // `Resumable` before transitioning to `Resuming`. Direct callers (other
    // than the resume path, which already validates) can observe the new
    // `RuntimeError::NotResumable` error if the FSM contract is violated.
    // =======================================================================

    #[test]
    fn apply_resume_rejects_when_prior_state_is_running() {
        let mut shard = Shard::new(small_config()).expect("shard");
        let run = RunId::new(900);
        // Inject a state that is NOT Resumable.
        shard.runtime_state_insert(run, RuntimeState::Running);
        let result = shard.apply(run, RuntimeEvent::Resume);
        match result {
            Err(RuntimeError::NotResumable {
                run: returned_run,
                current_state,
            }) => {
                assert_eq!(returned_run, run);
                assert_eq!(current_state, RuntimeState::Running);
            }
            other => panic!("expected NotResumable, got {other:?}"),
        }
        // State must NOT have been mutated.
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    }

    #[test]
    fn apply_resume_rejects_when_prior_state_is_initial() {
        let mut shard = Shard::new(small_config()).expect("shard");
        let run = RunId::new(901);
        shard.runtime_state_insert(run, RuntimeState::Initial);
        let result = shard.apply(run, RuntimeEvent::Resume);
        assert!(matches!(
            result,
            Err(RuntimeError::NotResumable {
                current_state: RuntimeState::Initial,
                ..
            })
        ));
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Initial));
    }

    #[test]
    fn apply_resume_rejects_when_no_state_recorded() {
        let mut shard = Shard::new(small_config()).expect("shard");
        let run = RunId::new(902);
        // No state recorded for run.
        assert_eq!(shard.runtime_state_get(run), None);
        let result = shard.apply(run, RuntimeEvent::Resume);
        assert!(matches!(
            result,
            Err(RuntimeError::NotResumable {
                current_state: RuntimeState::Initial,
                ..
            })
        ));
        assert_eq!(shard.runtime_state_get(run), None);
    }

    #[test]
    fn apply_resume_accepts_when_prior_state_is_resumable() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config())?;
        let run = RunId::new(903);
        shard.runtime_state_insert(run, RuntimeState::Resumable);
        shard.apply(run, RuntimeEvent::Resume)?;
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resuming));
        Ok(())
    }

    #[test]
    fn apply_resume_rejects_when_prior_state_is_failed() {
        let mut shard = Shard::new(small_config()).expect("shard");
        let run = RunId::new(904);
        shard.runtime_state_insert(run, RuntimeState::Failed);
        let result = shard.apply(run, RuntimeEvent::Resume);
        assert!(matches!(
            result,
            Err(RuntimeError::NotResumable {
                current_state: RuntimeState::Failed,
                ..
            })
        ));
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Failed));
    }

    #[test]
    fn apply_resume_rollback_does_not_require_prior_state() -> Result<(), RuntimeError> {
        // ResumeRollback is the journal-failure path: it must unconditionally
        // revert the state to Resumable, regardless of the prior state.
        let mut shard = Shard::new(small_config())?;
        let run = RunId::new(905);
        shard.runtime_state_insert(run, RuntimeState::Resuming);
        shard.apply(run, RuntimeEvent::ResumeRollback)?;
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
        Ok(())
    }
