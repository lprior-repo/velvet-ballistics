    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;

    use super::{MAX_COMMAND_QUEUE_CAPACITY, Shard, ShardCommand, ShardConfig, ShardHealth};

    fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finished"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    // =======================================================================
    // ShardConfig::new validation
    // =======================================================================

    #[test]
    fn config_new_accepts_min_valid_capacity() {
        let result = ShardConfig::new(1, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        let expected = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn config_new_rejects_zero_capacity() {
        let result = ShardConfig::new(0, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: 0,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
    }

    #[test]
    fn config_new_rejects_capacity_exceeding_max() {
        let too_large = MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1);
        let result = ShardConfig::new(too_large, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: too_large,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
    }

    #[test]
    fn config_new_rejects_zero_max_active_runs() {
        let result = ShardConfig::new(1, 1, 1, 0, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
    }

    #[test]
    fn config_new_accepts_max_command_queue_capacity() {
        let result = ShardConfig::new(
            MAX_COMMAND_QUEUE_CAPACITY,
            1,
            1,
            1,
            vb_core::policy::RuntimePolicy::Relaxed,
        );
        let expected = ShardConfig {
            command_queue_capacity: MAX_COMMAND_QUEUE_CAPACITY,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn config_new_preserves_all_fields() {
        let config = ShardConfig::new(64, 128, 256, 32, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            config,
            Ok(ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 128,
                step_budget_per_tick: 256,
                max_active_runs: 32,
                policy: vb_core::policy::RuntimePolicy::Relaxed,
            })
        );
    }

    // =======================================================================
    // Shard construction
    // =======================================================================

    #[test]
    fn shard_new_creates_empty_shard() {
        let shard = Shard::new(small_config());
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.command_queue_len(), 0);
        assert_eq!(shard.is_shutting_down(), false);
    }

    // =======================================================================
    // Queue operations
    // =======================================================================

    #[test]
    fn enqueue_and_capacity_tracking() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.command_queue_capacity(), 4);
        assert_eq!(shard.remaining_capacity(), 4);
        assert_eq!(shard.is_queue_full(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.remaining_capacity(), 3);
    }

    #[test]
    fn queue_full_at_capacity_boundary() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.is_queue_full(), true);
        assert_eq!(shard.remaining_capacity(), 0);
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    // =======================================================================
    // Tick processing
    // =======================================================================

    #[test]
    fn tick_on_empty_queue_returns_true() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn tick_processes_shutdown_returns_false() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
    }

    #[test]
    fn tick_after_shutdown_always_returns_false() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.tick(), Ok(false));
    }

    // =======================================================================
    // drain_for_shutdown
    // =======================================================================

    #[test]
    fn drain_for_shutdown_processes_pending_commands() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.drain_for_shutdown(), Ok(()));
        assert_eq!(shard.is_shutting_down(), true);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn drain_for_shutdown_on_empty_queue_hits_capacity_limit() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.drain_for_shutdown(),
            Err(RuntimeError::ShutdownInProgress)
        );
    }
