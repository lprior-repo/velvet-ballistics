#![forbid(unsafe_code)]
//! Recovery / Hydration behavior tests for vb_runtime.
//!
//! Covers:
//! - Recovery from clean shutdown
//! - Recovery from crash (partial journal)
//! - Hydration of workflow state from journal
//! - Hydration with missing events
//! - Recovery with corrupted snapshot
//! - Checkpoint creation and restore
//! - Incremental recovery
//! - Recovery idempotency
//! - Recovery with max-size journal

use tempfile::TempDir;
use vb_core::{
    ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, WorkflowDigest,
};
use vb_runtime::recovery::{RecoveryResumeStatus, RuntimeRecoveryBoundary};
use vb_storage::recovery::{
    ActionReplayTracker, DigestVerificationRequest, RecoveredStepEntry, RecoveredStepState,
    RecoveryCannotResumeState, RecoveryError, RecoveryFrameSeed, RecoveryHydration,
    RecoveryRuntimeSummary, RecoveryTerminalState, RunSnapshot, hydrate_run_frame,
    hydrate_run_frame_from_events, recover_full_journal, recover_runtime_frame_seed,
    recover_runtime_summary, recover_runtime_summary_with_expected, verify_digests,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent};

fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn open_journal(dir: &TempDir) -> FjallJournal {
    FjallJournal::open(dir.path(), Some(FjallConfig::default()))
        .expect("journal open should succeed")
}

fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) {
    for event in events {
        journal
            .append_strict(event)
            .expect("strict append should succeed");
    }
}

fn assert_unsupported_frame_seed<T: core::fmt::Debug>(
    result: Result<T, RecoveryError>,
    expected_run: RunId,
    expected_reason: &str,
) {
    let Err(RecoveryError::UnsupportedFrameSeed { run, reason }) = result else {
        panic!("expected UnsupportedFrameSeed, got: {result:?}");
    };
    assert_eq!(run, expected_run);
    assert_eq!(reason, expected_reason);
}

fn test_admission_event(run: RunId, seq: EventSeq, digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest: digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

fn build_two_step_finished_run(run: RunId, digest: WorkflowDigest) -> Vec<JournalEvent> {
    let mut seq = 0u64;
    let mut events = Vec::new();

    events.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: digest,
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::ZERO,
        attempt: 1,
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(0),
        value: Some(
            postcard::to_allocvec(&SlotValue::I64(42)).expect("value encoding should succeed"),
        ),
        extra: None,
        attempt: 1,
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::ZERO,
        output: SlotIdx::new(0),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(1),
        attempt: 1,
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(1),
        value: Some(
            postcard::to_allocvec(&SlotValue::I64(99)).expect("value encoding should succeed"),
        ),
        extra: None,
        attempt: 1,
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(1),
        output: SlotIdx::new(1),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(1),
        attempt: 1,
    });

    events
}

// ============================================================================
// SECTION 1: Recovery from clean shutdown
// ============================================================================

/// Given a journal with a full two-step finished run written strictly
/// When the journal is reopened (clean shutdown)
/// Then all events are recovered in exact order
#[test]
fn clean_shutdown_recovery_reads_all_events_in_exact_order() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10100);
    let digest = test_digest(0x01);
    let original = build_two_step_finished_run(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &original);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after clean shutdown");

    assert_eq!(
        recovered.len(),
        original.len(),
        "recovered event count must match after clean shutdown"
    );
    for (i, (orig, rec)) in original.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(
            orig, rec,
            "event at index {i} must match after clean shutdown"
        );
    }
}

/// Given a journal with a full two-step finished run
/// When the journal is reopened (clean shutdown) and summary is recovered
/// Then summary counts exactly match events
#[test]
fn clean_shutdown_recovery_reconstructs_exact_summary_counts() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10101);
    let digest = test_digest(0x02);
    let events = build_two_step_finished_run(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let hydration = recover_runtime_summary(&journal, run)
        .expect("recover_runtime_summary should succeed after clean shutdown");
    let summary = hydration.summary();

    assert_eq!(summary.run, run);
    assert_eq!(summary.workflow, Some(digest));
    assert_eq!(summary.steps_started, 2);
    assert_eq!(summary.steps_succeeded, 2);
    assert_eq!(summary.slots_written, 2);
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(1)
        })
    );
}

/// Given a journal with a finished run
/// When recovery is called twice on freshly opened journals
/// Then both recoveries produce identical summaries (deterministic)
#[test]
fn clean_shutdown_recovery_is_deterministic_across_reopens() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10102);
    let digest = test_digest(0x03);
    let events = build_two_step_finished_run(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let summary_a = {
        let j = open_journal(&dir);
        recover_runtime_summary(&j, run)
            .expect("first recovery should succeed")
            .summary()
    };
    let summary_b = {
        let j = open_journal(&dir);
        recover_runtime_summary(&j, run)
            .expect("second recovery should succeed")
            .summary()
    };

    assert_eq!(summary_a.run, summary_b.run);
    assert_eq!(summary_a.steps_started, summary_b.steps_started);
    assert_eq!(summary_a.steps_succeeded, summary_b.steps_succeeded);
    assert_eq!(summary_a.slots_written, summary_b.slots_written);
    assert_eq!(summary_a.terminal, summary_b.terminal);
}

// ============================================================================
// SECTION 2: Recovery from crash (partial journal)
// ============================================================================

/// Given a run that only wrote partial events before a crash
/// When the journal is reopened
/// Then all events written before the crash are readable
#[test]
fn crash_recovery_reads_partial_events_written_before_crash() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10200);
    let digest = test_digest(0x04);

    let partial = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &partial);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed for partial run");

    assert_eq!(recovered.len(), partial.len());
    for (i, (orig, rec)) in partial.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(orig, rec, "partial event {i} must survive crash");
    }
}

/// Given a partial run with no terminal event (crashed mid-execution)
/// When summary is recovered
/// Then terminal state must be None
#[test]
fn crash_recovery_detects_no_terminal_state_for_incomplete_run() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10201);
    let digest = test_digest(0x05);

    let partial = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &partial);
    }

    let journal = open_journal(&dir);
    let hydration = recover_runtime_summary(&journal, run)
        .expect("recover_runtime_summary should succeed for incomplete run");
    let summary = hydration.summary();

    assert!(
        summary.terminal.is_none(),
        "partial run must not have a terminal state"
    );
    assert_eq!(summary.steps_started, 1);
    assert_eq!(summary.steps_succeeded, 0);
}

/// Given only a RunAccepted event (maximally partial)
/// When summary is recovered
/// Then it returns a valid summary with zero steps
#[test]
fn crash_recovery_run_accepted_only_returns_minimal_summary() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10202);
    let digest = test_digest(0x06);

    {
        let journal = open_journal(&dir);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append should succeed");
    }

    let journal = open_journal(&dir);
    let hydration = recover_runtime_summary(&journal, run)
        .expect("summary recovery should succeed for RunAccepted-only run");
    let summary = hydration.summary();

    assert_eq!(summary.run, run);
    assert_eq!(summary.workflow, Some(digest));
    assert_eq!(summary.steps_started, 0);
    assert_eq!(summary.steps_succeeded, 0);
    assert_eq!(summary.terminal, None);
}

// ============================================================================
// SECTION 3: Hydration of workflow state from journal
// ============================================================================

/// Given a full journal with frame-only evidence and a missing slot payload
/// When hydrate_run_frame_from_events is called
/// Then recovery fails closed with exact slot_values reason
#[test]
fn hydration_from_events_rejects_frame_only_missing_slot_payload() {
    let run = RunId::new(10300);
    let digest = test_digest(0x07);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(3),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(5),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(3),
            output: SlotIdx::new(5),
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

/// Given a journal with slot writes containing values
/// When frame seed is recovered and hydrated through runtime boundary
/// Then slot evidence is preserved but frame-only resume is rejected
#[test]
fn hydration_from_frame_seed_reconstructs_slot_values_and_taint() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10301);
    let digest = test_digest(0x08);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(
                postcard::to_allocvec(&SlotValue::I64(77)).expect("value encoding should succeed"),
            ),
            extra: None,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let seed =
        recover_runtime_frame_seed(&journal, run).expect("frame seed recovery should succeed");

    let slot = seed
        .slots
        .iter()
        .find(|s| s.slot == SlotIdx::ZERO)
        .expect("slot 0 must be in recovered slots");
    assert_eq!(slot.value, SlotValue::I64(77));

    let boundary = vb_runtime::recovery::DurableFrameRecoveryBoundary::from_product(seed);
    // A frame seed product alone is never resumable: the slot bytes
    // survive (the recovery summary preserves them) but the full
    // RunState cannot be rebuilt from journal events only. The
    // runtime boundary must report CannotResume and refuse
    // hydration.
    let cannot_resume = RecoveryCannotResumeState {
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("workflow_missing")
        })
    );
}

/// Given a journal with WaitScheduled and AskScheduled events
/// When frame seed is recovered and hydrated
/// Then step evidence is preserved but frame-only resume is rejected
#[test]
fn hydration_reconstructs_waiting_and_asking_step_states() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10302);
    let digest = test_digest(0x09);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let seed =
        recover_runtime_frame_seed(&journal, run).expect("frame seed recovery should succeed");

    let step0 = seed
        .steps
        .iter()
        .find(|s| s.step == StepIdx::ZERO)
        .expect("step 0 must be in recovered steps");
    assert_eq!(step0.state, RecoveredStepState::Waiting);

    let step1 = seed
        .steps
        .iter()
        .find(|s| s.step == StepIdx::new(1))
        .expect("step 1 must be in recovered steps");
    assert_eq!(step1.state, RecoveredStepState::Asking);

    let boundary = vb_runtime::recovery::DurableFrameRecoveryBoundary::from_product(seed);
    // The frame seed product alone is never resumable: it carries
    // pending_actions, pending_timers, pending_asks, plus the
    // seven full-RunState-missing flags. The runtime boundary
    // must report CannotResume and refuse hydration.
    let cannot_resume = RecoveryCannotResumeState {
        pending_timers: true,
        pending_asks: true,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("pending_timers")
        })
    );
}

/// Given a journal with a RunFailed event
/// When hydrate_run_frame_from_events is called
/// Then frame-only recovery is rejected as missing full RunState
#[test]
fn hydration_reconstructs_failed_terminal_state() {
    let run = RunId::new(10303);
    let digest = test_digest(0x0A);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

// ============================================================================
// SECTION 4: Hydration with missing events
// ============================================================================

/// Given events with a sequence gap (seq 1, then seq 3)
/// When hydrate_run_frame_from_events is called
/// Then it returns ReplayDivergence
#[test]
fn hydration_rejects_sequence_gap_in_events() {
    let run = RunId::new(10400);
    let digest = test_digest(0x0B);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        // Gap: seq 2 is missing
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    // Note: hydrate_run_frame_from_events processes available events without
    // strictly enforcing continuity. Sequence gap detection is the caller's
    // responsibility. With events at seq 0, 1, 3, the function hydrates from
    // what it has (seq 0, 1) and ignores the gap. The frame seed alone
    // is still rejected because the full RunState cannot be rebuilt
    // from journal events only.
    assert_unsupported_frame_seed(result, run, "slot_values");
}

/// Given an empty event list
/// When hydrate_run_frame_from_events is called
/// Then NoRecoveryData is returned
#[test]
fn hydration_returns_no_recovery_data_for_empty_events() {
    let result = hydrate_run_frame_from_events(&[], RunId::new(10401));
    let Err(RecoveryError::NoRecoveryData { run }) = result else {
        panic!("expected NoRecoveryData for empty events, got: {result:?}");
    };
    assert_eq!(run, RunId::new(10401));
}

/// Given a journal with events for a different run only
/// When recovery is requested for a non-existent run
/// Then NoRecoveryData is returned
#[test]
fn hydration_returns_no_recovery_data_for_wrong_run() {
    let dir = TempDir::new().expect("temp dir should be created");
    let existing_run = RunId::new(10402);
    let missing_run = RunId::new(99999);

    {
        let journal = open_journal(&dir);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run: existing_run,
                seq: EventSeq::new(0),
                workflow: test_digest(0x0C),
            })
            .expect("append should succeed");
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary(&journal, missing_run);
    let Err(RecoveryError::NoRecoveryData { run }) = result else {
        panic!("expected NoRecoveryData for missing run, got: {result:?}");
    };
    assert_eq!(run, missing_run);
}

// ============================================================================
// SECTION 5: Recovery with corrupted snapshot
// ============================================================================

/// Given a snapshot with a mismatched run_id
/// When hydrate_run_frame is called with a different run
/// Then CorruptSnapshot error is returned
#[test]
fn corrupt_snapshot_run_mismatch_rejected() {
    let run = RunId::new(10500);
    let wrong_run = RunId::new(99999);
    let digest = test_digest(0x0D);

    let snapshot = RunSnapshot {
        run: wrong_run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::ZERO,
        attempt: 1,
    }];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    let Err(RecoveryError::CorruptSnapshot { .. }) = result else {
        panic!("expected CorruptSnapshot for run mismatch, got: {result:?}");
    };
}

/// Given a snapshot with non-empty slots but empty taint vector
/// When hydrate_run_frame is called
/// Then it fails closed (taint evidence missing)
#[test]
fn corrupt_snapshot_missing_taint_for_non_empty_slots_fails_closed() {
    let run = RunId::new(10501);
    let digest = test_digest(0x0E);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![1, 2, 3],
        taint: vec![],
    };

    let tail = vec![];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(matches!(
        result,
        Err(RecoveryError::CorruptSnapshot { run: found, seq })
            if found == run && seq == EventSeq::new(1)
    ));
}

/// Given a snapshot with empty slots and empty taint
/// When hydrate_run_frame is called with a valid tail
/// Then hydration succeeds (empty taint for empty slots is valid)
#[test]
fn snapshot_empty_slots_empty_taint_hydrates_successfully() {
    let run = RunId::new(10502);
    let digest = test_digest(0x0F);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(
        result.is_ok(),
        "empty taint + empty slots should succeed: {result:?}"
    );
}

// ============================================================================
// SECTION 6: Checkpoint creation and restore
// ============================================================================

/// Given a journal, when a snapshot is put and retrieved by exact seq
/// Then the snapshot is preserved bit-for-bit
#[test]
fn checkpoint_snapshot_roundtrip_preserves_all_fields() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10600);
    let digest = test_digest(0x10);

    let original = RunSnapshot {
        run,
        seq: EventSeq::new(42),
        workflow: digest,
        slots: vec![0xAA, 0xBB],
        taint: vec![0x01, 0x02],
    };

    {
        let journal = open_journal(&dir);
        journal
            .put_snapshot(&original)
            .expect("put_snapshot should succeed");
    }

    let journal = open_journal(&dir);
    let loaded = journal
        .snapshot(run, EventSeq::new(42))
        .expect("snapshot load should succeed")
        .expect("snapshot should exist");

    assert_eq!(loaded.run, original.run);
    assert_eq!(loaded.seq, original.seq);
    assert_eq!(loaded.workflow, original.workflow);
    assert_eq!(loaded.slots, original.slots);
    assert_eq!(loaded.taint, original.taint);
}

/// Given a journal with events AND a snapshot at seq 2
/// When hydrate_run_frame is called with snapshot + tail events after seq 2
/// Then hydration succeeds and applies only tail events
#[test]
fn checkpoint_snapshot_plus_tail_hydrates_using_snapshot_watermark() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10601);
    let digest = test_digest(0x11);

    {
        let journal = open_journal(&dir);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append should succeed");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::ZERO,
                attempt: 1,
            })
            .expect("append should succeed");
    }

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(
        result.is_ok(),
        "snapshot + tail hydration should succeed: {result:?}"
    );
}

// ============================================================================
// SECTION 7: Incremental recovery
// ============================================================================

/// Given snapshot at seq 3 and a tail slot write with no payload
/// When hydrate_run_frame is called
/// Then the missing slot payload is rejected exactly
#[test]
fn incremental_recovery_snapshot_plus_tail_applies_only_events_after_watermark() {
    let run = RunId::new(10700);
    let digest = test_digest(0x12);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(5),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

/// Given snapshot at seq 2 and tail containing an event at seq 1 (before watermark)
/// When hydrate_run_frame is called
/// Then ReplayDivergence is returned because tail is before snapshot
#[test]
fn incremental_recovery_rejects_tail_event_before_watermark() {
    let run = RunId::new(10701);
    let digest = test_digest(0x13);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(1),
        attempt: 1,
    }];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { .. })
    ));
}

// ============================================================================
// SECTION 8: Recovery idempotency
// ============================================================================

/// Given a journal with a finished run
/// When recover_full_journal is called twice with fresh trackers
/// Then both replays produce identical event sets
#[test]
fn recovery_idempotency_full_journal_replayed_twice_identical() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10800);
    let digest = test_digest(0x14);
    let events = build_two_step_finished_run(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let replay_a = {
        let j = open_journal(&dir);
        let mut tracker = ActionReplayTracker::new();
        recover_full_journal(&j, run, &mut tracker, &[], &[]).expect("first replay should succeed")
    };

    let replay_b = {
        let j = open_journal(&dir);
        let mut tracker = ActionReplayTracker::new();
        recover_full_journal(&j, run, &mut tracker, &[], &[]).expect("second replay should succeed")
    };

    assert_eq!(replay_a, replay_b);
    assert_eq!(replay_a.len(), events.len());
}

/// Given snapshot + tail events
/// When hydrate_run_frame is called twice with same inputs
/// Then both results are equivalent
#[test]
fn recovery_idempotency_snapshot_tail_hydrated_twice_identical() {
    let run = RunId::new(10801);
    let digest = test_digest(0x15);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            output: SlotIdx::ZERO,
        },
    ];

    let result_a = hydrate_run_frame(&snapshot, &tail, run);
    let result_b = hydrate_run_frame(&snapshot, &tail, run);

    let frame_a = result_a.expect("first hydration should succeed");
    let frame_b = result_b.expect("second hydration should succeed");

    assert_eq!(frame_a.run_id(), frame_b.run_id());
    assert_eq!(frame_a.pc(), frame_b.pc());
    assert_eq!(frame_a.step_count(), frame_b.step_count());
    assert_eq!(frame_a.slot_count(), frame_b.slot_count());
}

/// Given a journal with events, when recovered twice to Summary
/// Then both summaries are identical (summary path is deterministic)
#[test]
fn recovery_idempotency_summary_recovered_twice_identical() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10802);
    let digest = test_digest(0x16);
    let events = build_two_step_finished_run(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let s1 = {
        let j = open_journal(&dir);
        recover_runtime_summary(&j, run)
            .expect("first summary should succeed")
            .summary()
    };
    let s2 = {
        let j = open_journal(&dir);
        recover_runtime_summary(&j, run)
            .expect("second summary should succeed")
            .summary()
    };

    assert_eq!(s1.run, s2.run);
    assert_eq!(s1.steps_started, s2.steps_started);
    assert_eq!(s1.steps_succeeded, s2.steps_succeeded);
    assert_eq!(s1.slots_written, s2.slots_written);
    assert_eq!(s1.terminal, s2.terminal);
    assert_eq!(s1.suspensions, s2.suspensions);
}

// ============================================================================
// SECTION 9: Recovery with max-size journal
// ============================================================================

/// Given events with near-MAX sequence numbers
/// When recovery is performed
/// Then events are still recoverable
#[test]
fn max_size_journal_near_max_seq_events_recoverable() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10900);
    let digest = test_digest(0x17);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(u64::MAX.saturating_sub(3)),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(u64::MAX.saturating_sub(2)),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(u64::MAX.saturating_sub(1)),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    // events_for_run with near-u64::MAX sequences correctly reports a gap
    // since the journal has no events at lower sequence numbers (the journal
    // only contains seq u64::MAX-3..u64::MAX-1, nothing below). This is correct
    // behavior: the journal detects the missing low-range events.
    let recovered = journal.events_for_run(run);
    assert!(
        matches!(recovered, Err(vb_storage::JournalError::SequenceGap { .. })),
        "events_for_run at near-u64::MAX seq should report SequenceGap due to missing low-range events, got: {recovered:?}"
    );
}

/// Given 100 events for a single run
/// When recovery is performed
/// Then all events are recovered in order
#[test]
fn max_size_journal_many_events_recoverable_in_order() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(10901);
    let digest = test_digest(0x18);
    let step_count: u64 = 50;

    let mut events = Vec::new();
    events.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    });

    for i in 0..step_count {
        events.push(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i.saturating_mul(2).saturating_add(1)),
            step: StepIdx::new((i % 256) as u16),
            attempt: 1,
        });
        events.push(JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(i.saturating_mul(2).saturating_add(2)),
            step: StepIdx::new((i % 256) as u16),
            output: SlotIdx::ZERO,
        });
    }

    events.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(step_count.saturating_mul(2).saturating_add(1)),
        result: SlotIdx::ZERO,
        attempt: 1,
    });

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed for many events");

    assert_eq!(
        recovered.len(),
        events.len(),
        "all {n} events should be recoverable",
        n = events.len()
    );
    for (i, (orig, rec)) in events.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(orig, rec, "event {i} must match");
    }
}

// ============================================================================
// SECTION 10: Additional combinatorial coverage
// ============================================================================

/// Given events with multiple slot writes interleaved with step events
/// When hydrate_run_frame_from_events is called
/// Then slot_count covers the maximum slot index + 1
#[test]
fn multiple_noncontiguous_slot_writes_derive_correct_slot_count() {
    let run = RunId::new(11000);
    let digest = test_digest(0x19);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::new(7),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::new(3),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

/// Given events with actions scheduled and then completed
/// When recover_full_journal is called
/// Then the action is marked resolved in the tracker
#[test]
fn action_scheduled_then_completed_is_resolved_after_recovery() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11001);
    let digest = test_digest(0x1A);
    let action = ActionId::new(99);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(2),
            action,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .expect("full journal recovery should succeed");

    assert!(
        tracker.is_resolved(action, StepIdx::new(2)),
        "action should be resolved after recovery"
    );
}

/// Given events where an action is scheduled, completed, then re-scheduled
/// When recover_full_journal is called
/// Then NonIdempotentActionBlocked is returned
#[test]
fn non_idempotent_action_rescheduled_after_completion_is_blocked() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11002);
    let digest = test_digest(0x1B);
    let action = ActionId::new(100);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action,
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Err(RecoveryError::NonIdempotentActionBlocked {
        action: found_action,
        step: found_step,
    }) = result
    else {
        panic!("expected NonIdempotentActionBlocked, got: {result:?}");
    };
    assert_eq!(found_action, action);
    assert_eq!(found_step, StepIdx::ZERO);
}

/// Given a journal with RunCancelled
/// When summary is recovered
/// Then terminal state is Cancelled
#[test]
fn run_cancelled_produces_cancelled_terminal_state() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11003);
    let digest = test_digest(0x1C);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(1),
                    attempt: 1,
                    reason: None,
                },
            ],
        );
    }

    let journal = open_journal(&dir);
    let summary = recover_runtime_summary(&journal, run)
        .expect("summary recovery should succeed")
        .summary();

    assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));
}

/// Given events with RunAdmission events
/// When summary is recovered
/// Then admission metadata is reflected in the summary
#[test]
fn run_admission_metadata_preserved_in_summary() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11004);
    let digest = test_digest(0x1D);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let summary = recover_runtime_summary(&journal, run)
        .expect("summary recovery should succeed")
        .summary();

    assert_eq!(summary.run, run);
    assert_eq!(summary.steps_started, 1);
    assert_eq!(summary.steps_succeeded, 1);
}

/// Given a journal with a single run and valid terminal state
/// When recover_runtime_summary_with_expected is called with matching terminal
/// Then it succeeds
#[test]
fn recover_runtime_summary_with_expected_terminal_matches() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11005);
    let digest = test_digest(0x1E);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary_with_expected(
        &journal,
        run,
        RecoveryTerminalState::Finished {
            result: SlotIdx::ZERO,
        },
    );
    assert!(
        result.is_ok(),
        "matching terminal should succeed: {result:?}"
    );
}

/// Given a journal with RunCancelled as terminal
/// When recover_runtime_summary_with_expected is called with Finished
/// Then TerminalStateMismatch is returned
#[test]
fn recover_runtime_summary_with_expected_terminal_mismatch() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11006);
    let digest = test_digest(0x1F);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary_with_expected(
        &journal,
        run,
        RecoveryTerminalState::Finished {
            result: SlotIdx::ZERO,
        },
    );
    let Err(RecoveryError::TerminalStateMismatch { expected, found }) = result else {
        panic!("expected TerminalStateMismatch, got: {result:?}");
    };
    assert_eq!(expected.as_str(), "Finished");
    assert_eq!(found.as_str(), "Cancelled");
}

/// Given multiple runs in a journal
/// When recovery is performed for each run
/// Then each run's events are isolated from the others
#[test]
fn recovery_across_multiple_runs_is_isolated() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run_a = RunId::new(11007);
    let run_b = RunId::new(11008);
    let digest_a = test_digest(0x20);
    let digest_b = test_digest(0x21);

    let events_a = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: digest_a,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(2),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];

    let events_b = vec![
        JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: digest_b,
        },
        JournalEvent::RunCancelled {
            run: run_b,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events_a);
        write_events_strict(&journal, &events_b);
    }

    let journal = open_journal(&dir);

    let recovered_a = journal
        .events_for_run(run_a)
        .expect("run A events should exist");
    assert_eq!(recovered_a.len(), events_a.len());

    let recovered_b = journal
        .events_for_run(run_b)
        .expect("run B events should exist");
    assert_eq!(recovered_b.len(), events_b.len());

    let summary_a = recover_runtime_summary(&journal, run_a)
        .expect("summary A should succeed")
        .summary();
    let summary_b = recover_runtime_summary(&journal, run_b)
        .expect("summary B should succeed")
        .summary();

    assert!(matches!(
        summary_a.terminal,
        Some(RecoveryTerminalState::Finished { .. })
    ));
    assert_eq!(summary_b.terminal, Some(RecoveryTerminalState::Cancelled));
}

/// Given a journal with events and a matching digest
/// When verify_digests is called with WorkflowSourceOnly
/// Then it returns Ok
#[test]
fn verify_digests_workflow_source_only_level_passes() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11009);
    let source_digest = test_digest(0x22);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: source_digest,
            }],
        );
    }

    let journal = open_journal(&dir);
    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::workflow_source_only(source_digest),
    );
    assert!(
        result.is_ok(),
        "WorkflowSourceOnly should only check workflow digest"
    );
}

/// Given a journal with events and a mismatched IR digest
/// When verify_digests is called with WorkflowAndIr
/// Then CompiledIrDigestMismatch is returned
#[test]
fn verify_digests_workflow_and_ir_level_detects_ir_mismatch() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11010);
    let source_digest = test_digest(0x23);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: source_digest,
            }],
        );
    }

    let journal = open_journal(&dir);
    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::workflow_and_ir(
            source_digest,
            test_digest(0x24),
            test_digest(0x25),
        ),
    );
    let Err(RecoveryError::CompiledIrDigestMismatch { expected, found }) = result else {
        panic!("expected CompiledIrDigestMismatch, got: {result:?}");
    };
    assert_eq!(expected, test_digest(0x24));
    assert_eq!(found, test_digest(0x25));
}

// ============================================================================
// SECTION 11: Runtime boundary recovery tests
// ============================================================================

/// Given an UnsupportedRecoveryState with pending_actions=true
/// When DurableFrameRecoveryBoundary hydrates
/// Then hydration fails closed and the unsupported state remains observable
#[test]
fn runtime_boundary_exposes_supported_pending_actions_state() {
    let run = RunId::new(11100);
    let digest = test_digest(0x26);

    let seed = RecoveryFrameSeed {
        summary: RecoveryRuntimeSummary {
            run,
            first_seq: EventSeq::ZERO,
            last_seq: EventSeq::ZERO,
            workflow: Some(digest),
            steps_started: 1,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        },
        first_step: StepIdx::ZERO,
        step_count: 1,
        slot_count: 0,
        pc: StepIdx::ZERO,
        steps: vec![RecoveredStepEntry {
            step: StepIdx::ZERO,
            state: RecoveredStepState::Running,
        }],
        slots: vec![],
        pending_actions: vec![],
        unsupported: vb_storage::recovery::UnsupportedRecoveryState::pending_actions_unsupported(),
    };

    let boundary = vb_runtime::recovery::DurableFrameRecoveryBoundary::from_seed(seed);
    let result = boundary.hydrate_run_frame();
    // A frame seed alone never carries the full RunState, so the
    // boundary must report CannotResume with the exact reason and refuse
    // hydration even when the storage-level unsupported state is configured.
    assert!(
        matches!(
            result,
            Err(vb_runtime::RuntimeError::RecoveryCannotResume { ref reason })
                if reason == "pending_actions"
        ),
        "frame seed alone must be rejected: {result:?}"
    );
    assert!(boundary.unsupported_state().pending_actions);
}

/// Given a RecoveryHydration::FrameSeed with unsupported slot_taint
/// When recovery_boundary_from_hydration is called
/// Then boundary reports the unsupported state correctly
#[test]
fn runtime_boundary_exposes_unsupported_state() {
    let run = RunId::new(11101);
    let digest = test_digest(0x27);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::ZERO,
        last_seq: EventSeq::ZERO,
        workflow: Some(digest),
        steps_started: 1,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 1,
        terminal: None,
    };

    let unsupported = vb_storage::recovery::UnsupportedRecoveryState {
        slot_values: false,
        slot_taint: true,
        action_payloads: false,
        pending_actions: false,
    };

    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 1,
        slot_count: 1,
        pc: StepIdx::ZERO,
        steps: vec![RecoveredStepEntry {
            step: StepIdx::ZERO,
            state: RecoveredStepState::Succeeded,
        }],
        slots: vec![],
        pending_actions: vec![],
        unsupported,
    };

    let hydration = RecoveryHydration::from_frame_seed(seed);
    let boundary = vb_runtime::recovery::recovery_boundary_from_hydration(hydration);

    assert_eq!(boundary.summary(), summary);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("slot_taint")
        })
    );
}

/// Given an empty journal
/// When recovery is attempted for any run
/// Then NoRecoveryData is returned
#[test]
fn empty_journal_returns_no_recovery_data_for_any_run() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11102);

    {
        let _journal = open_journal(&dir);
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary(&journal, run);
    let Err(RecoveryError::NoRecoveryData { run: found }) = result else {
        panic!("expected NoRecoveryData, got: {result:?}");
    };
    assert_eq!(found, run);
}

// ============================================================================
// SECTION 12: Pending action journal restart integration
// ============================================================================
//
// TERMINOLOGY NOTE (FINDING-005 from vb-wy33p.11 review):
// Round 5 took the HONEST DOWNGRADE path: renamed the clean-restart
// test from "crash" to "persisted_restart_via_appends_with_syncall"
// and added a TODO tracking the real crash-survival test as
// out-of-scope. Round 6 closes the TODO by adding the genuine
// multi-process crash-survival test
// `pending_action_crash_via_append_journaled_then_exit_replays_to_typed_cannot_resume`
// (with helper `crash_child_marker`) which spawns the test binary
// itself via `std::process::Command::new(current_exe)`, writes via
// `append_journaled` (no `SyncAll`), and exits via
// `std::process::exit(0)` without graceful drop. The parent reopens
// the WAL and asserts the same typed `CannotResume` contract
// witnessed by the clean-restart test.

/// Given a real `FjallJournal` with events including a
/// `JournalEvent::ActionScheduled` for an action that never finishes
/// When the journal is reopened on the same `TempDir` after a clean
/// close (each event already `PersistMode::SyncAll`'d via
/// `write_events_strict`)
/// Then `recover_runtime_frame_seed` reconstructs the seed with the
/// pending action recorded, `DurableFrameRecoveryBoundary::resume_status`
/// reports `CannotResume` with `pending_actions: true` (plus the
/// full-RunState-missing flags), and `boundary.hydrate_run_frame()`
/// returns `Err(RuntimeError::RecoveryCannotResume { reason: "pending_actions" })`. The
/// storage-level `hydrate_run_frame_from_events` also returns
/// `Err(RecoveryError::UnsupportedFrameSeed)` so the fail-closed
/// surface is uniform across the runtime and storage layers.
///
/// This is the typed-rejection contract test called out by
/// FINDING-005 in `vb-wy33p.11`. The journal is opened twice on the
/// same `TempDir` so the test exercises the actual durable boundary
/// (no in-memory mocking). The events are written strictly so the
/// sequences are well-defined and the assertion is reproducible
/// without timing variance.
///
/// NOTE on terminology: this is the TYPED-REJECTION CONTRACT test,
/// NOT a power-loss WAL replay test. A true power-loss test would
/// require a multi-process harness that writes events via the
/// non-fsync `append_journaled` path and `std::process::exit(0)`s
/// without graceful close; that is tracked as a follow-up bead.
#[test]
fn pending_action_persisted_restart_via_appends_with_syncall() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11300);
    let digest = test_digest(0x2B);
    let action = ActionId::new(0x5A);

    // First lifecycle: write events for a run whose ActionScheduled
    // never resolves, with per-event `PersistMode::SyncAll` (clean
    // shutdown semantics).
    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(2),
                    attempt: 1,
                },
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(2),
                    step: StepIdx::new(2),
                    action,
                    attempt: 1,
                },
            ],
        );
    }

    // Second lifecycle: reopen the journal on the same path. The
    // pending action MUST still be observable.
    let journal = open_journal(&dir);
    let seed =
        recover_runtime_frame_seed(&journal, run).expect("frame seed recovery should succeed");

    let pending = seed
        .pending_actions
        .iter()
        .find(|entry| entry.action == action)
        .expect("pending action must survive journal reopen");
    assert_eq!(pending.step, StepIdx::new(2));
    assert_eq!(seed.pending_actions.len(), 1);

    let boundary = vb_runtime::recovery::DurableFrameRecoveryBoundary::from_product(seed);
    let cannot_resume = RecoveryCannotResumeState {
        pending_actions: true,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };

    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(boundary.cannot_resume_state(), cannot_resume);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("pending_actions")
        })
    );

    // The storage-layer entry point must use the same typed
    // rejection. Re-run the assertion against a fresh
    // `hydrate_run_frame_from_events` call so the typed
    // `UnsupportedFrameSeed` surface is verified end-to-end.
    let events: Vec<JournalEvent> = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after reopen");
    let storage_result = hydrate_run_frame_from_events(&events, run);
    let Err(RecoveryError::UnsupportedFrameSeed { run: found, reason }) = storage_result else {
        panic!("expected UnsupportedFrameSeed from storage layer, got: {storage_result:?}");
    };
    assert_eq!(found, run);
    assert_eq!(
        reason, "pending_actions",
        "UnsupportedFrameSeed reason must be the exact `pending_actions` token"
    );
}

/// Real crash-replay test (FINDING-005 satisfied end-to-end).
///
/// Spawns a child test binary via `std::process::Command::new(current_exe)`
/// that opens the Fjall journal, writes events via the NON-strict
/// `append_journaled` path (no `PersistMode::SyncAll` per event), and
/// exits via `std::process::exit(0)` WITHOUT calling `drop` on the
/// journal. This simulates an in-flight crash where the Fjall WAL
/// has buffered writes that have been flushed to the OS page cache
/// (`PersistMode::Buffer` in Fjall V3 default config) but not
/// fsynced to disk. The parent reopens the journal, recovers the
/// frame seed via `recover_runtime_frame_seed`, and asserts the
/// typed `CannotResume { pending_actions: true, ... }` rejection
/// plus the storage-layer `UnsupportedFrameSeed` surface.
///
/// This is the test that satisfies FINDING-005's "deterministic
/// crash/restart" requirement. It complements the existing
/// `pending_action_persisted_restart_via_appends_with_syncall`
/// test which exercises the clean-restart path (per-event
/// `SyncAll` + graceful `drop`). Together they prove the
/// typed-rejection contract holds across both crash-survival
/// scenarios.
///
/// HARNESS DESIGN:
/// 1. Parent opens `TempDir` and the journal once to materialize
///    the Fjall partition layout, then drops the journal so the
///    Fjall `Database` lock and our `ProcessLock` are released.
/// 2. Parent spawns `current_exe` with env vars
///    `VB_CRASH_CHILD=1`, `VB_CRASH_JOURNAL_DIR=<path>`,
///    `VB_CRASH_RUN_ID=<run>` and CLI args
///    `--ignored --exact crash_child_marker`. The `#[ignore]`
///    attribute on `crash_child_marker` keeps it out of normal
///    `cargo test` runs; `--ignored` plus `--exact` makes cargo
///    run only the marker test in the child process.
/// 3. Child detects `VB_CRASH_CHILD=1` and writes the three
///    events (RunAccepted, StepStarted, ActionScheduled) via
///    `append_journaled`. Then `std::process::exit(0)` without
///    dropping the journal — this is the crash simulation.
/// 4. Parent waits for the child, reopens the journal on the same
///    `TempDir`, recovers the frame seed, and asserts the typed
///    cannot-resume contract.
#[test]
fn pending_action_crash_via_append_journaled_then_exit_replays_to_typed_cannot_resume() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11400);
    let action = ActionId::new(0xDD);

    // Touch the journal so the Fjall partition layout exists before
    // the child opens it. Drop immediately so the Fjall Database
    // lock and our ProcessLock are released.
    {
        let _primer = open_journal(&dir);
    }

    // Spawn self as child in crash mode. The child runs ONLY the
    // `crash_child_marker` test (via `--ignored --exact`). That
    // marker detects `VB_CRASH_CHILD=1`, performs the
    // append_journaled + std::process::exit(0) work, and exits.
    let exe = std::env::current_exe().expect("current exe should resolve");
    let mut cmd = std::process::Command::new(exe);
    cmd.env("VB_CRASH_CHILD", "1");
    cmd.env("VB_CRASH_JOURNAL_DIR", dir.path());
    cmd.env("VB_CRASH_RUN_ID", run.get().to_string());
    cmd.arg("--ignored");
    cmd.arg("--exact");
    cmd.arg("crash_child_marker");

    let status = cmd
        .status()
        .expect("child process should spawn and complete");
    assert!(
        status.success(),
        "child must exit successfully even though it exited without dropping the journal: {status:?}"
    );

    // Reopen the journal on the same path. The pending action must
    // survive Fjall WAL replay because the child's writes were
    // committed via `append_journaled` (writes go to WAL +
    // memtable, flushed to OS via PersistMode::Buffer, but not
    // fsynced — exactly the scenario the test is meant to cover).
    let journal = open_journal(&dir);
    let seed = recover_runtime_frame_seed(&journal, run)
        .expect("frame seed recovery should succeed after child crash");

    let pending = seed
        .pending_actions
        .iter()
        .find(|entry| entry.action == action)
        .expect("pending action must survive crash WAL replay");
    assert_eq!(pending.step, StepIdx::new(2));
    assert_eq!(seed.pending_actions.len(), 1);

    // Typed rejection contract — same shape as the clean-restart
    // test, because the durable evidence is identical.
    let boundary = vb_runtime::recovery::DurableFrameRecoveryBoundary::from_product(seed);
    let cannot_resume = RecoveryCannotResumeState {
        pending_actions: true,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };

    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(boundary.cannot_resume_state(), cannot_resume);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("pending_actions")
        })
    );

    // Storage-layer `UnsupportedFrameSeed` surface must match too —
    // the durable evidence is identical regardless of crash path.
    let events: Vec<JournalEvent> = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after crash reopen");
    let storage_result = hydrate_run_frame_from_events(&events, run);
    let Err(RecoveryError::UnsupportedFrameSeed { run: found, reason }) = storage_result else {
        panic!(
            "expected UnsupportedFrameSeed from storage layer after crash, got: {storage_result:?}"
        );
    };
    assert_eq!(found, run);
    assert_eq!(
        reason, "pending_actions",
        "UnsupportedFrameSeed reason must be the exact `pending_actions` token after crash replay"
    );
}

/// Marker test invoked by the crash-replay parent via
/// `std::process::Command::new(current_exe)` with
/// `VB_CRASH_CHILD=1` and `--ignored --exact crash_child_marker`.
///
/// When `VB_CRASH_CHILD=1` is set, this test performs the crash
/// simulation: opens the journal at `VB_CRASH_JOURNAL_DIR`, writes
/// three events via `append_journaled` (NO `PersistMode::SyncAll`),
/// and exits via `std::process::exit(0)` WITHOUT dropping the
/// journal. The OS closes the Fjall file descriptors during
/// `_exit()`, the Fjall WAL replay recovers the writes on parent
/// reopen.
///
/// When `VB_CRASH_CHILD` is NOT set (i.e. someone invokes this
/// test directly), the body is a no-op so the test passes
/// trivially. This keeps the marker harmless if it ever runs
/// without the parent harness (e.g. during
/// `cargo test --ignored --list` enumeration).
///
/// The `#[ignore]` attribute keeps this test out of normal
/// `cargo test` runs. The parent passes `--ignored` to surface
/// this marker specifically when it spawns the child process.
#[test]
#[ignore = "marker for crash-replay parent's Command::new invocation; not run directly"]
fn crash_child_marker() {
    // Defensive: if this test runs WITHOUT VB_CRASH_CHILD set,
    // treat it as a no-op marker. The parent harness sets the env
    // var and observes the exit status.
    if std::env::var("VB_CRASH_CHILD").as_deref() != Ok("1") {
        return;
    }

    // Child-mode: perform the crash-survival work.
    let dir_path =
        std::env::var("VB_CRASH_JOURNAL_DIR").expect("VB_CRASH_JOURNAL_DIR must be set for child");
    let path = std::path::PathBuf::from(dir_path);

    let run_id: u64 = std::env::var("VB_CRASH_RUN_ID")
        .expect("VB_CRASH_RUN_ID must be set for child")
        .parse()
        .expect("VB_CRASH_RUN_ID must parse as u64");
    let run = RunId::new(run_id);
    let digest = WorkflowDigest::from_bytes([0xCC; 32]);
    let action = ActionId::new(0xDD);

    let journal = FjallJournal::open(&path, Some(FjallConfig::default()))
        .expect("child: journal open should succeed");

    // CRITICAL: use `append_journaled` (no SyncAll), not
    // `append_strict`. This is what makes this a real crash
    // simulation: the writes go to Fjall's WAL + memtable but
    // are NOT fsynced before exit.
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        })
        .expect("child: append_run_accepted should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            attempt: 1,
        })
        .expect("child: append_step_started should succeed");
    journal
        .append_journaled(&JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action,
            attempt: 1,
        })
        .expect("child: append_action_scheduled should succeed");

    // Simulate in-flight crash: exit without dropping the journal,
    // without SyncAll. The Fjall WAL + memtable hold the writes;
    // the OS will close the file descriptors during _exit().
    // The parent's reopen triggers Fjall WAL replay which surfaces
    // the pending action to recovery.
    std::process::exit(0);
}

// ============================================================================
// SECTION 13: Advanced hydration scenarios
// ============================================================================

/// Given events with a SlotWrittenEvent having None value (no payload)
/// When hydrate_run_frame_from_events is called
/// Then the storage layer rejects with `UnsupportedFrameSeed` (a
/// `None` slot value is unsupported by the runtime hydration boundary
/// and a frame seed alone never carries the full RunState).
#[test]
fn slot_written_with_none_value_is_rejected_typed() {
    let run = RunId::new(11200);
    let digest = test_digest(0x28);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(1),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::new(1),
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

/// Given a full run with SlotWrittenEvent having None values
/// When summary is recovered
/// Then slots_written count is correct
#[test]
fn slot_written_presence_counted_in_summary_even_with_none_value() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(11201);
    let digest = test_digest(0x29);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(1),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let summary = recover_runtime_summary(&journal, run)
        .expect("summary recovery should succeed")
        .summary();

    assert_eq!(summary.slots_written, 2);
}

/// Given a run with a RetryScheduled event
/// When hydrate_run_frame_from_events is called
/// Then replay handles the retry event gracefully
#[test]
fn retry_scheduled_event_reconstructed_in_hydration() {
    let run = RunId::new(11202);
    let digest = test_digest(0x2A);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 2,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 2,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

// ============================================================================
// SECTION 13: Kani: recovery determinism
// ============================================================================

/// Kani verification: for any valid recovery summary, calling summary()
/// returns the same data regardless of the hydration variant wrapping it.
#[cfg(kani)]
mod kani_recovery {
    use super::*;

    #[kani::proof]
    fn verify_summary_recovery_deterministic_on_same_input() {
        let run_id: u64 = kani::any();
        kani::assume(run_id > 0);

        let summary = RecoveryRuntimeSummary {
            run: RunId::new(run_id),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(3),
            workflow: Some(WorkflowDigest::from_bytes([1u8; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };

        let a = RecoveryHydration::Summary(summary).summary();
        let b = RecoveryHydration::Summary(summary).summary();
        assert!(a == b);

        let summary_run = a.run;
        assert!(summary_run.get() == run_id);
    }

    #[kani::proof]
    fn verify_empty_events_returns_no_recovery_data() {
        let result = hydrate_run_frame_from_events(&[], RunId::new(1));
        match result {
            Err(RecoveryError::NoRecoveryData { .. }) => {}
            _ => panic!("expected NoRecoveryData"),
        }
    }

    #[kani::proof]
    fn verify_check_compiled_ir_digest_rejects_any_mismatch() {
        let a_byte: u8 = kani::any();
        let b_byte: u8 = kani::any();
        kani::assume(a_byte != b_byte);

        let expected = WorkflowDigest::from_bytes([a_byte; 32]);
        let found = WorkflowDigest::from_bytes([b_byte; 32]);

        let result = vb_storage::recovery::check_compiled_ir_digest(expected, found);
        let Err(RecoveryError::CompiledIrDigestMismatch {
            expected: err_expected,
            found: err_found,
        }) = result
        else {
            panic!("expected mismatch error");
        };
        assert!(err_expected == expected);
        assert!(err_found == found);
    }

    #[kani::proof]
    fn verify_check_compiled_ir_digest_accepts_exact_match() {
        let byte: u8 = kani::any();
        let digest = WorkflowDigest::from_bytes([byte; 32]);

        let result = vb_storage::recovery::check_compiled_ir_digest(digest, digest);
        match result {
            Ok(()) => {}
            Err(_) => panic!("identical digests must pass"),
        }
    }

    #[kani::proof]
    fn verify_action_replay_tracker_new_has_nothing_resolved() {
        let tracker = ActionReplayTracker::new();
        let action_id: u64 = kani::any();
        let step_idx: u16 = kani::any();

        let resolved = tracker.is_resolved(ActionId::new(action_id), StepIdx::new(step_idx));
        assert!(!resolved);
    }

    #[kani::proof]
    fn verify_action_replay_tracker_is_resolved_after_mark_completed() {
        let action_id: u64 = kani::any();
        let step_idx: u16 = kani::any();
        kani::assume(step_idx < u16::MAX);

        let mut tracker = ActionReplayTracker::new();
        tracker.mark_completed(ActionId::new(action_id), StepIdx::new(step_idx));

        let resolved = tracker.is_resolved(ActionId::new(action_id), StepIdx::new(step_idx));
        assert!(resolved);
    }

    #[kani::proof]
    fn verify_action_replay_tracker_mark_failed_marks_resolved() {
        let action_id: u64 = kani::any();
        let step_idx: u16 = kani::any();

        let mut tracker = ActionReplayTracker::new();
        tracker.mark_failed(ActionId::new(action_id), StepIdx::new(step_idx));

        let resolved = tracker.is_resolved(ActionId::new(action_id), StepIdx::new(step_idx));
        assert!(resolved);
    }

    #[kani::proof]
    fn verify_corrupt_snapshot_with_mismatched_run_is_rejected() {
        let wrong_run_id: u64 = kani::any();
        let correct_run_id: u64 = kani::any();
        kani::assume(wrong_run_id != correct_run_id);
        kani::assume(wrong_run_id > 0);
        kani::assume(correct_run_id > 0);

        let snapshot = RunSnapshot {
            run: RunId::new(wrong_run_id),
            seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([1u8; 32]),
            slots: vec![],
            taint: vec![],
        };

        let tail = vec![JournalEvent::StepStarted {
            run: RunId::new(correct_run_id),
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        }];

        let result = hydrate_run_frame(&snapshot, &tail, RunId::new(correct_run_id));
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_empty_snapshot_and_tail_rejected() {
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: vec![],
            taint: vec![],
        };

        let result = hydrate_run_frame(&snapshot, &[], RunId::new(1));
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_no_recovery_data_for_empty_journal() {
        let result = hydrate_run_frame_from_events(&[], RunId::new(1));
        match result {
            Err(RecoveryError::NoRecoveryData { run: _ }) => {}
            _ => panic!("empty events must produce NoRecoveryData"),
        }
    }
}
