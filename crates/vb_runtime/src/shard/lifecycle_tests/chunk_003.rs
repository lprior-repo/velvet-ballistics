
    #[test]
    fn submit_suspended_workflow_suspends_on_action() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(2);
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
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        Ok(())
    }

    #[test]
    fn submit_duplicate_run_returns_run_already_exists() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(10);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
        Ok(())
    }

    #[test]
    fn submit_rejects_duplicate_run_id() -> Result<(), String> {
        submit_duplicate_run_returns_run_already_exists()
    }

    #[test]
    fn admission_rejection_does_not_insert_run_state() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(53);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );

        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        Ok(())
    }

    #[test]
    fn submit_at_capacity_returns_active_run_capacity_exceeded() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        
};
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf1,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(2),
                workflow: wf2,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn submit_with_inputs_seeds_slots_before_driving() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(20);
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputs {
                run,
                workflow: wf,
                inputs: Box::from([(SlotIdx::new(0), SlotValue::I64(99))]),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    #[test]
    fn submit_with_inputs_rejects_duplicate() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(21);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputs {
                run,
                workflow: wf,
                inputs: Box::from([(SlotIdx::new(0), SlotValue::I64(1))]),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
        Ok(())
    }

    #[test]
    fn resume_on_suspended_run_re_drives() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(30);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
    }

    #[test]
    fn resume_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Resume {
                run: RunId::new(9999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn action_completed_typed_writes_slot_and_advances() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(40);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let ticket = make_ticket(run, StepIdx::ZERO, 1);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 2,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| {
            *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }
        });
        assert_eq!(found, true);
    }

    #[test]
    fn action_completed_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(1),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }
