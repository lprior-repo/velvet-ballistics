
/// Workflow: SetConst(slot0=true) -> Ask(prompt=slot0, timeout=Some(slot1)) -> AskResume(answer=slot2) -> Finish(result=slot2)
fn ask_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let set_timeout = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
    };
    let resume = CompiledNode {
        id: vb_core::ids::StepIdx::new(3),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(4)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(4),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_then_finish"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            vb_core::value::ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn retryable_failure() -> vb_core::action::ActionFailure {
    vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: vb_core::action::RetryPolicy::Retryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// ---------------------------------------------------------------------------
// handle_submit: valid workflow with inputs via SubmitWithInputs
// ---------------------------------------------------------------------------

#[test]
fn shard_submit_with_inputs_seeds_slots_and_drives() -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    let run = super::RunId::new(700);
    let inputs = Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(true))]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_submit_with_inputs_rejects_duplicate_run() -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let run = super::RunId::new(701);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let inputs = Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(false))]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    Ok(())
}

#[test]
fn shard_submit_with_inputs_rejects_capacity_exceeded() -> Result<(), &'static str> {
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
        max_terminal_outcomes: 100_000,
};
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let wf1 = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let wf2 = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let inputs = Box::from([]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run: super::RunId::new(2),
            workflow: wf2,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    Ok(())
}

struct RejectAdmissionHeaderJournal {
    reject_submitted: bool,
    reject_admission: bool,
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectAdmissionHeaderJournal {
    fn shared(reject_submitted: bool, reject_admission: bool) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            reject_submitted,
            reject_admission,
            events: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> crate::RuntimeResult<Vec<RuntimeJournalEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RejectAdmissionHeaderJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        let reject_submitted =
            matches!(event, RuntimeJournalEvent::RunSubmitted { .. }) && self.reject_submitted;
        let reject_admission =
            matches!(event, RuntimeJournalEvent::RunAdmission { .. }) && self.reject_admission;
        if reject_submitted || reject_admission {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

fn assert_admission_header_persistence_failed(result: crate::RuntimeResult<bool>) {
    let is_expected = matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    );
    assert!(is_expected, "expected AdmissionHeaderPersistenceFailed");
}

#[test]
fn shard_submit_run_submitted_append_failure_maps_to_admission_header_persistence_failed(
) -> Result<(), &'static str> {
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let journal = RejectAdmissionHeaderJournal::shared(true, false);
    let mut shard = match Shard::new_with_journal_and_artifact_store(        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let run = super::RunId::new(712);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    assert_admission_header_persistence_failed(shard.tick());

    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    assert_eq!(journal.snapshot(), Ok(Vec::new()));
    Ok(())
}

#[test]
fn shard_submit_run_admission_append_failure_maps_to_admission_header_persistence_failed(
) -> Result<(), &'static str> {
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let journal = RejectAdmissionHeaderJournal::shared(false, true);
    let mut shard = match Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    ) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let run = super::RunId::new(713);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    assert_admission_header_persistence_failed(shard.tick());

    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    match journal.snapshot() {
        Ok(events) => {
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events.first(),
                Some(RuntimeJournalEvent::RunSubmitted { .. })
            ));
        }
        Err(error) => panic!("journal snapshot failed: {error:?}"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_resume: resume a waiting run after timer was already removed
// ---------------------------------------------------------------------------

#[test]
fn shard_resume_on_waiting_run_after_timer_removed_still_suspends() -> Result<(), &'static str> {
    // Submit a timed wait workflow, which enters a wait-suspended state with a pending timer.
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow =
        timed_wait_then_finish_workflow().ok_or("timed wait workflow fixture construction failed")?;
    let run = super::RunId::new(710);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);

    // When resuming while the run is waiting, drive_run re-drives and re-suspends
    // because the WaitUntil deadline hasn't been met (no timer fire).
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Run is still active (re-suspended)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
        }
        other => assert_eq!(other, None),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_cancel: cancel a finished run (no-op, already removed)
// ---------------------------------------------------------------------------

#[test]
fn shard_cancel_on_finished_run_succeeds_silently_without_counter_increment(
) -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    let run = super::RunId::new(720);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // When cancelling the already-finished run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then no additional counter increment
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// RS-011: drive_run failure during submit must drain coalesce buffer before
// discarding the journal sequence
// ---------------------------------------------------------------------------
//
// Bug summary (chunk_001_submit.rs:231-247):
//   The pre-fix code dropped the per-run journal sequence on drive_run
//   failure but did NOT drain coalesce_buffer. Any events the failed
//   drive left in the coalesce window were then persisted on the next
//   flush as "ghost" events for a run that had already been recorded
//   as failed (or, in the rare buffered-window case, was about to be).
//
// The fix calls `discard_buffered_events_for_run(run)` BEFORE
// `discard_journal_sequence(run)`, but only when the run state was
// actually lost during the failed drive (RS-005 re-inserts state on
// intermediate failures, so the drain is skipped when state is
// recoverable).
//
// Tests below cover both layers:
//   1. Helper-level: discard_buffered_events_for_run isolates the target
//      run, leaving other runs' buffered events untouched.
//   2. Integration: drive_run failure during apply_terminal_finished
//      (journal rejects RunFinished) triggers the fix path, which
//      discards the per-run journal sequence and is a no-op against
//      an empty coalesce buffer (window=1, synchronous writes).

/// RS-011 helper-level test: `discard_buffered_events_for_run` must
/// remove only the target run's buffered events and leave other runs'
/// events untouched. `discard_journal_sequence` must symmetrically
/// remove only the target run's per-run sequence counter.
#[test]
fn rs011_discard_buffered_events_for_run_isolates_target_run() -> Result<(), &'static str> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        // > 1 keeps the buffered path active so manual injection of
        // coalesce_buffer entries is not at risk of being flushed by
        // a stray tick between setup and assertion.
        coalesce_window_ticks: 4,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
    };
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };

    let target_run = super::RunId::new(810);
    let other_run = super::RunId::new(811);

    // Pre-seed the buffer with events for two runs. We push directly into
    // the pub(crate) coalesce_buffer because the fix path is the only
    // place this scenario naturally arises (drive_run failure with state
    // not present), and that combination cannot be triggered by the
    // existing test fixtures alone.
    let step_zero = vb_core::ids::StepIdx::ZERO;
    let step_one = vb_core::ids::StepIdx::new(1);
    shard
        .coalesce_buffer
        .push((RuntimeJournalEvent::StepStarted { run: target_run, step: step_zero }, vb_storage::EventSeq::ZERO));
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepSucceeded {
            run: target_run,
            step: step_zero,
            output: SlotIdx::ZERO,
            attempt: 1,
        },
        vb_storage::EventSeq::new(1),
    ));
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepStarted { run: target_run, step: step_one },
        vb_storage::EventSeq::new(2),
    ));
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepStarted { run: other_run, step: step_zero },
        vb_storage::EventSeq::ZERO,
    ));

    // Pre-seed the journal sequence map so we can verify the symmetric
    // discard.
    shard
        .journal_sequences
        .insert(target_run, vb_storage::EventSeq::new(3));
    shard
        .journal_sequences
        .insert(other_run, vb_storage::EventSeq::new(1));

    // Sanity: buffer holds 4 events (3 for target, 1 for other) and
    // both runs have a sequence counter.
    assert_eq!(shard.coalesce_buffer.len(), 4);
    assert!(shard.journal_sequences.get(&target_run).is_some());
    assert!(shard.journal_sequences.get(&other_run).is_some());

    // Exercise the helper the fix relies on.
    shard.discard_buffered_events_for_run(target_run);

    // RS-011 invariant 1: only target_run's buffered events are gone.
    assert_eq!(
        shard.coalesce_buffer.len(),
        1,
        "discard must remove only target_run's events"
    );
    assert_eq!(
        shard.coalesce_buffer[0].0.run_id(),
        other_run,
        "remaining buffered event must belong to other_run"
    );

    // Exercise the symmetric sequence discard.
    shard.discard_journal_sequence(target_run);

    // RS-011 invariant 2: only target_run's sequence is gone; other_run's
    // sequence counter must be untouched.
    assert!(
        shard.journal_sequences.get(&target_run).is_none(),
        "target_run sequence must be discarded"
    );
    assert_eq!(
        shard.journal_sequences.get(&other_run).copied(),
        Some(vb_storage::EventSeq::new(1)),
        "other_run sequence must be preserved"
    );
    Ok(())
}

/// RS-011 integration test: when `drive_run` fails inside
/// `apply_terminal_finished` (the journal rejects the terminal
/// `RunFinished` event), the run state is NOT re-inserted and the fix
/// path in `handle_submit_with_inputs_contracts_and_header_mode` must
/// discard the per-run journal sequence. Pre-fix, the sequence counter
/// would survive the failed submit and a subsequent flush would have
/// nothing to drain — but a future buffered drive for the same run id
/// would start at the wrong sequence, producing replay divergence.
///
/// This test exercises the fix path via a journal stub that accepts
/// every event except `RunFinished`, which is rejected with
/// `JournalError::QueueFull`. Because `coalesce_window_ticks = 1` in
/// `small_config()`, evidence events flush synchronously and the
/// coalesce buffer is empty when the fix path runs — the discard is
/// a no-op against the buffer but the sequence discard is the
/// observable invariant.
struct RejectRunFinishedJournal {
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectRunFinishedJournal {
    fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            events: std::sync::Mutex::new(Vec::new()),
        })
    }
}

impl crate::journal::RuntimeJournal for RejectRunFinishedJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        if matches!(event, RuntimeJournalEvent::RunFinished { .. }) {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }
    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn rs011_submit_failure_with_run_state_lost_discards_journal_sequence() -> Result<(), &'static str>
{
    let config = small_config();
    let journal = RejectRunFinishedJournal::shared();
    let mut shard = match Shard::new_with_journal_and_artifact_store(
        config,
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    ) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    let run = super::RunId::new(820);

    // ── RS-011 pre-seed: exercise the buffered-events discard path ───────
    //
    // The black-hat reviewer caught that `small_config()` sets
    // `coalesce_window_ticks = 1`, so every evidence event during drive_run
    // is written synchronously via `append_sequenced` — the coalesce buffer
    // is empty when drive_run fails. Under that regime
    // `discard_buffered_events_for_run(run)` is a no-op and the original
    // assertion `coalesce_buffer has no events for run` passes trivially,
    // regardless of whether the fix is present. The pre-fix code is then
    // indistinguishable from the post-fix code under this test, defeating
    // its purpose as a regression gate.
    //
    // To make the test actually exercise the fix path we pre-seed
    // `coalesce_buffer` with events that represent "events buffered during
    // earlier ticks that are now orphaned by drive_run failure". This is
    // the exact scenario the fix targets: the run state was lost during the
    // drive, the per-run sequence is about to be discarded, and the buffer
    // holds ghost events that would otherwise persist on the next coalesce
    // flush. With the fix in place `discard_buffered_events_for_run(run)`
    // runs before the error propagates and drains these events; with the
    // fix reverted the events survive and the strengthened assertion
    // below catches the regression.
    //
    // The pre-seed runs BEFORE the Submit enqueue so the events are
    // present at the start of the failed `tick()`. They are written
    // directly into the `pub(crate) coalesce_buffer` rather than via
    // `append_journal_event` because the natural "buffered events for an
    // orphaned run" state is internal to drive_run's failure path and
    // cannot be reproduced through the public API alone.
    let step_zero = vb_core::ids::StepIdx::ZERO;
    let step_one = vb_core::ids::StepIdx::new(1);
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepStarted {
            run,
            step: step_zero,
        },
        vb_storage::EventSeq::ZERO,
    ));
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepSucceeded {
            run,
            step: step_zero,
            output: SlotIdx::ZERO,
            attempt: 1,
        },
        vb_storage::EventSeq::new(1),
    ));
    shard.coalesce_buffer.push((
        RuntimeJournalEvent::StepStarted {
            run,
            step: step_one,
        },
        vb_storage::EventSeq::new(2),
    ));

    // Sanity: the pre-seeded events are in place before tick() runs.
    assert_eq!(
        shard.coalesce_buffer.len(),
        3,
        "RS-011 pre-seed must inject exactly 3 buffered events for run {run:?}"
    );
    assert!(
        shard
            .coalesce_buffer
            .iter()
            .all(|(event, _)| event.run_id() == run),
        "RS-011 pre-seed must inject events that all belong to the target run"
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // drive_run drives the workflow (SetConst + Finish), emits 5 evidence
    // events written synchronously (window=1), then attempts the
    // terminal RunFinished event which the journal rejects. The
    // pre-seeded buffered events above must be drained by the fix path
    // before the error propagates.
    let tick_result = shard.tick();
    let error = match tick_result {
        Err(error) => error,
        Ok(value) => return Err("drive_run must fail when journal rejects RunFinished"),
    };

    // The failure must surface as a typed storage journal error, not a
    // panic or admission-header error.
    match &error {
        RuntimeError::StorageJournalAppend { source } => {
            assert!(
                matches!(source.as_ref(), vb_storage::JournalError::QueueFull),
                "expected QueueFull source, got {source:?}"
            );
        }
        other => return Err("expected StorageJournalAppend error from drive_run failure"),
    }

    // RS-011 invariant 3: run_state must NOT contain the run after
    // drive_run failure on the terminal path. Pre-fix, `take_run_state`
    // removed it but `finish_run`'s failed journal write never
    // re-inserted it — so this condition holds regardless of the fix.
    // The fix code branches on this exact condition to decide whether
    // to drain.
    assert!(
        !shard.run_state_contains(run),
        "RS-011: run must NOT be in run_state after drive_run failure"
    );

    // RS-011 invariant 4: the per-run journal sequence counter must be
    // discarded. Evidence events wrote 5 synchronous entries
    // (StepStarted, SlotWritten, StepSucceeded for SetConst, then
    // StepStarted, StepSucceeded for Finish), advancing
    // journal_sequences[run] to 5 before the RunFinished write failed.
    // Pre-fix the counter would survive at 5; post-fix it must be gone.
    assert!(
        shard.journal_sequences.get(&run).is_none(),
        "RS-011: journal_sequences[run] must be discarded by the fix path, \
         got {:?}",
        shard.journal_sequences.get(&run).copied()
    );

    // RS-011 invariant 5: coalesce_buffer must contain no events for
    // the failed run. We PRE-SEEDED three events for `run` above so this
    // assertion is only satisfied when `discard_buffered_events_for_run`
    // actually drains them from the fix path. With `small_config()`'s
    // `coalesce_window_ticks = 1`, drive_run's own evidence events are
    // written synchronously, so the only way for ghost events to land in
    // the buffer is via the pre-seed — which simulates the buffered-mode
    // failure the fix targets. If `self.discard_buffered_events_for_run(run)`
    // were removed from `chunk_001_submit.rs`, the pre-seeded events would
    // survive the failed tick and persist as ghost events on the next
    // coalesce flush.
    let buffered_for_failed_run = shard
        .coalesce_buffer
        .iter()
        .filter(|(event, _)| event.run_id() == run)
        .count();
    assert_eq!(
        buffered_for_failed_run, 0,
        "RS-011: coalesce_buffer must hold no events for the failed run after \
         drive_run failure; pre-seeded events would be drained by the fix path. \
         Found {buffered_for_failed_run} entries still buffered for run {run:?} \
         (would persist as ghost events on the next coalesce flush)"
    );

    // Pre-fix the journal would have recorded RunSubmitted, RunAdmission,
    // plus the 5 evidence events; RunFinished was rejected. The fix
    // path does not touch the durable journal contents — it only
    // cleans up in-memory bookkeeping. Verify the durable record
    // reflects the partial drive.
    let recorded = journal
        .events
        .lock()
        .map(|events| events.clone())
        .map_err(|_| "journal poisoned")?;
    assert!(
        recorded
            .iter()
            .any(|e| matches!(e, RuntimeJournalEvent::RunSubmitted { .. })),
        "RunSubmitted must have been durably written before drive_run failure"
    );
    assert!(
        recorded
            .iter()
            .all(|e| !matches!(e, RuntimeJournalEvent::RunFinished { .. })),
        "RunFinished must NOT be in the durable record (rejected by journal)"
    );
    Ok(())
}
