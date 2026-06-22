#![allow(
    unused_imports,
    dead_code,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]
//! Regression tests for vb-1rqz7.{1,2,3,4,5,6,7,8} — eight vb_storage P0 bugs
//! found during the 2026-06-21 bug hunt.
//!
//! Each test exercises the production code path that was previously buggy and
//! asserts the fixed behaviour via the public API only.
//!
//! Beads covered:
//! - vb-1rqz7.1 / SJ-002 — `inject_seq_gap` no longer masquerades as `RunCancelled`
//! - vb-1rqz7.2 / SJ-003 — `inject_raw_event` / `inject_seq_gap` acquire the
//!   write lock and reject duplicate keys
//! - vb-1rqz7.3 / SJ-005 — `derive_lifecycle_state_from_events` classifies
//!   every current `JournalEvent` variant explicitly (no wildcard)
//! - vb-1rqz7.4 / SR-001 — `recover_full_journal` reads the full event
//!   history despite any durable snapshot for the run
//! - vb-1rqz7.5 / SR-002 — public recovery APIs read the full history so
//!   pre-snapshot `RunAccepted`/`RunAdmission` events are not silently dropped
//! - vb-1rqz7.8 / SR-006 — snapshot+tail recovery rejects a cross-snapshot gap
//!   at `snapshot.seq + 1`

use crate::{
    DurableActionOutcome, EventSeq, JournalError, JournalEvent, RunSnapshot,
    constants::DIGEST_BYTES,
    recovery::RecoveryTerminalState,
    recovery::{
        self, ActionReplayTracker, check_workflow_source_digest, recover_run_admission,
        recover_runtime_frame_seed, recover_runtime_summary, recover_runtime_summary_with_expected,
        replay::{is_terminal_event, load_snapshot, recover_full_journal},
    },
    test_helpers::make_temp_journal_pair,
};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

// =========================================================================
// vb-1rqz7.4 / SR-001 — recover_full_journal reads full history
// =========================================================================

#[test]
fn recover_full_journal_reads_history_before_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(0x5E0);
    let workflow = WorkflowDigest::from_bytes([0xA1; DIGEST_BYTES]);

    let journal = crate::FjallJournal::open(&path, None).expect("open should succeed");

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append StepStarted should succeed");
    journal
        .append_journaled(&JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
            attempt: 1,
        })
        .expect("append RunFinished should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let tail_only = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(
        tail_only.len(),
        2,
        "events_for_run is snapshot-tail optimised; expected 2 tail events, got {:?}",
        tail_only.len()
    );

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .expect("recover_full_journal must succeed and read full history");
    assert_eq!(
        replayed.len(),
        4,
        "recover_full_journal must read pre-snapshot events too, got {:?}",
        replayed.len()
    );
    assert_eq!(replayed[0].seq(), EventSeq::new(0));
    assert_eq!(replayed[1].seq(), EventSeq::new(1));
    assert_eq!(replayed[2].seq(), EventSeq::new(2));
    assert_eq!(replayed[3].seq(), EventSeq::new(3));
}

#[test]
fn recover_full_journal_succeeds_without_snapshot() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E1);
    let workflow = WorkflowDigest::from_bytes([0xA2; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append should succeed");
    journal
        .append_journaled(&JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        })
        .expect("append should succeed");

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .expect("recover_full_journal must succeed without snapshot");
    assert_eq!(replayed.len(), 3);
}

// =========================================================================
// vb-1rqz7.3 / SJ-005 — exhaustive lifecycle classification
// =========================================================================

#[test]
fn lifecycle_classifies_run_killed_as_cancelled() {
    use crate::journal::incident::lifecycle::derive_lifecycle_state_from_events;
    use vb_core::workflow::LifecycleState;

    let killed = JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        attempt: 1,
        reason: None,
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[killed]),
        LifecycleState::Cancelled,
        "RunKilled must classify as Cancelled, never the wildcard Active"
    );
}

#[test]
fn lifecycle_classifies_action_scheduled_ticket_as_active() {
    use crate::journal::incident::lifecycle::derive_lifecycle_state_from_events;
    use vb_core::{ActionTicket, workflow::LifecycleState};

    let event = JournalEvent::ActionScheduledTicket {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        ticket: ActionTicket::default(),
        input: SlotIdx::new(0),
        output: SlotIdx::new(1),
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[event]),
        LifecycleState::Active
    );
}

#[test]
fn lifecycle_classifies_action_completed_envelope_as_active() {
    use crate::DurableActionOutcome;
    use crate::journal::incident::lifecycle::derive_lifecycle_state_from_events;
    use vb_core::{ActionTicket, Taint, workflow::LifecycleState};

    let event = JournalEvent::ActionCompletedEnvelope {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        ticket: ActionTicket::default(),
        output: SlotIdx::new(1),
        outcome: DurableActionOutcome::Ready,
        value: vec![],
        encoded_len: 0,
        taint: Taint::Clean,
        value_digest: [0u8; 32],
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[event]),
        LifecycleState::Active
    );
}

#[test]
fn lifecycle_classifies_wait_cancelled_as_active() {
    use crate::journal::incident::lifecycle::derive_lifecycle_state_from_events;
    use vb_core::workflow::LifecycleState;

    let event = JournalEvent::WaitCancelledEvent {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        step: StepIdx::new(0),
        attempt: 1,
        reason: None,
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[event]),
        LifecycleState::Active
    );
}

#[test]
fn lifecycle_classifies_ask_cancelled_as_active() {
    use crate::journal::incident::lifecycle::derive_lifecycle_state_from_events;
    use vb_core::workflow::LifecycleState;

    let event = JournalEvent::AskCancelledEvent {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        step: StepIdx::new(0),
        attempt: 1,
        reason: None,
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[event]),
        LifecycleState::Active
    );
}

// =========================================================================
// vb-1rqz7.2 / SJ-003 — injection writes take the write lock and check duplicates
// =========================================================================

#[test]
fn inject_raw_event_rejects_duplicate_key() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0xD01);
    let seq = EventSeq::new(2);

    let first_bytes = journal
        .inject_raw_event(run, seq, crate::records::RecordKind::StepStarted, b"first")
        .expect("first inject should succeed");
    let first_record = first_bytes;
    let _ = first_record;

    let result =
        journal.inject_raw_event(run, seq, crate::records::RecordKind::StepStarted, b"second");
    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == run && s == seq
        ),
        "inject_raw_event must reject duplicate (run, seq) without overwriting; got {:?}",
        result
    );

    let stored = journal
        .get_event_bytes(run, seq)
        .expect("get_event_bytes should succeed")
        .expect("first inject must remain stored");
    assert!(
        stored
            .windows(b"first".len())
            .any(|window| window == b"first"),
        "duplicate inject must not have overwritten the original 'first' record; stored bytes were {:?}",
        stored
    );
    assert!(
        !stored
            .windows(b"second".len())
            .any(|window| window == b"second"),
        "duplicate inject must not have stored the 'second' payload; stored bytes were {:?}",
        stored
    );
}

#[test]
fn inject_seq_gap_rejects_duplicate_key() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0xD02);
    let seq = EventSeq::new(7);

    journal
        .inject_seq_gap(run, seq)
        .expect("first gap inject should succeed");

    let result = journal.inject_seq_gap(run, seq);
    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == run && s == seq
        ),
        "inject_seq_gap must reject duplicate (run, seq); got {:?}",
        result
    );
}

// =========================================================================
// vb-1rqz7.1 / SJ-002 — inject_seq_gap does not write a RunCancelled event
// =========================================================================

#[test]
fn inject_seq_gap_does_not_classify_run_as_cancelled() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0xD03);
    let workflow = WorkflowDigest::from_bytes([0xA3; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append StepStarted should succeed");
    journal
        .inject_seq_gap(run, EventSeq::new(7))
        .expect("inject_seq_gap should succeed");

    assert!(
        journal
            .has_seq_gap_marker(run, EventSeq::new(7))
            .expect("has_seq_gap_marker should succeed"),
        "inject_seq_gap must persist a durable gap marker at (run, gap_seq)"
    );
    assert!(
        !journal
            .has_seq_gap_marker(run, EventSeq::new(0))
            .expect("has_seq_gap_marker should succeed"),
        "inject_seq_gap must not stamp an unrelated seq"
    );

    let events_full = journal
        .events_for_run_full(run)
        .expect("events_for_run_full should succeed");
    assert!(
        events_full.iter().all(|event| !is_terminal_event(event)),
        "gap markers must never produce a terminal event in the journal stream, \
         got {:?}",
        events_full
    );
    let has_cancelled = events_full
        .iter()
        .any(|event| matches!(event, JournalEvent::RunCancelled { .. }));
    assert!(
        !has_cancelled,
        "inject_seq_gap must not surface as a RunCancelled event; got {:?}",
        events_full
    );
    assert_eq!(
        events_full.len(),
        2,
        "events_for_run_full must surface both real events, got {:?}",
        events_full
    );
}

// Keep DIGEST_BYTES referenced so the import does not get flagged.
#[allow(dead_code)]
const _DIGEST_BYTES_USED: usize = DIGEST_BYTES;

// =========================================================================
// vb-1rqz7.32 / SR-004 — load_snapshot distinguishes missing from corrupt
// =========================================================================

#[test]
fn load_snapshot_reports_missing_when_no_row_exists() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5F0);
    let seq = EventSeq::new(7);

    let err = load_snapshot(&journal, run, seq).expect_err("missing snapshot must error");
    assert!(
        matches!(err, recovery::RecoveryError::MissingSnapshot { run: r, seq: s }
            if r == run && s == seq),
        "missing snapshot must return MissingSnapshot, got {:?}",
        err
    );
}

#[test]
fn load_snapshot_returns_snapshot_when_present() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5F1);
    let workflow = WorkflowDigest::from_bytes([0xB1; DIGEST_BYTES]);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let loaded =
        load_snapshot(&journal, run, EventSeq::new(3)).expect("present snapshot must load");
    assert_eq!(
        loaded.seq, snapshot.seq,
        "loaded snapshot must match persisted seq"
    );
    assert_eq!(
        loaded.run, snapshot.run,
        "loaded snapshot must match persisted run"
    );
}

// =========================================================================
// vb-1rqz7.5 / SR-002 — public recovery APIs read full event history
// =========================================================================

#[test]
fn check_workflow_source_digest_reads_full_history_after_snapshot() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E2);
    let workflow = WorkflowDigest::from_bytes([0xA3; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");

    // Persist a snapshot at seq 1 so the tail-only reader would skip the
    // pre-snapshot RunAccepted and RunAdmission events entirely.
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    // SR-002: digest verification must read the pre-snapshot RunAccepted event.
    check_workflow_source_digest(&journal, run, workflow)
        .expect("digest verification must succeed against the pre-snapshot RunAccepted event");
}

#[test]
fn recover_runtime_summary_includes_pre_snapshot_workflow() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E3);
    let workflow = WorkflowDigest::from_bytes([0xA4; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append StepStarted should succeed");

    // Snapshot at seq 2 — after RunAccepted and RunAdmission but before any
    // tail events. A tail-only reader would return an empty event list and
    // `NoRecoveryData`.
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let hydration = recover_runtime_summary(&journal, run)
        .expect("summary recovery must include pre-snapshot RunAccepted event");
    let summary = match hydration {
        crate::recovery::RecoveryHydration::Summary(s) => s,
        other => panic!("expected Summary hydration, got {other:?}"),
    };
    assert_eq!(
        summary.workflow,
        Some(workflow),
        "summary.workflow must be populated from the pre-snapshot RunAccepted event, got {:?}",
        summary.workflow
    );
}

#[test]
fn recover_runtime_summary_with_expected_handles_pre_snapshot_terminal() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E4);
    let workflow = WorkflowDigest::from_bytes([0xA5; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");
    journal
        .append_journaled(&JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        })
        .expect("append RunFinished should succeed");

    // Snapshot at the terminal event so the tail-only reader skips the
    // pre-snapshot terminal entirely.
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let hydration = recover_runtime_summary_with_expected(
        &journal,
        run,
        RecoveryTerminalState::Finished {
            result: SlotIdx::new(0),
        },
    )
    .expect("summary recovery must observe the pre-snapshot RunFinished event");
    let _ = hydration;
}

#[test]
fn recover_runtime_frame_seed_includes_pre_snapshot_steps() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E5);
    let workflow = WorkflowDigest::from_bytes([0xA6; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append StepStarted should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let seed = recover_runtime_frame_seed(&journal, run)
        .expect("frame seed recovery must observe the pre-snapshot StepStarted event");
    assert!(
        seed.steps.iter().any(|entry| entry.step == StepIdx::new(0)),
        "frame seed must record the pre-snapshot StepStarted event, got {:?}",
        seed.steps
    );
}

#[test]
fn recover_run_admission_includes_pre_snapshot_admission() {
    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E6);
    let workflow = WorkflowDigest::from_bytes([0xA7; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");

    let admission = recover_run_admission(&journal, run)
        .expect("admission recovery must observe the pre-snapshot RunAdmission event");
    assert!(
        admission.is_some(),
        "recover_run_admission must return Some when RunAdmission precedes the snapshot"
    );
}

// =========================================================================
// vb-1rqz7.8 / SR-006 — snapshot+tail recovery rejects cross-snapshot gap
// =========================================================================

#[test]
fn recover_snapshot_plus_tail_rejects_gap_after_snapshot_seq() {
    use crate::recovery::recover_snapshot_plus_tail;

    let (_temp, journal) = make_temp_journal_pair();
    let run = RunId::new(0x5E7);
    let workflow = WorkflowDigest::from_bytes([0xA8; DIGEST_BYTES]);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("append RunAccepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        })
        .expect("append RunAdmission should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append StepStarted should succeed");
    journal
        .append_journaled(&JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        })
        .expect("append StepSucceeded should succeed");

    // Snapshot covers seq 0..=2. The tail deliberately starts at seq 4,
    // skipping seq 3 — a gap that the old per-event `event.seq > snapshot.seq`
    // check would silently accept.
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    let tail = vec![JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(4),
        result: SlotIdx::new(0),
        attempt: 1,
    }];
    let mut tracker = ActionReplayTracker::new();

    let Err(err) = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker) else {
        panic!("recover_snapshot_plus_tail must reject a tail whose first event skips seq 3");
    };
    assert!(
        matches!(err, crate::recovery::RecoveryError::ReplayDivergence { ref detail, .. }
            if detail.contains("not contiguous with snapshot seq")),
        "expected contiguity rejection, got {:?}",
        err
    );
}
