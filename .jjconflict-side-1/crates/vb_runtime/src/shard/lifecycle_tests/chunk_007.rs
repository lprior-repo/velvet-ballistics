
    #[test]
    fn ask_timer_fire_fails_run_when_no_answer() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = ask_workflow() else {
            return;
        };
        let run = RunId::new(500);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.pending_timer_count(), 0);
    }

    #[test]
    fn multiple_submits_fill_to_capacity_then_reject() -> Result<(), String> {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 2,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        submit_run(
            &mut shard,
            RunId::new(0),
            require_workflow("suspended", suspended_workflow())?,
        );
        submit_run(
            &mut shard,
            RunId::new(1),
            require_workflow("suspended", suspended_workflow())?,
        );
        assert_eq!(shard.active_run_count(), 2);
        let wf = require_workflow("suspended", suspended_workflow())?;
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(99),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
        );
        Ok(())
    }

    #[test]
    fn red_ask_answer_secret_redaction() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("ask", ask_workflow())?;
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
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Symbol(vb_core::ids::SymbolId::new(9999)),
            taint: Taint::Secret,
            encoded_len: 0,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::SecretResultNotAllowed));
        Ok(())
    }

    #[test]
    fn red_ask_answer_payload_size_one_byte_over() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("ask", ask_workflow())?;
        let max_size = wf.resource_contract().max_ipc_payload_bytes;
        let run = RunId::new(12);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(encoded_len) = max_size.checked_add(1) else {
            return Err(String::from("max_ipc_payload_bytes overflowed"));
        };
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Blob(vb_core::ids::BlobId::new(u64::MAX)),
            taint: Taint::Clean,
            encoded_len,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::IpcPayloadSizeExceeded {
                size: encoded_len,
                max: max_size,
            })
        );
        Ok(())
    }
