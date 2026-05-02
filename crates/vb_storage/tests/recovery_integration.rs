//! Crash-recovery integration tests for the Fjall-backed storage layer.
//!
//! Tests full round-trip recovery, partial write detection,
//! Strict vs Journaled durability, and action replay tracking.

#![forbid(unsafe_code)]

use tempfile::TempDir;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    extract_terminal, is_terminal_event, recover_full_journal, recover_runtime_summary,
    ActionReplayTracker, RecoveryError, RecoveryHydration, RecoveryTerminalState,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent,
    JournalWriterQueue, StorageLimits};

/// Helper: creates a deterministic workflow digest from a single byte.
fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

/// Helper: opens a FjallJournal in the given temp directory.
fn open_journal(dir: &TempDir) -> FjallJournal {
    FjallJournal::open(dir.path(), Some(FjallConfig::default()))
        .expect("journal open should succeed")
}

/// Helper: builds the complete set of journal events for a two-step run.
fn build_full_run_events(run: RunId, digest: WorkflowDigest) -> Vec<JournalEvent> {
    let mut events = Vec::new();
    let mut seq = 0u64;

    // RunAccepted
    events.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: digest,
    });
    seq = seq.saturating_add(1);

    // Step 0: start, write slots, succeed
    events.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(0),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(0),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(1),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(0),
        output: SlotIdx::new(1),
    });
    seq = seq.saturating_add(1);

    // Step 1: start, write slot, succeed
    events.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(1),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot: SlotIdx::new(2),
    });
    seq = seq.saturating_add(1);

    events.push(JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(1),
        output: SlotIdx::new(2),
    });
    seq = seq.saturating_add(1);

    // RunFinished
    events.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(2),
    });

    events
}

/// Helper: writes events to the journal using the given durability method.
fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) {
    for event in events {
        journal
            .append_strict(event)
            .expect("strict append should succeed");
    }
}

// ---------------------------------------------------------------------------
// Test A: Full round-trip recovery
// ---------------------------------------------------------------------------

#[test]
fn full_round_trip_recovery_reads_all_events_in_order() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(100);
    let digest = test_digest(0xAA);
    let original_events = build_full_run_events(run, digest);

    // Phase 1: Write all events with strict durability.
    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &original_events);
    }
    // Journal is dropped here, simulating a clean shutdown.

    // Phase 2: Reopen and read all events back.
    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");

    assert_eq!(
        recovered.len(),
        original_events.len(),
        "recovered event count must match original"
    );

    // Verify every event is present in order with matching fields.
    for (i, (original, recovered_event)) in
        original_events.iter().zip(recovered.iter()).enumerate()
    {
        assert_eq!(
            original, recovered_event,
            "event at index {i} must match after recovery"
        );
    }
}

#[test]
fn full_round_trip_recovery_reconstructs_summary() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(101);
    let digest = test_digest(0xBB);
    let events = build_full_run_events(run, digest);

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let hydration = recover_runtime_summary(&journal, run)
        .expect("recover_runtime_summary should succeed");

    match hydration {
        RecoveryHydration::Summary(summary) => {
            assert_eq!(summary.run, run);
            assert_eq!(summary.workflow, Some(digest));
            assert_eq!(summary.steps_started, 2, "two StepStarted events");
            assert_eq!(summary.steps_succeeded, 2, "two StepSucceeded events");
            assert_eq!(summary.slots_written, 3, "three SlotWritten events");
            assert_eq!(
                summary.terminal,
                Some(RecoveryTerminalState::Finished {
                    result: SlotIdx::new(2)
                }),
                "run should be finished with result slot 2"
            );
        }
        RecoveryHydration::FrameSeed(_) => {
            panic!("expected Summary hydration, got FrameSeed");
        }
    }
}

#[test]
fn full_round_trip_recovery_detects_slot_writes() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(102);
    let digest = test_digest(0xCC);

    {
        let journal = open_journal(&dir);
        let events = build_full_run_events(run, digest);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");

    // Count SlotWrittenEvent variants.
    let mut slot_count = 0u64;
    let mut slot_indices = Vec::new();
    for event in &recovered {
        if let JournalEvent::SlotWrittenEvent { slot, .. } = event {
            slot_count = slot_count.saturating_add(1);
            slot_indices.push(slot.get());
        }
    }

    assert_eq!(slot_count, 3, "three slot writes expected");
    assert_eq!(slot_indices.len(), 3);
    assert_eq!(slot_indices[0], 0, "slot 0");
    assert_eq!(slot_indices[1], 1, "slot 1");
    assert_eq!(slot_indices[2], 2, "slot 2");
}

// ---------------------------------------------------------------------------
// Test B: Partial write recovery
// ---------------------------------------------------------------------------

#[test]
fn partial_write_recovery_reads_events_written_before_crash() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(200);
    let digest = test_digest(0xDD);

    // Write only the first 4 events (RunAccepted, StepStarted, SlotWritten, SlotWritten)
    // then drop the journal without RunFinished.
    let partial_events = {
        let all = build_full_run_events(run, digest);
        let mut partial = Vec::new();
        for (i, event) in all.iter().enumerate() {
            if i >= 4 {
                break;
            }
            partial.push(event.clone());
        }
        partial
    };

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &partial_events);
    }

    // Reopen and verify partial events are readable.
    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed for partial run");

    assert_eq!(
        recovered.len(),
        partial_events.len(),
        "recovered partial event count must match"
    );

    for (i, (original, recovered_event)) in
        partial_events.iter().zip(recovered.iter()).enumerate()
    {
        assert_eq!(
            original, recovered_event,
            "partial event at index {i} must match"
        );
    }
}

#[test]
fn partial_write_recovery_detects_incomplete_state() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(201);
    let digest = test_digest(0xEE);

    // Write RunAccepted + StepStarted only (no terminal event).
    let partial_events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &partial_events);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");

    // Verify there is NO terminal event.
    let terminal = extract_terminal(&recovered);
    assert!(
        terminal.is_none(),
        "partial run must not have a terminal event"
    );

    // Verify recovery summary has no terminal state.
    let hydration = recover_runtime_summary(&journal, run)
        .expect("recover_runtime_summary should succeed for partial run");

    match hydration {
        RecoveryHydration::Summary(summary) => {
            assert!(
                summary.terminal.is_none(),
                "partial run summary must have no terminal state"
            );
            assert_eq!(summary.steps_started, 1);
            assert_eq!(summary.steps_succeeded, 0);
        }
        RecoveryHydration::FrameSeed(_) => {
            panic!("expected Summary hydration for partial run");
        }
    }
}

#[test]
fn partial_write_with_only_run_accepted_is_recoverable() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(202);
    let digest = test_digest(0xFF);

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
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");

    assert_eq!(recovered.len(), 1);
    assert!(matches!(
        recovered.first(),
        Some(JournalEvent::RunAccepted { .. })
    ));

    let hydration = recover_runtime_summary(&journal, run)
        .expect("summary recovery should succeed");
    match hydration {
        RecoveryHydration::Summary(summary) => {
            assert_eq!(summary.run, run);
            assert_eq!(summary.workflow, Some(digest));
            assert_eq!(summary.steps_started, 0);
            assert_eq!(summary.terminal, None);
        }
        RecoveryHydration::FrameSeed(_) => {
            panic!("expected Summary hydration");
        }
    }
}

// ---------------------------------------------------------------------------
// Test C: Strict vs Journaled durability
// ---------------------------------------------------------------------------

#[test]
fn strict_durability_survives_immediate_reopen() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(300);
    let digest = test_digest(0x11);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after strict writes");

    assert_eq!(recovered.len(), events.len());
    for (i, (original, recovered_event)) in events.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(original, recovered_event, "strict event {i} must match");
    }
}

#[test]
fn journaled_durability_appears_after_flush() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(301);
    let digest = test_digest(0x22);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
    ];

    {
        let journal = open_journal(&dir);
        let queue = JournalWriterQueue::new(64, 16, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");

        // Enqueue events as journaled (not strict).
        for event in &events {
            queue
                .enqueue_journaled(event.clone())
                .expect("enqueue should succeed");
        }

        // Flush the queue to persist journaled writes.
        let report = queue
            .drain_all(&journal)
            .expect("drain_all should succeed");
        assert_eq!(report.drained, 2, "both events should be drained");
        assert_eq!(report.written, 2, "both events should be written");
    }

    // Reopen and verify events survived.
    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after journaled flush");

    assert_eq!(
        recovered.len(),
        events.len(),
        "journaled events must survive after flush and reopen"
    );
    for (i, (original, recovered_event)) in events.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(
            original, recovered_event,
            "journaled event {i} must match after flush"
        );
    }
}

#[test]
fn journaled_queue_shutdown_drains_all_events() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(302);
    let digest = test_digest(0x33);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(5),
        },
    ];

    {
        let journal = open_journal(&dir);
        let queue = JournalWriterQueue::new(64, 16, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");

        for event in &events {
            queue
                .enqueue_journaled(event.clone())
                .expect("enqueue should succeed");
        }

        // Shutdown drains all remaining writes.
        let report = queue
            .shutdown(&journal)
            .expect("shutdown should succeed");
        assert_eq!(report.drained, 3, "all three events should be drained");
        assert_eq!(report.written, 3, "all three events should be written");
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(recovered.len(), events.len());
}

#[test]
fn strict_batch_writes_are_atomic() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(303);
    let digest = test_digest(0x44);

    let events = build_full_run_events(run, digest);

    {
        let journal = open_journal(&dir);
        journal
            .append_strict_batch(&events)
            .expect("strict batch append should succeed");
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed after batch");

    assert_eq!(recovered.len(), events.len());
    for (i, (original, recovered_event)) in events.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(original, recovered_event, "batch event {i} must match");
    }
}

// ---------------------------------------------------------------------------
// Test D: Action replay tracking
// ---------------------------------------------------------------------------

#[test]
fn action_replay_tracker_reconstructs_from_events() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(400);
    let digest = test_digest(0x55);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: ActionId::new(10),
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: ActionId::new(10),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::new(0),
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker)
        .expect("recover_full_journal should succeed");

    assert_eq!(replayed.len(), events.len());

    // Tracker should know that action 10 at step 0 was completed.
    assert!(
        tracker.is_resolved(ActionId::new(10), StepIdx::new(0)),
        "action 10 at step 0 should be resolved (completed)"
    );

    // A different action should not be resolved.
    assert!(
        !tracker.is_resolved(ActionId::new(20), StepIdx::new(0)),
        "action 20 at step 0 should not be resolved"
    );
}

#[test]
fn action_replay_tracker_tracks_failed_actions() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(401);
    let digest = test_digest(0x66);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: ActionId::new(11),
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: ActionId::new(11),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker)
        .expect("recover_full_journal should succeed");

    assert_eq!(replayed.len(), events.len());

    // Failed action should also be resolved.
    assert!(
        tracker.is_resolved(ActionId::new(11), StepIdx::new(0)),
        "action 11 at step 0 should be resolved (failed)"
    );
}

#[test]
fn action_replay_blocks_duplicate_scheduled_action() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(402);
    let digest = test_digest(0x77);

    // Schedule action, complete it, then schedule it again (divergence).
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: ActionId::new(12),
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: ActionId::new(12),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            action: ActionId::new(12),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker);

    assert!(
        result.is_err(),
        "recovery should fail when a completed action is rescheduled"
    );
    match result {
        Err(RecoveryError::NonIdempotentActionBlocked { action, step }) => {
            assert_eq!(action, ActionId::new(12));
            assert_eq!(step, StepIdx::new(0));
        }
        Err(other) => {
            panic!("expected NonIdempotentActionBlocked, got: {other}");
        }
        Ok(_) => {
            panic!("recovery should have failed with NonIdempotentActionBlocked");
        }
    }
}

#[test]
fn empty_run_returns_no_recovery_data() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(999);

    // Open and close the journal without writing anything for this run.
    {
        let _journal = open_journal(&dir);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker);

    assert!(
        result.is_err(),
        "recovery for a run with no events should fail"
    );
    match result {
        Err(RecoveryError::NoRecoveryData { run: found_run }) => {
            assert_eq!(found_run, run);
        }
        Err(other) => {
            panic!("expected NoRecoveryData, got: {other}");
        }
        Ok(_) => {
            panic!("should have failed with NoRecoveryData");
        }
    }
}

#[test]
fn terminal_event_identification_after_recovery() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(500);
    let digest = test_digest(0x88);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let recovered = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");

    // The last event should be terminal.
    let last = recovered
        .last()
        .expect("should have at least one event");
    assert!(
        is_terminal_event(last),
        "last event must be terminal (RunFinished)"
    );

    let terminal = extract_terminal(&recovered);
    assert!(
        terminal.is_some(),
        "extract_terminal must find the RunFinished event"
    );
    if let Some(JournalEvent::RunFinished { result, .. }) = terminal {
        assert_eq!(*result, SlotIdx::new(0));
    } else {
        panic!("expected RunFinished terminal event");
    }
}

#[test]
fn recovery_across_multiple_runs_is_isolated() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run_a = RunId::new(600);
    let run_b = RunId::new(601);
    let digest_a = test_digest(0xA1);
    let digest_b = test_digest(0xB2);

    let events_a = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: digest_a,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        },
        JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
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
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events_a);
        write_events_strict(&journal, &events_b);
    }

    let journal = open_journal(&dir);

    // Run A should recover independently.
    let recovered_a = journal
        .events_for_run(run_a)
        .expect("run A events should exist");
    assert_eq!(recovered_a.len(), 3);
    assert_eq!(recovered_a.first().map(|e| e.run_id()), Some(run_a));

    // Run B should recover independently.
    let recovered_b = journal
        .events_for_run(run_b)
        .expect("run B events should exist");
    assert_eq!(recovered_b.len(), 2);
    assert_eq!(recovered_b.first().map(|e| e.run_id()), Some(run_b));

    // Verify terminal states differ.
    let summary_a = recover_runtime_summary(&journal, run_a)
        .expect("summary for run A should succeed");
    let summary_b = recover_runtime_summary(&journal, run_b)
        .expect("summary for run B should succeed");

    match summary_a {
        RecoveryHydration::Summary(s) => {
            assert!(matches!(
                s.terminal,
                Some(RecoveryTerminalState::Finished { .. })
            ));
        }
        RecoveryHydration::FrameSeed(_) => {
            panic!("expected Summary");
        }
    }

    match summary_b {
        RecoveryHydration::Summary(s) => {
            assert_eq!(s.terminal, Some(RecoveryTerminalState::Cancelled));
        }
        RecoveryHydration::FrameSeed(_) => {
            panic!("expected Summary");
        }
    }
}
