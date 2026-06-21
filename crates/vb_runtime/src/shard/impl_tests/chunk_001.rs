    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;

    use super::{Shard, ShardCommand, ShardConfig, ShardHealth};
    use crate::shard::types::MAX_COMMAND_QUEUE_CAPACITY;
    use crate::shard::{DEFAULT_MAX_TERMINAL_RUNS, DEFAULT_TERMINAL_RUNS_TTL_TICKS};

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
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: 16,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 100_000,
        }
    }

    // =======================================================================
    // ShardConfig::new validation
    // =======================================================================

    #[test]
    fn config_new_accepts_min_valid_capacity() -> Result<(), RuntimeError> {
        let result = ShardConfig::new(1, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        let expected = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: DEFAULT_MAX_TERMINAL_RUNS,
            terminal_runs_ttl_ticks: DEFAULT_TERMINAL_RUNS_TTL_TICKS,
            max_terminal_outcomes: crate::shard::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES,
        };
        assert_eq!(result, Ok(expected));
        Ok(())
    }

    #[test]
    fn config_new_rejects_zero_capacity() -> Result<(), RuntimeError> {
        let result = ShardConfig::new(0, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: 0,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_rejects_capacity_exceeding_max() -> Result<(), RuntimeError> {
        let too_large = MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1);
        let result = ShardConfig::new(too_large, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: too_large,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_rejects_zero_max_active_runs() -> Result<(), RuntimeError> {
        let result = ShardConfig::new(1, 1, 1, 0, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
        Ok(())
    }

    #[test]
    fn config_new_accepts_max_command_queue_capacity() -> Result<(), RuntimeError> {
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
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: DEFAULT_MAX_TERMINAL_RUNS,
            terminal_runs_ttl_ticks: DEFAULT_TERMINAL_RUNS_TTL_TICKS,
            max_terminal_outcomes: crate::shard::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES,
        };
        assert_eq!(result, Ok(expected));
        Ok(())
    }

    #[test]
    fn config_new_preserves_all_fields() -> Result<(), RuntimeError> {
        let config = ShardConfig::new(64, 128, 256, 32, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            config,
            Ok(ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 128,
                step_budget_per_tick: 256,
                max_active_runs: 32,
                policy: vb_core::policy::RuntimePolicy::Relaxed,
                coalesce_window_ticks: 1,
                snapshot_interval_steps: 0,
                max_terminal_runs: DEFAULT_MAX_TERMINAL_RUNS,
                terminal_runs_ttl_ticks: DEFAULT_TERMINAL_RUNS_TTL_TICKS,
                max_terminal_outcomes: crate::shard::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES,
            })
        );
        Ok(())
    }

    // =======================================================================
    // RQ-W0-15: ShardConfig full validation (new_full constructor and
    // Shard::new struct-literal bypass closure)
    // =======================================================================

    #[test]
    fn config_new_full_accepts_all_valid_fields() -> Result<(), RuntimeError> {
        let result = ShardConfig::new_full(
            8,
            16,
            32,
            4,
            vb_core::policy::RuntimePolicy::Strict,
            5,
            100,
            16,
            86_400,
            100_000,
        );
        assert_eq!(
            result,
            Ok(ShardConfig {
                command_queue_capacity: 8,
                trace_capacity: 16,
                step_budget_per_tick: 32,
                max_active_runs: 4,
                policy: vb_core::policy::RuntimePolicy::Strict,
                coalesce_window_ticks: 5,
                snapshot_interval_steps: 100,
                max_terminal_runs: 16,
                terminal_runs_ttl_ticks: 86_400,
                max_terminal_outcomes: 100_000,
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_full_rejects_zero_coalesce_window() -> Result<(), RuntimeError> {
        let result = ShardConfig::new_full(
            8,
            16,
            32,
            4,
            vb_core::policy::RuntimePolicy::Strict,
            0,
            100,
            16,
            86_400,
            100_000,
        );
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "coalesce_window_ticks_zero",
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_full_rejects_zero_max_terminal_runs() -> Result<(), RuntimeError> {
        let result = ShardConfig::new_full(
            8,
            16,
            32,
            4,
            vb_core::policy::RuntimePolicy::Strict,
            1,
            100,
            0,
            86_400,
            100_000,
        );
        assert_eq!(result, Err(RuntimeError::LruRingCapacityZero));
        Ok(())
    }

    #[test]
    fn config_new_full_accepts_zero_snapshot_interval_as_disabled()
    -> Result<(), RuntimeError> {
        let result = ShardConfig::new_full(
            8,
            16,
            32,
            4,
            vb_core::policy::RuntimePolicy::Strict,
            1,
            0,
            16,
            86_400,
            100_000,
        );
        assert_eq!(
            result,
            Ok(ShardConfig {
                command_queue_capacity: 8,
                trace_capacity: 16,
                step_budget_per_tick: 32,
                max_active_runs: 4,
                policy: vb_core::policy::RuntimePolicy::Strict,
                coalesce_window_ticks: 1,
                snapshot_interval_steps: 0,
                max_terminal_runs: 16,
                terminal_runs_ttl_ticks: 86_400,
                max_terminal_outcomes: 100_000,
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_full_accepts_zero_ttl_ticks() -> Result<(), RuntimeError> {
        let result = ShardConfig::new_full(
            8,
            16,
            32,
            4,
            vb_core::policy::RuntimePolicy::Strict,
            1,
            100,
            16,
            0,
            100_000,
        );
        assert_eq!(
            result,
            Ok(ShardConfig {
                command_queue_capacity: 8,
                trace_capacity: 16,
                step_budget_per_tick: 32,
                max_active_runs: 4,
                policy: vb_core::policy::RuntimePolicy::Strict,
                coalesce_window_ticks: 1,
                snapshot_interval_steps: 100,
                max_terminal_runs: 16,
                terminal_runs_ttl_ticks: 0,
                max_terminal_outcomes: 100_000,
            })
        );
        Ok(())
    }

    #[test]
    fn shard_new_rejects_struct_literal_with_zero_coalesce_window() {
        let config = ShardConfig {
            command_queue_capacity: 8,
            trace_capacity: 16,
            step_budget_per_tick: 32,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Strict,
            coalesce_window_ticks: 0,
            snapshot_interval_steps: 0,
            max_terminal_runs: 16,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 16,
        };
        let result = Shard::new(config);
        assert_eq!(
            result.err(),
            Some(RuntimeError::UnsupportedOperation {
                operation: "coalesce_window_ticks_zero",
            })
        );
    }

    #[test]
    fn shard_new_rejects_struct_literal_with_zero_max_terminal_runs() {
        let config = ShardConfig {
            command_queue_capacity: 8,
            trace_capacity: 16,
            step_budget_per_tick: 32,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Strict,
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: 0,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 16,
        };
        let result = Shard::new(config);
        assert_eq!(result.err(), Some(RuntimeError::LruRingCapacityZero));
    }

    // =======================================================================
    // Shard construction
    // =======================================================================

    #[test]
    fn shard_new_creates_empty_shard() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config())?;
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.command_queue_len(), 0);
        assert_eq!(shard.is_shutting_down(), false);
        Ok(())
    }

    // =======================================================================
    // Queue operations
    // =======================================================================

    #[test]
    fn enqueue_and_capacity_tracking() -> Result<(), RuntimeError> {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            ..small_config()
        };
        let shard = Shard::new(config)?;
        assert_eq!(shard.command_queue_capacity(), 4);
        assert_eq!(shard.remaining_capacity(), 4);
        assert_eq!(shard.is_queue_full(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.remaining_capacity(), 3);
        Ok(())
    }

    #[test]
    fn queue_full_at_capacity_boundary() -> Result<(), RuntimeError> {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            ..small_config()
        };
        let shard = Shard::new(config)?;
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.is_queue_full(), true);
        assert_eq!(shard.remaining_capacity(), 0);
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
        Ok(())
    }

    // =======================================================================
    // Tick processing
    // =======================================================================

    #[test]
    fn tick_on_empty_queue_returns_true() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.tick(), Ok(true));
        Ok(())
    }

    #[test]
    fn tick_processes_shutdown_returns_false() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
        Ok(())
    }

    #[test]
    fn tick_after_shutdown_always_returns_false() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config())?;
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.tick(), Ok(false));
        Ok(())
    }

    // =======================================================================
    // drain_for_shutdown
    // =======================================================================

    #[test]
    fn drain_for_shutdown_processes_pending_commands() -> Result<(), RuntimeError> {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            ..small_config()
        };
        let mut shard = Shard::new(config)?;
        let wf = finished_workflow().ok_or(RuntimeError::QueueFull)?;
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
        Ok(())
    }

    #[test]
    fn drain_for_shutdown_on_empty_queue_hits_capacity_limit() -> Result<(), RuntimeError> {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            ..small_config()
        };
        let mut shard = Shard::new(config)?;
        assert_eq!(
            shard.drain_for_shutdown(),
            Err(RuntimeError::ShutdownInProgress)
        );
        Ok(())
    }
