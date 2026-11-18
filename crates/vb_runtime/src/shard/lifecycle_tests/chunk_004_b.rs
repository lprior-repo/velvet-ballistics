#[test]
fn handle_action_completion_journal_first() -> Result<(), String> {
    // Regression test for RS-010 (non-legacy path):
    // handle_action_completion must append the ActionCompletedEnvelope journal
    // event BEFORE mutating the run frame. On journal append failure, the
    // frame, action_attempts, counters, and journal sequence MUST remain
    // unchanged so a failed append does not diverge memory-only state from
    // durable evidence.
    struct EnvelopeFailsJournal;
    impl crate::journal::RuntimeJournal for EnvelopeFailsJournal {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }

        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            // Reject the non-legacy completion's ActionCompletedEnvelope append
            // with a typed JournalError so the shard surfaces a
            // StorageJournalAppend failure.
            if matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }))
            {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(EnvelopeFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("suspended_workflow", suspended_workflow())?;
    let run = RunId::new(50_020);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Capture frame invariants BEFORE the failing completion.
    let frame_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .frame
        .clone();
    let pc_before = frame_before.pc();
    let step_state_before = frame_before.step_state(StepIdx::ZERO);
    let executed_before = frame_before.executed();
    let attempts_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .action_attempts
        .clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());

    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    // The typed StorageJournalAppend error MUST surface (no swallowed map_err).
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );

    // Frame MUST remain unchanged — the journal-first ordering guarantees
    // that a failed append does not diverge the frame from the journal.
    let frame_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected completion")
        .frame
        .clone();
    assert_eq!(frame_after.pc(), pc_before, "pc must be unchanged");
    assert_eq!(
        frame_after.step_state(StepIdx::ZERO),
        step_state_before,
        "step state must be unchanged"
    );
    assert_eq!(
        frame_after.executed(),
        executed_before,
        "executed count must be unchanged"
    );
    assert_eq!(frame_after, frame_before, "frame must be byte-equal");
    // action_attempts, counters, journal sequence, and trace MUST remain
    // unchanged.
    let attempts_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected completion")
        .action_attempts
        .clone();
    assert_eq!(attempts_after, attempts_before, "action_attempts unchanged");
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
    Ok(())
}

#[test]
fn handle_action_failure_journal_first() {
    // Regression test for RS-010 (failure path):
    // handle_action_failure must append the ActionFailed journal event BEFORE
    // mutating the run frame. On journal append failure, the frame,
    // action_attempts, counters, and journal sequence MUST remain unchanged
    // so a failed append does not diverge memory-only state from durable
    // evidence. The retry-policy and error-handler decisions in
    // apply_action_failure_to_state are part of the same critical section and
    // must not be persisted to memory without durable evidence.
    struct ActionFailedFailsJournal;
    impl crate::journal::RuntimeJournal for ActionFailedFailsJournal {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }

        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            // Reject the failure path's ActionFailed append with a typed
            // JournalError so the shard surfaces a StorageJournalAppend
            // failure.
            if matches!(event, RuntimeJournalEvent::ActionFailed { .. }) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::ActionFailed { .. }))
            {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(ActionFailedFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50_030);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Capture frame invariants BEFORE the failing failure-path.
    let frame_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .frame
        .clone();
    let pc_before = frame_before.pc();
    let step_state_before = frame_before.step_state(StepIdx::ZERO);
    let executed_before = frame_before.executed();
    let attempts_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .action_attempts
        .clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // The typed StorageJournalAppend error MUST surface (no swallowed map_err).
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );

    // Frame MUST remain unchanged — the journal-first ordering guarantees
    // that a failed append does not diverge the frame from the journal.
    let frame_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected failure")
        .frame
        .clone();
    assert_eq!(frame_after.pc(), pc_before, "pc must be unchanged");
    assert_eq!(
        frame_after.step_state(StepIdx::ZERO),
        step_state_before,
        "step state must be unchanged"
    );
    assert_eq!(
        frame_after.executed(),
        executed_before,
        "executed count must be unchanged"
    );
    assert_eq!(frame_after, frame_before, "frame must be byte-equal");
    // action_attempts, counters, journal sequence, and trace MUST remain
    // unchanged. The trace ring and pending_action state are part of the
    // same critical section as the frame mutation.
    let attempts_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected failure")
        .action_attempts
        .clone();
    assert_eq!(attempts_after, attempts_before, "action_attempts unchanged");
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
}

#[test]
fn handle_action_failure_append_failure_keeps_retry_state_and_pending_action() -> Result<(), String>
{
    struct ActionFailedFailsOnce {
        failed: std::sync::atomic::AtomicBool,
    }
    impl crate::journal::RuntimeJournal for ActionFailedFailsOnce {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }

        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            let should_fail = matches!(
                event,
                RuntimeJournalEvent::ActionFailed {
                    run: event_run,
                    step: event_step,
                    ..
                } if event_run == RunId::new(50_050) && event_step == StepIdx::new(1)
            );
            if should_fail && !self.failed.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            let should_fail = events.iter().any(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::ActionFailed {
                        run: event_run,
                        step: event_step,
                        ..
                    } if *event_run == RunId::new(50_050) && *event_step == StepIdx::new(1)
                )
            });
            if should_fail && !self.failed.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(ActionFailedFailsOnce {
        failed: std::sync::atomic::AtomicBool::new(false),
    });
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_050);
    submit_run(
        &mut shard,
        run,
        retry_workflow()?,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    let frame_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry workflow suspension")?
        .frame
        .clone();
    let attempts_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry workflow suspension")?
        .action_attempts
        .clone();
    let pending_actions_before = shard.pending_action_clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );
    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected action failure")?;
    assert_eq!(state_after.frame, frame_before, "frame must be unchanged");
    assert_eq!(
        state_after.action_attempts, attempts_before,
        "retry attempts must be unchanged"
    );
    assert_eq!(shard.pending_action_clone(), pending_actions_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let retried_state = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry is scheduled")?;
    assert_eq!(retried_state.action_attempts.get(1).copied(), Some(2));
    assert_eq!(
        shard.pending_action_get(run).map(|pending| pending.attempt),
        Some(2)
    );
    Ok(())
}

#[test]
fn fail_run_action_failure_queue_full_keeps_state_and_allows_retry() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 8;
    const QUEUE_BATCH_SIZE: usize = 8;
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix("vb-runtime-fail-run-batch-")
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let journal = std::sync::Arc::new(
        vb_storage::FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?,
    );
    let queue = std::sync::Arc::new(
        vb_storage::JournalWriterQueue::new(
            QUEUE_CAPACITY,
            QUEUE_BATCH_SIZE,
            vb_storage::StorageLimits::DEFAULT,
        )
        .map_err(|error| error.to_string())?,
    );
    let adapter = std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let shared: SharedRuntimeJournal = adapter.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_060);
    submit_run(
        &mut shard,
        run,
        require_workflow("suspended", suspended_workflow())?,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );

    let target_pending = QUEUE_CAPACITY
        .checked_sub(1)
        .ok_or("queue capacity must leave one slot")?;
    let counts_before_fill = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    if counts_before_fill.journaled > target_pending {
        return Err(format!(
            "submit queued {} journaled rows, exceeding target {target_pending}",
            counts_before_fill.journaled
        ));
    }
    let filler_needed = target_pending.saturating_sub(counts_before_fill.journaled);
    let filler_runs = [
        RunId::new(50_061),
        RunId::new(50_062),
        RunId::new(50_063),
        RunId::new(50_064),
        RunId::new(50_065),
        RunId::new(50_066),
        RunId::new(50_067),
        RunId::new(50_068),
    ];
    for filler_run in filler_runs.iter().copied().take(filler_needed) {
        queue
            .enqueue_journaled(vb_storage::JournalEvent::RunCancelled {
                run: filler_run,
                seq: vb_storage::EventSeq::ZERO,
                attempt: 1,
                reason: None,
            })
            .map_err(|error| error.to_string())?;
    }
    let filled_counts = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    assert_eq!(filled_counts.journaled, target_pending);
    assert_eq!(filled_counts.strict, 0);

    let ticket = make_ticket(run, StepIdx::ZERO, 1);
    let frame_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after action suspension")?
        .frame
        .clone();
    let attempts_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after action suspension")?
        .action_attempts
        .clone();
    let pending_actions_before = shard.pending_action_clone();
    let counters_before = shard.counters().snapshot();
    let runtime_state_before = shard.runtime_state_get(run);
    let terminal_before = shard.terminal_runs_contains(run);
    let active_before = shard.active_run_count();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected typed StorageJournalAppend(QueueFull), got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected fail-run batch")?;
    assert_eq!(state_after.frame, frame_before, "frame must be unchanged");
    assert_eq!(state_after.action_attempts, attempts_before);
    assert_eq!(shard.pending_action_clone(), pending_actions_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.runtime_state_get(run), runtime_state_before);
    assert_eq!(shard.terminal_runs_contains(run), terminal_before);
    assert_eq!(shard.active_run_count(), active_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?,
        filled_counts,
        "failed atomic batch must not consume the one remaining queue slot"
    );
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let flushed = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(flushed.drained, target_pending);
    assert_eq!(flushed.written, target_pending);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| matches!(
            event,
            vb_storage::JournalEvent::ActionFailedEvent { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
        )),
        false,
        "rejected batch must leave no ActionFailed or RunFailed durable rows"
    );

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.terminal_runs_contains(run), true);
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    let retry_counts = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    assert_eq!(retry_counts.journaled, 3);
    assert_eq!(retry_counts.strict, 0);
    let retry_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.drained, 3);
    assert_eq!(retry_flush.written, 3);
    let durable_after_retry = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_retry
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_retry
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_retry
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RetryCheckpoint { .. }))
            .count(),
        1
    );
    Ok(())
}


// Direct in-memory terminal failure coverage. The finite budget replacement
// below is synthetic; this does not simulate OOM or establish recovery safety.
#[cfg(not(kani))]
#[test]
fn terminal_action_failure_refusal_preserves_ticket_and_retained_identity() -> Result<(), String> {
    struct RejectTerminalFailure {
        accepted: crate::journal::VolatileRuntimeJournal,
        attempted: std::sync::Mutex<Option<(Vec<RuntimeJournalEvent>, vb_storage::EventSeq)>>,
    }
    impl crate::journal::RuntimeJournal for RejectTerminalFailure {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced_batch(&[event], vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.append_sequenced_batch(&[event], seq)
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events.iter().any(|event| matches!(event, RuntimeJournalEvent::RunFailed { .. })) {
                let mut attempted = self.attempted.lock()
                    .map_err(|_| RuntimeError::JournalPoisoned)?;
                *attempted = Some((events.to_vec(), seq));
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "test_terminal_action_failure_refusal",
                });
            }
            crate::journal::RuntimeJournal::append_sequenced_batch(&self.accepted, events, seq)
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    for fallback in [false, true] {
        let journal = std::sync::Arc::new(RejectTerminalFailure {
            accepted: crate::journal::VolatileRuntimeJournal::new(),
            attempted: std::sync::Mutex::new(None),
        });
        let shared: SharedRuntimeJournal = journal.clone();
        let mut config = small_config();
        config.max_active_runs = 1;
        let mut shard = Shard::new_with_journal(config, shared)
            .map_err(|error| error.to_string())?;
        let retained = RunId::new(50_131);
        let run = RunId::new(50_132);
        shard.checked_out_run_insert(retained).map_err(|error| error.to_string())?;
        shard.terminal_runs_insert(retained).map_err(|error| error.to_string())?;
        let workflow = require_workflow("terminal-failure-refusal", suspended_workflow())?;
        submit_run(&mut shard, run, workflow, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
        let ticket = shard.pending_action_get(run).ok_or("missing issued pending ticket")?;
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        if fallback {
            let state = shard.run_state_get_mut(run).ok_or("missing active run")?;
            state.admission = Some(crate::admission::RunAdmission::with_budget(
                state.workflow.digest(),
                run,
                CapabilitySet::empty(),
                vb_core::RuntimePolicy::Relaxed,
                journal_bytes_budget(1),
            ));
            state.journal_bytes_written = 0;
        }
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retry_policy: VbRetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let state = shard.run_state_get(run).ok_or("missing before state")?;
        let preflight = super::preflight_action_failure(state, ticket, &failure)
            .map_err(|error| error.to_string())?;
        assert!(matches!(preflight.outcome, super::ActionFailureOutcome::FailRun));
        let normal_events = vec![
            RuntimeJournalEvent::ActionFailed {
                run,
                step: ticket.step,
                action: ticket.action,
                attempt: ticket.attempt,
            },
            RuntimeJournalEvent::RunFailed { run, attempt: state.run_attempt.get() },
        ];
        assert_eq!(
            shard.command_journal_bytes_exceeds(state, &normal_events)
                .map_err(|error| error.to_string())?,
            fallback
        );
        let expected_events = if fallback {
            vec![RuntimeJournalEvent::RunFailed { run, attempt: state.run_attempt.get() }]
        } else {
            normal_events
        };
        let before = state.clone();
        let pending = shard.pending_action_clone();
        let seq = shard.journal_sequence_for(run);
        let quota = shard.charged_quotas.clone();
        let usage = shard.usage_snapshot();
        let global = shard.global_cost_usage();
        let counters = shard.counters().snapshot();
        let trace = shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity());
        let runtime = shard.runtime_state_get(run);
        let accepted = journal.accepted.snapshot().map_err(|error| error.to_string())?;
        assert!(journal.attempted.lock().map_err(|_| "attempt lock poisoned")?.is_none());

        assert_eq!(
            shard.handle_action_failure(ticket, failure),
            Err(RuntimeError::UnsupportedOperation {
                operation: "test_terminal_action_failure_refusal",
            })
        );
        assert_eq!(
            journal.attempted.lock().map_err(|_| "attempt lock poisoned")?.as_ref(),
            Some(&(expected_events, seq))
        );
        assert_eq!(journal.accepted.snapshot().map_err(|error| error.to_string())?, accepted);
        assert_eq!(shard.run_state_get(run), Some(&before));
        assert_eq!(shard.pending_action_clone(), pending);
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        assert_eq!(shard.journal_seq_get(run), Some(seq));
        assert_eq!(shard.charged_quotas, quota);
        assert_eq!(shard.usage_snapshot(), usage);
        assert_eq!(shard.global_cost_usage(), global);
        assert_eq!(shard.counters().snapshot(), counters);
        assert_eq!(
            shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity()),
            trace
        );
        assert_eq!(shard.runtime_state_get(run), runtime);
        assert!(!shard.checked_out_run_contains(run));
        assert_eq!(shard.active_run_count(), 1);
        assert!(shard.terminal_runs_contains(retained));
        assert!(!shard.terminal_runs_contains(run));
    }
    Ok(())
}
