
    #[derive(Clone, Copy)]
    enum TerminalCommandKind {
        Cancel,
        Kill,
    }

    impl TerminalCommandKind {
        fn command(self, run: RunId) -> ShardCommand {
            match self {
                Self::Cancel => ShardCommand::Cancel { run, reason: None },
                Self::Kill => ShardCommand::Kill { run, reason: None },
            }
        }

        fn first_filler_run(self) -> u64 {
            match self {
                Self::Cancel => 51_011,
                Self::Kill => 51_021,
            }
        }

        fn run(self) -> RunId {
            match self {
                Self::Cancel => RunId::new(51_010),
                Self::Kill => RunId::new(51_020),
            }
        }
    }

    fn assert_terminal_append_failure_preserves_action_suspension(
        kind: TerminalCommandKind,
    ) -> Result<(), String> {
        const QUEUE_CAPACITY: usize = 8;
        const QUEUE_BATCH_SIZE: usize = 8;
        let (_journal, queue, adapter, shared) = queued_journal_fixture(
            "vb-runtime-terminal-action-atomic-",
            QUEUE_CAPACITY,
            QUEUE_BATCH_SIZE,
        )?;
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let run = kind.run();
        submit_run(&mut shard, run, require_workflow("suspended", suspended_workflow())?);
        let timer = guard_test_timer(PendingTimerKind::Wait);
        assert_eq!(shard.pending_timer_insert(run, timer), Ok(None));
        let filled = fill_queue_to_one_free_slot(&queue, QUEUE_CAPACITY, kind.first_filler_run())?;
        let state_before = shard.run_state_get(run).ok_or("run must be active")?.clone();
        let pending_before = shard.pending_action_get(run);
        let counters_before = shard.counters().snapshot();
        let runtime_before = shard.runtime_state_get(run);
        let active_before = shard.active_run_count();
        let seq_before = shard.journal_sequences.get(&run).copied();
        let trace_before = shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity());

        assert_eq!(shard.enqueue(kind.command(run)), Ok(()));
        let result = shard.tick();
        assert!(matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ));
        assert_eq!(shard.run_state_get(run), Some(&state_before));
        assert_eq!(shard.pending_action_get(run), pending_before);
        assert_eq!(shard.pending_timer_get(run), Some(timer));
        assert_eq!(shard.runtime_state_get(run), runtime_before);
        assert_eq!(shard.active_run_count(), active_before);
        assert!(!shard.checked_out_run_contains(run));
        assert!(!shard.terminal_runs_contains(run));
        assert_eq!(shard.counters().snapshot(), counters_before);
        assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
        assert_eq!(shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity()), trace_before);
        assert_eq!(queue.pending_profile_counts().map_err(|error| error.to_string())?.journaled, filled);

        let flushed = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
        assert_eq!(flushed.written, flushed.drained);
        assert_eq!(shard.enqueue(kind.command(run)), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_action_get(run), None);
        assert_eq!(shard.pending_timer_get(run), None);
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.active_run_count(), 0);
        Ok(())
    }

    #[test]
    fn cancel_append_failure_preserves_action_suspension() -> Result<(), String> {
        assert_terminal_append_failure_preserves_action_suspension(TerminalCommandKind::Cancel)
    }

    #[test]
    fn kill_append_failure_preserves_action_suspension() -> Result<(), String> {
        assert_terminal_append_failure_preserves_action_suspension(TerminalCommandKind::Kill)
    }

    fn valid_output() -> ActionOutputReady {
        ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 2,
        }
    }

    fn assert_no_completion_mutation(
        shard: &Shard,
        run: RunId,
        state: &super::super::types::RunState,
        trace: &[TraceEvent],
        seq: Option<vb_storage::EventSeq>,
    ) {
        assert_eq!(shard.run_state_get(run), Some(state));
        assert_eq!(shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity()), trace);
        assert_eq!(shard.journal_sequences.get(&run).copied(), seq);
        assert_eq!(shard.counters().snapshot().runs_completed, 0);
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    #[test]
    fn action_completion_rejects_missing_and_mismatched_pending_action() -> Result<(), String> {
        let mut missing = Shard::new(small_config());
        let run_missing = RunId::new(51_030);
        submit_run(&mut missing, run_missing, require_workflow("suspended", suspended_workflow())?);
        let ticket = pending_ticket(&missing, run_missing, StepIdx::ZERO, 1);
        let _removed = missing.pending_action_remove(run_missing);
        let state = missing.run_state_get(run_missing).ok_or("missing case active")?.clone();
        let trace = missing.trace_ring().snapshot_for_run(run_missing, missing.trace_ring().capacity());
        let seq = missing.journal_sequences.get(&run_missing).copied();
        assert_eq!(missing.enqueue(ShardCommand::ActionCompleted { ticket, output: valid_output() }), Ok(()));
        assert_eq!(missing.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(missing.pending_action_get(run_missing), None);
        assert_no_completion_mutation(&missing, run_missing, &state, &trace, seq);

        let mut mismatched = Shard::new(small_config());
        let run_mismatch = RunId::new(51_031);
        submit_run(&mut mismatched, run_mismatch, require_workflow("suspended", suspended_workflow())?);
        let pending = mismatched.pending_action_get(run_mismatch);
        let mut wrong = pending_ticket(&mismatched, run_mismatch, StepIdx::ZERO, 1);
        wrong.capacity = wrong.capacity.saturating_add(1);
        let state = mismatched.run_state_get(run_mismatch).ok_or("mismatch active")?.clone();
        let trace = mismatched.trace_ring().snapshot_for_run(run_mismatch, mismatched.trace_ring().capacity());
        let seq = mismatched.journal_sequences.get(&run_mismatch).copied();
        assert_eq!(mismatched.enqueue(ShardCommand::ActionCompleted { ticket: wrong, output: valid_output() }), Ok(()));
        assert_eq!(mismatched.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(mismatched.pending_action_get(run_mismatch), pending);
        assert_no_completion_mutation(&mismatched, run_mismatch, &state, &trace, seq);
        Ok(())
    }

    #[test]
    fn legacy_action_completion_rejects_missing_and_mismatched_pending_action(
    ) -> Result<(), String> {
        let mut missing = Shard::new(small_config());
        let run_missing = RunId::new(51_034);
        submit_run(&mut missing, run_missing, require_workflow("suspended", suspended_workflow())?);
        let _removed = missing.pending_action_remove(run_missing);
        let state = missing.run_state_get(run_missing).ok_or("legacy missing active")?.clone();
        let trace = missing.trace_ring().snapshot_for_run(run_missing, missing.trace_ring().capacity());
        let seq = missing.journal_sequences.get(&run_missing).copied();
        assert_eq!(missing.enqueue(ShardCommand::ActionCompletedLegacy { run: run_missing, step: StepIdx::ZERO }), Ok(()));
        assert_eq!(missing.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(missing.pending_action_get(run_missing), None);
        assert_no_completion_mutation(&missing, run_missing, &state, &trace, seq);

        let mut mismatched = Shard::new(small_config());
        let run_mismatch = RunId::new(51_035);
        submit_run(&mut mismatched, run_mismatch, require_workflow("suspended", suspended_workflow())?);
        let pending = mismatched.pending_action_get(run_mismatch);
        let state = mismatched.run_state_get(run_mismatch).ok_or("legacy mismatch active")?.clone();
        let trace = mismatched
            .trace_ring()
            .snapshot_for_run(run_mismatch, mismatched.trace_ring().capacity());
        let seq = mismatched.journal_sequences.get(&run_mismatch).copied();
        assert_eq!(mismatched.enqueue(ShardCommand::ActionCompletedLegacy { run: run_mismatch, step: StepIdx::new(1) }), Ok(()));
        assert_eq!(mismatched.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(mismatched.pending_action_get(run_mismatch), pending);
        assert_no_completion_mutation(&mismatched, run_mismatch, &state, &trace, seq);
        Ok(())
    }

    #[test]
    fn action_failure_rejects_missing_and_mismatched_pending_action() -> Result<(), String> {
        let mut missing = Shard::new(small_config());
        let run_missing = RunId::new(51_032);
        submit_run(&mut missing, run_missing, require_workflow("suspended", suspended_workflow())?);
        let ticket = pending_ticket(&missing, run_missing, StepIdx::ZERO, 1);
        let _removed = missing.pending_action_remove(run_missing);
        let state = missing.run_state_get(run_missing).ok_or("missing failure active")?.clone();
        let trace = missing.trace_ring().snapshot_for_run(run_missing, missing.trace_ring().capacity());
        let seq = missing.journal_sequences.get(&run_missing).copied();
        assert_eq!(missing.enqueue(ShardCommand::ActionFailed { ticket, failure: non_retryable_failure() }), Ok(()));
        assert_eq!(missing.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(missing.pending_action_get(run_missing), None);
        assert_no_completion_mutation(&missing, run_missing, &state, &trace, seq);

        let mut mismatched = Shard::new(small_config());
        let run_mismatch = RunId::new(51_033);
        submit_run(&mut mismatched, run_mismatch, require_workflow("suspended", suspended_workflow())?);
        let pending = mismatched.pending_action_get(run_mismatch);
        let mut wrong = pending_ticket(&mismatched, run_mismatch, StepIdx::ZERO, 1);
        wrong.capacity = wrong.capacity.saturating_add(1);
        let state = mismatched.run_state_get(run_mismatch).ok_or("mismatch failure active")?.clone();
        let trace = mismatched.trace_ring().snapshot_for_run(run_mismatch, mismatched.trace_ring().capacity());
        let seq = mismatched.journal_sequences.get(&run_mismatch).copied();
        assert_eq!(mismatched.enqueue(ShardCommand::ActionFailed { ticket: wrong, failure: non_retryable_failure() }), Ok(()));
        assert_eq!(mismatched.tick(), Err(RuntimeError::InvalidActionCompletion));
        assert_eq!(mismatched.pending_action_get(run_mismatch), pending);
        assert_no_completion_mutation(&mismatched, run_mismatch, &state, &trace, seq);
        Ok(())
    }
