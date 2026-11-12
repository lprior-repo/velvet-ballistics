fn queued_journal_fixture(
    prefix: &str,
    capacity: usize,
    batch_size: usize,
) -> Result<
    (
        std::sync::Arc<vb_storage::FjallJournal>,
        std::sync::Arc<vb_storage::JournalWriterQueue>,
        std::sync::Arc<crate::journal::QueuedStorageRuntimeJournal>,
        SharedRuntimeJournal,
    ),
    String,
> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let journal = std::sync::Arc::new(
        vb_storage::FjallJournal::open(dir.keep(), None).map_err(|error| error.to_string())?,
    );
    let queue = std::sync::Arc::new(
        vb_storage::JournalWriterQueue::new(
            capacity,
            batch_size,
            vb_storage::StorageLimits::DEFAULT,
        )
        .map_err(|error| error.to_string())?,
    );
    let adapter = std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let shared: SharedRuntimeJournal = adapter.clone();
    Ok((journal, queue, adapter, shared))
}

fn fill_queue_to_one_free_slot(
    queue: &vb_storage::JournalWriterQueue,
    capacity: usize,
    first_filler_run: u64,
) -> Result<usize, String> {
    let target_pending = capacity
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
    for offset in 0..filler_needed {
        let offset = u64::try_from(offset).map_err(|error| error.to_string())?;
        let run_number = first_filler_run
            .checked_add(offset)
            .ok_or("filler run id overflow")?;
        queue
            .enqueue_journaled(vb_storage::JournalEvent::RunCancelled {
                run: RunId::new(run_number),
                seq: vb_storage::EventSeq::ZERO,
                attempt: 1,
                reason: None,
            })
            .map_err(|error| error.to_string())?;
    }
    queue
        .pending_profile_counts()
        .map(|counts| counts.journaled)
        .map_err(|error| error.to_string())
}

fn next_journal_seq(
    seq: Option<vb_storage::EventSeq>,
) -> Result<Option<vb_storage::EventSeq>, String> {
    match seq {
        Some(value) => value
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
            .map(Some)
            .ok_or_else(|| "journal sequence overflow".to_owned()),
        None => Ok(Some(vb_storage::EventSeq::new(1))),
    }
}

#[test]
fn submit_drive_queue_full_after_admission_leaves_resumable_retry() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 4;
    const QUEUE_BATCH_SIZE: usize = 4;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-submit-drive-rollback-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_100);
    let workflow = require_workflow("suspended", suspended_workflow())?;

    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: workflow.clone(),
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
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
        "expected first drive batch to fail closed on QueueFull, got {result:?}"
    );

    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.terminal_runs_contains(run), false);
    assert_eq!(
        shard.journal_sequences.get(&run).copied(),
        Some(vb_storage::EventSeq::new(2))
    );
    assert_eq!(
        shard.counters().snapshot(),
        crate::counters::CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        }
    );
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        vec![TraceEvent::RunSubmitted { run }]
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        2
    );
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));

    let header_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(header_flush.drained, 2);
    assert_eq!(header_flush.written, 2);
    let durable_header = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        durable_header.as_slice(),
        [
            vb_storage::JournalEvent::RunAccepted { seq, .. },
            vb_storage::JournalEvent::RunAdmissionBudget { seq: admission_seq, .. },
        ] if *seq == vb_storage::EventSeq::new(0)
            && *admission_seq == vb_storage::EventSeq::new(1)
    ));
    assert_eq!(
        durable_header.iter().any(|event| matches!(
            event,
            vb_storage::JournalEvent::StepStarted { .. }
                | vb_storage::JournalEvent::ActionScheduledTicket { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
        )),
        false
    );

    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    assert_eq!(
        shard.pending_action_get(run).map(|ticket| ticket.attempt),
        Some(1)
    );
    assert_eq!(
        shard.journal_sequences.get(&run).copied(),
        Some(vb_storage::EventSeq::new(6))
    );

    let resume_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(resume_flush.drained, 4);
    assert_eq!(resume_flush.written, 4);
    let durable_after_resume = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunResumed { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(
                event,
                vb_storage::JournalEvent::ActionScheduledTicket { .. }
            ))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn retry_now_action_failure_queue_full_preserves_post_failure_state() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 16;
    const QUEUE_BATCH_SIZE: usize = 16;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-retry-post-failure-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_070);
    submit_run(&mut shard, run, retry_workflow()?);
    // Watermark contract: the retry gate flushes an undrained own-prefix, so
    // drain the run's submit prefix first. The queue is then refilled with
    // other runs' events and the retry drive still meets a full journal,
    // preserving this test's QueueFull scenario under the new invariant.
    adapter.drain_all().map_err(|error| error.to_string())?;
    let filled_counts = fill_queue_to_one_free_slot(&queue, QUEUE_CAPACITY, 50_071)?;
    assert_eq!(filled_counts, QUEUE_CAPACITY - 1);

    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let counters_before = shard.counters().snapshot();
    let active_before = shard.active_run_count();
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);

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
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected QueueFull after accepted ActionFailed, got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected retry drive")?;
    assert_eq!(state_after.action_attempts.get(1).copied(), Some(2));
    assert_eq!(state_after.frame.pc(), StepIdx::new(1));
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.active_run_count(), active_before);
    assert_eq!(
        shard.journal_sequences.get(&run).copied(),
        next_journal_seq(seq_before)?
    );
    let trace_after = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    assert_eq!(trace_after.len(), trace_before.len() + 1);
    assert!(trace_after.iter().any(|event| {
        matches!(event, TraceEvent::ActionFailed { run: event_run, step, .. }
            if *event_run == run && *step == StepIdx::new(1))
    }));
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        QUEUE_CAPACITY
    );
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let rejected_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(rejected_flush.drained, QUEUE_CAPACITY);
    assert_eq!(rejected_flush.written, QUEUE_CAPACITY);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| {
                matches!(event, vb_storage::JournalEvent::StepStarted { run: event_run, step, .. }
                    if *event_run == run && *step == StepIdx::new(1))
            })
            .count(),
        1,
        "failed retry drive must not append a second StepStarted for the retried action"
    );
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| {
                matches!(event, vb_storage::JournalEvent::ActionScheduledTicket { run: event_run, .. }
                    if *event_run == run)
            })
            .count(),
        1,
        "failed retry drive must not append the attempt-2 ActionScheduledTicket"
    );

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.pending_action_get(run).map(|pending| pending.attempt),
        Some(2)
    );
    assert_eq!(
        shard
            .run_state_get(run)
            .and_then(|state| state.action_attempts.get(1).copied()),
        Some(2)
    );
    let retry_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.written, retry_flush.drained);
    Ok(())
}

#[test]
fn drive_handler_action_failure_queue_full_preserves_post_failure_state() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 16;
    const QUEUE_BATCH_SIZE: usize = 16;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-handler-post-failure-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_080);
    submit_run(
        &mut shard,
        run,
        require_workflow("error_handler", error_handler_workflow())?,
    );
    let filled_counts = fill_queue_to_one_free_slot(&queue, QUEUE_CAPACITY, 50_081)?;
    assert_eq!(filled_counts, QUEUE_CAPACITY - 1);
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);

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
        "expected QueueFull after accepted handler ActionFailed, got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected handler drive")?;
    assert_eq!(state_after.frame.pc(), StepIdx::new(2));
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.terminal_runs_contains(run), false);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(
        shard.journal_sequences.get(&run).copied(),
        next_journal_seq(seq_before)?
    );
    let trace_after = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    assert_eq!(trace_after.len(), trace_before.len() + 1);
    assert!(trace_after.iter().any(|event| {
        matches!(event, TraceEvent::ActionFailed { run: event_run, step, .. }
            if *event_run == run && *step == StepIdx::new(1))
    }));
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let rejected_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(rejected_flush.drained, QUEUE_CAPACITY);
    assert_eq!(rejected_flush.written, QUEUE_CAPACITY);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| {
            matches!(event, vb_storage::JournalEvent::StepStarted { run: event_run, step, .. }
                if *event_run == run && *step == StepIdx::new(2))
        }),
        false,
        "failed handler drive must not append handler StepStarted"
    );
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| {
            matches!(event, vb_storage::JournalEvent::RunFinished { run: event_run, .. }
                if *event_run == run)
        }),
        false,
        "failed handler drive must not append RunFinished"
    );

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.terminal_runs_contains(run), true);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    let retry_flush = adapter
        .flush_batch()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.written, retry_flush.drained);
    Ok(())
}

#[test]
fn finish_run_append_failure_reports_rollback_failure() -> Result<(), String> {
    struct RejectRunFinishedJournal;
    impl crate::journal::RuntimeJournal for RejectRunFinishedJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if matches!(event, RuntimeJournalEvent::RunFinished { .. }) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::QueueFull),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(RejectRunFinishedJournal);
    let mut config = small_config();
    config.max_active_runs = 1;
    let mut shard = Shard::new_with_journal(config, shared);
    let run = RunId::new(50_090);
    let workflow = require_workflow("finished", finished_workflow())?;
    let state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or("run state fixture must build")?;
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    shard.max_active_runs = 0;
    let result = shard.finish_run(run, state);

    match result {
        Err(RuntimeError::RollbackFailed {
            operation,
            primary,
            rollback,
        }) => {
            assert_eq!(operation, "finish_run");
            assert!(matches!(
                primary.as_ref(),
                RuntimeError::StorageJournalAppend { source }
                    if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
            ));
            assert!(matches!(
                rollback.as_ref(),
                RuntimeError::ActiveRunCapacityExceeded { capacity: 0 }
            ));
            Ok(())
        }
        other => Err(format!("expected rollback-combined error, got {other:?}")),
    }
}

// =======================================================================
// handle_action_completion terminal run fence
// =======================================================================

#[test]
fn handle_action_completion_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(RunId::new(420), StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_cancelled() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(421);
    submit_run(&mut shard, run, wf);
    // Cancel the run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Now attempt action completion on cancelled run
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
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_finished() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(422);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Finished workflow completes immediately — run should no longer be active
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
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// =======================================================================
// handle_action_failure
// =======================================================================

#[test]
fn handle_action_failure_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(RunId::new(430), StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_failure_returns_stale_attempt_when_attempt_mismatch() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(431);
    submit_run(&mut shard, run, wf);
    // Set current attempt to 3
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    // Send attempt=1 (stale) failure
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 3,
        })
    );
}
