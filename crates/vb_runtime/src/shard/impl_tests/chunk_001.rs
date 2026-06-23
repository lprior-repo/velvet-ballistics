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
            Err(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::CommandQueueCapacityExceeded {
                    capacity: 0,
                    max: MAX_COMMAND_QUEUE_CAPACITY,
                }],
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
            Err(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::CommandQueueCapacityExceeded {
                    capacity: too_large,
                    max: MAX_COMMAND_QUEUE_CAPACITY,
                }],
            })
        );
        Ok(())
    }

    #[test]
    fn config_new_rejects_zero_max_active_runs() -> Result<(), RuntimeError> {
        let result = ShardConfig::new(1, 1, 1, 0, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::ActiveRunCapacityZero],
            })
        );
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
            Err(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::UnsupportedOperation {
                    operation: "coalesce_window_ticks_zero",
                }],
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
        assert_eq!(
            result,
            Err(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::LruRingCapacityZero],
            })
        );
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
            Some(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::UnsupportedOperation {
                    operation: "coalesce_window_ticks_zero",
                }],
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
        assert_eq!(
            result.err(),
            Some(RuntimeError::ConfigInvalid {
                errors: vec![RuntimeError::LruRingCapacityZero],
            })
        );
    }

    // RS-217: every invalid field is reported, not just the first.
    // `validate()` must aggregate all field failures into a single
    // `RuntimeError::ConfigInvalid` so operators see the full report.
    #[test]
    fn shard_config_validate_aggregates_all_invalid_fields() {
        let config = ShardConfig {
            // invalid: command_queue_capacity == 0
            command_queue_capacity: 0,
            // invalid: trace_capacity == 0
            trace_capacity: 0,
            // invalid: step_budget_per_tick == 0
            step_budget_per_tick: 0,
            // invalid: max_active_runs == 0
            max_active_runs: 0,
            policy: vb_core::policy::RuntimePolicy::Strict,
            // invalid: coalesce_window_ticks == 0
            coalesce_window_ticks: 0,
            snapshot_interval_steps: 0,
            // invalid: max_terminal_runs == 0
            max_terminal_runs: 0,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 16,
        };
        let result = config.validate();
        let expected = Err(RuntimeError::ConfigInvalid {
            errors: vec![
                RuntimeError::CommandQueueCapacityExceeded {
                    capacity: 0,
                    max: MAX_COMMAND_QUEUE_CAPACITY,
                },
                RuntimeError::UnsupportedOperation {
                    operation: "trace_capacity_zero",
                },
                RuntimeError::UnsupportedOperation {
                    operation: "step_budget_per_tick_zero",
                },
                RuntimeError::ActiveRunCapacityZero,
                RuntimeError::UnsupportedOperation {
                    operation: "coalesce_window_ticks_zero",
                },
                RuntimeError::LruRingCapacityZero,
            ],
        });
        assert_eq!(result, expected);
    }

    // RS-217: `ShardConfig::new_full` with multiple invalid inputs must
    // surface every field failure, in declaration order, inside a single
    // `RuntimeError::ConfigInvalid` instead of returning on the first one.
    #[test]
    fn config_new_full_aggregates_multiple_invalid_fields() {
        let result = ShardConfig::new_full(
            0,    // invalid command_queue_capacity
            0,    // invalid trace_capacity
            0,    // invalid step_budget_per_tick
            0,    // invalid max_active_runs
            vb_core::policy::RuntimePolicy::Strict,
            0,    // invalid coalesce_window_ticks
            100,  // valid snapshot_interval_steps
            0,    // invalid max_terminal_runs
            86_400,
            100_000,
        );
        let expected = Err(RuntimeError::ConfigInvalid {
            errors: vec![
                RuntimeError::CommandQueueCapacityExceeded {
                    capacity: 0,
                    max: MAX_COMMAND_QUEUE_CAPACITY,
                },
                RuntimeError::UnsupportedOperation {
                    operation: "trace_capacity_zero",
                },
                RuntimeError::UnsupportedOperation {
                    operation: "step_budget_per_tick_zero",
                },
                RuntimeError::ActiveRunCapacityZero,
                RuntimeError::UnsupportedOperation {
                    operation: "coalesce_window_ticks_zero",
                },
                RuntimeError::LruRingCapacityZero,
            ],
        });
        assert_eq!(result, expected);
    }

    // RS-217: Display impl must show every inner error so operators see
    // the complete report rather than only the first failure.
    #[test]
    fn shard_config_validate_display_lists_all_errors() {
        let config = ShardConfig {
            command_queue_capacity: 0,
            trace_capacity: 0,
            step_budget_per_tick: 1,
            max_active_runs: 0,
            policy: vb_core::policy::RuntimePolicy::Strict,
            coalesce_window_ticks: 0,
            snapshot_interval_steps: 0,
            max_terminal_runs: 0,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 16,
        };
        let err = config.validate().unwrap_err();
        let display = format!("{err}");
        assert!(
            display.contains("shard config invalid"),
            "display must lead with shard config invalid marker: {display}"
        );
        // Confirm the count of inner errors is reported.
        assert!(
            display.contains("5 field error"),
            "display must report the count of inner errors: {display}"
        );
        // Confirm the index markers for every entry are present so
        // operators can pinpoint which fields are invalid.
        assert!(
            display.contains("[0]"),
            "display must include index [0]: {display}"
        );
        assert!(
            display.contains("[1]"),
            "display must include index [1]: {display}"
        );
        assert!(
            display.contains("[2]"),
            "display must include index [2]: {display}"
        );
        assert!(
            display.contains("[3]"),
            "display must include index [3]: {display}"
        );
        assert!(
            display.contains("[4]"),
            "display must include index [4]: {display}"
        );
        // Verify each invalid field's identifier is reachable in the
        // display output for the entries that have a static message.
        assert!(
            display.contains("command queue capacity"),
            "display must mention command queue capacity failure: {display}"
        );
        assert!(
            display.contains("trace_capacity_zero"),
            "display must mention trace_capacity_zero: {display}"
        );
        assert!(
            display.contains("active run capacity cannot be zero"),
            "display must mention active run capacity zero: {display}"
        );
        assert!(
            display.contains("coalesce_window_ticks_zero"),
            "display must mention coalesce_window_ticks_zero: {display}"
        );
    }

    // RS-217: equality must compare the aggregated error list, not just
    // the variant tag.
    #[test]
    fn shard_config_config_invalid_equality_compares_inner_errors() {
        let single = RuntimeError::ConfigInvalid {
            errors: vec![RuntimeError::LruRingCapacityZero],
        };
        let single_same = RuntimeError::ConfigInvalid {
            errors: vec![RuntimeError::LruRingCapacityZero],
        };
        let single_other = RuntimeError::ConfigInvalid {
            errors: vec![RuntimeError::ActiveRunCapacityZero],
        };
        let multi = RuntimeError::ConfigInvalid {
            errors: vec![
                RuntimeError::LruRingCapacityZero,
                RuntimeError::ActiveRunCapacityZero,
            ],
        };
        assert_eq!(single, single_same);
        assert_ne!(single, single_other);
        assert_ne!(single, multi);
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

    // RA-014 regression: lock_admission must recover the mutex guard on poison
    // instead of permanently bricking the shard. Pre-fix, a poisoned
    // admission_lock caused every subsequent submit to return
    // `Err(JournalPoisoned)` forever (the mutex never un-poisons itself).
    // Post-fix, the recovered guard is returned as `Ok`, keeping the submit
    // path servicable.
    //
    // We poison the mutex by panicking while holding the guard (caught via
    // catch_unwind so the test process keeps running), then assert
    // lock_admission yields `Ok`. Tests may use panic/catch_unwind; production
    // code never panics, so this exercises a defense-in-depth recovery path.
    #[test]
    fn lock_admission_recovers_guard_after_mutex_poison() -> Result<(), RuntimeError> {
        let shard = Shard::new(small_config())?;

        // Sanity: before poisoning, lock_admission returns Ok.
        {
            let guard = shard.lock_admission();
            assert!(guard.is_ok(), "pre-poison lock_admission must succeed");
        }

        // Poison admission_lock by panicking while holding the guard. The guard
        // is dropped during unwinding, which marks the mutex poisoned.
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = shard.admission_lock.lock().expect("test helper lock");
            panic!("intentional test poison for admission_lock");
        }));
        assert!(
            panic_result.is_err(),
            "test helper must have panicked to poison the mutex"
        );

        // Post-fix: lock_admission must recover and return Ok, NOT
        // Err(JournalPoisoned). This is the regression assertion.
        let recovered = shard.lock_admission();
        assert!(
            recovered.is_ok(),
            "lock_admission must recover on poison (RA-014), got {:?}",
            recovered.as_ref().err()
        );

        // And the recovered guard is a real, held guard — dropping it must not
        // panic and must release the lock (a second acquisition succeeds).
        drop(recovered);
        let again = shard.lock_admission();
        assert!(again.is_ok(), "lock_admission must remain usable after recovery");
        Ok(())
    }
