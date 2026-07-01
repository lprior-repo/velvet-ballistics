
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
        assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.pending_timer_count(), 0);
    }

    // RS-110 regression: ask-timer fire must append a typed
    // RuntimeJournalEvent::AskTimedOut to the runtime journal BEFORE
    // drive_state advances the run. Replay relies on this signal to
    // distinguish an unanswered pending ask from an ask whose timeout
    // already fired; the original test only asserted `runs_failed == 1`,
    // which would also pass if the journal append were silently skipped.
    #[test]
    fn ask_timer_fire_appends_ask_timed_out_to_journal() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("ask", ask_workflow())?;
        let run = RunId::new(510);
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
        assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        let events = require_snapshot(&journal)
            .map_err(|e| e.to_string())?;
        assert!(
            events.iter().any(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::AskTimedOut { run: r, step: s }
                        if *r == run && *s == StepIdx::new(2)
                )
            }),
            "journal events should contain AskTimedOut {{ run: {run:?}, step: StepIdx(2) }}: {events:?}"
        );
        // The AskTimedOut event must be appended in the same journal pass
        // as the failure it explains — i.e. before the terminal RunFailed
        // on the same run. The step value is the Ask node id from the
        // `ask_workflow` fixture (`StepIdx::new(2)`), NOT `StepIdx::ZERO`
        // (which would be the `set_prompt` step). Asserting both the step
        // identity AND the ordering keeps this regression test focused on
        // the gap RS-110 exposed: the fix's typed journal emission with
        // the correct step provenance.
        let ask_timed_out_idx = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::AskTimedOut { run: r, step: s }
                        if *r == run && *s == StepIdx::new(2)
                )
            })
            .expect("AskTimedOut must be present (first assertion guarantees it)");
        let run_failed_idx = events
            .iter()
            .position(|event| matches!(event, RuntimeJournalEvent::RunFailed { run: r } if *r == run));
        assert!(
            run_failed_idx.map_or(true, |i| i > ask_timed_out_idx),
            "AskTimedOut must be appended before RunFailed for run {run:?}: {events:?}"
        );
        Ok(())
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
