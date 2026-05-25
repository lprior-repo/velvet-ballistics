#![forbid(unsafe_code)]
//! Recovery watermark cross-check tests.
//!
//! Verifies that sequence watermarks (first_seq, last_seq, snapshot seq)
//! maintain expected invariants across recovery paths.

use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryHydration, RunSnapshot,
    recover_runtime_frame_seed_from_events, recover_runtime_summary,
    recover_snapshot_plus_tail, summarize_recovery_events,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

fn sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn accepted_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: sample_digest(1),
    }
}

fn started_event(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn succeeded_event(run: RunId, seq: u64, step: u16, output: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        output: SlotIdx::new(output),
    }
}

fn finished_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(0),
        attempt: 1,
    }
}

fn make_snapshot(run: RunId, seq: u64) -> RunSnapshot {
    RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: sample_digest(1),
        slots: vec![0, 1],
        taint: vec![0, 0],
    }
}

// ---------------------------------------------------------------------------
// Core watermark invariant tests
// ---------------------------------------------------------------------------

#[test]
fn watermark_first_seq_equals_first_event_seq() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 0),
        started_event(run, 1, 0),
        succeeded_event(run, 2, 0, 0),
    ];

    let summary = summarize_recovery_events(&events)
        .expect("summarize should succeed")
        .summary();

    assert_eq!(summary.first_seq, EventSeq::new(0));
}

#[test]
fn watermark_last_seq_equals_last_event_seq() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 0),
        started_event(run, 1, 0),
        succeeded_event(run, 2, 0, 0),
    ];

    let summary = summarize_recovery_events(&events)
        .expect("summarize should succeed")
        .summary();

    assert_eq!(summary.last_seq, EventSeq::new(2));
}

#[test]
fn watermark_first_seq_le_last_seq() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 5),
        started_event(run, 6, 0),
        succeeded_event(run, 7, 0, 0),
    ];

    let summary = summarize_recovery_events(&events)
        .expect("summarize should succeed")
        .summary();

    assert!(
        summary.first_seq <= summary.last_seq,
        "first_seq {} must be <= last_seq {}",
        summary.first_seq.get(),
        summary.last_seq.get()
    );
}

#[test]
fn watermark_single_event_has_equal_first_and_last_seq() {
    let run = RunId::new(1);
    let events = vec![accepted_event(run, 42)];

    let summary = summarize_recovery_events(&events)
        .expect("summarize should succeed")
        .summary();

    assert_eq!(summary.first_seq, summary.last_seq);
    assert_eq!(summary.first_seq, EventSeq::new(42));
}

// ---------------------------------------------------------------------------
// Snapshot + tail watermark cross-checks
// ---------------------------------------------------------------------------

#[test]
fn watermark_snapshot_seq_lt_summary_first_seq() {
    let run = RunId::new(1);
    let snapshot = make_snapshot(run, 2);
    let tail = vec![
        started_event(run, 3, 0),
        succeeded_event(run, 4, 0, 0),
    ];

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
        .expect("snapshot+tail should succeed");

    // replay_events returns the tail events; the summary is built from them
    let summary = summarize_recovery_events(&replayed)
        .expect("summarize replayed should succeed")
        .summary();

    assert!(
        snapshot.seq < summary.first_seq,
        "snapshot seq {} must be < summary first_seq {}",
        snapshot.seq.get(),
        summary.first_seq.get()
    );
}

#[test]
fn watermark_snapshot_plus_tail_first_seq_equals_tail_first_event() {
    let run = RunId::new(1);
    let snapshot = make_snapshot(run, 5);
    let tail = vec![
        started_event(run, 6, 0),
        succeeded_event(run, 7, 0, 0),
    ];

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
        .expect("snapshot+tail should succeed");

    let summary = summarize_recovery_events(&replayed)
        .expect("summarize should succeed")
        .summary();

    assert_eq!(summary.first_seq, EventSeq::new(6));
    assert_eq!(summary.last_seq, EventSeq::new(7));
}

#[test]
fn watermark_snapshot_plus_tail_rejects_event_at_snapshot_seq() {
    let run = RunId::new(1);
    let snapshot = make_snapshot(run, 5);
    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(5),
        step: StepIdx::new(0),
        attempt: 1,
    }];

    let mut tracker = ActionReplayTracker::new();
    let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);

    assert!(result.is_err(), "tail event at snapshot seq must be rejected");
}

#[test]
fn watermark_snapshot_plus_tail_rejects_event_before_snapshot_seq() {
    let run = RunId::new(1);
    let snapshot = make_snapshot(run, 5);
    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(4),
        step: StepIdx::new(0),
        attempt: 1,
    }];

    let mut tracker = ActionReplayTracker::new();
    let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);

    assert!(
        result.is_err(),
        "tail event before snapshot seq must be rejected"
    );
}

// ---------------------------------------------------------------------------
// Full-journal vs snapshot+tail watermark parity
// ---------------------------------------------------------------------------

#[test]
fn watermark_full_journal_vs_snapshot_tail_parity() {
    let run = RunId::new(1);
    let all_events = vec![
        accepted_event(run, 0),
        started_event(run, 1, 0),
        succeeded_event(run, 2, 0, 0),
        started_event(run, 3, 1),
        succeeded_event(run, 4, 1, 1),
    ];

    // Full journal recovery summary
    let full_summary = summarize_recovery_events(&all_events)
        .expect("full summarize should succeed")
        .summary();

    // Snapshot+tail recovery (snapshot at seq 2)
    let snapshot = make_snapshot(run, 2);
    let tail = vec![
        started_event(run, 3, 1),
        succeeded_event(run, 4, 1, 1),
    ];

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
        .expect("snapshot+tail should succeed");

    let tail_summary = summarize_recovery_events(&replayed)
        .expect("tail summarize should succeed")
        .summary();

    // Watermark parity: the tail recovery should have the same last_seq
    assert_eq!(
        full_summary.last_seq, tail_summary.last_seq,
        "full journal and snapshot+tail must agree on last_seq"
    );
}

// ---------------------------------------------------------------------------
// EventSeq::MAX overflow sentinel rejection
// ---------------------------------------------------------------------------

#[test]
fn watermark_max_seq_rejected_in_validation() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 0),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::MAX,
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];

    let result = summarize_recovery_events(&events);
    assert!(
        result.is_err(),
        "EventSeq::MAX must be rejected during recovery"
    );
}

#[test]
fn watermark_max_seq_as_only_event_rejected() {
    let run = RunId::new(1);
    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: EventSeq::MAX,
        workflow: sample_digest(1),
    }];

    let result = summarize_recovery_events(&events);
    assert!(result.is_err(), "EventSeq::MAX as sole event must be rejected");
}

// ---------------------------------------------------------------------------
// Journal-backed recovery watermark tests
// ---------------------------------------------------------------------------

#[test]
fn watermark_journal_recovery_first_and_last_seq() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let run = RunId::new(42);

    journal
        .append_journaled(&accepted_event(run, 0))
        .expect("append accepted");
    journal
        .append_journaled(&started_event(run, 1, 0))
        .expect("append started");
    journal
        .append_journaled(&succeeded_event(run, 2, 0, 0))
        .expect("append succeeded");
    journal
        .append_journaled(&finished_event(run, 3))
        .expect("append finished");

    let summary = recover_runtime_summary(&journal, run)
        .expect("summary recovers")
        .summary();

    assert_eq!(summary.first_seq, EventSeq::new(0));
    assert_eq!(summary.last_seq, EventSeq::new(3));
    assert!(summary.first_seq <= summary.last_seq);
}

#[test]
fn watermark_journal_recovery_rejects_max_seq() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let run = RunId::new(42);

    journal
        .append_journaled(&accepted_event(run, 0))
        .expect("append accepted");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::MAX,
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("append max seq");

    let result = recover_runtime_summary(&journal, run);
    assert!(
        result.is_err(),
        "journal recovery must reject EventSeq::MAX"
    );
}

// ---------------------------------------------------------------------------
// Frame seed recovery watermark tests
// ---------------------------------------------------------------------------

#[test]
fn watermark_frame_seed_first_and_last_seq() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 10),
        started_event(run, 11, 0),
        succeeded_event(run, 12, 0, 0),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events)
        .expect("frame seed recovery should succeed");

    assert_eq!(seed.summary.first_seq, EventSeq::new(10));
    assert_eq!(seed.summary.last_seq, EventSeq::new(12));
}

#[test]
fn watermark_frame_seed_rejects_max_seq() {
    let run = RunId::new(1);
    let events = vec![
        accepted_event(run, 0),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::MAX,
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];

    let result = recover_runtime_frame_seed_from_events(&events);
    assert!(
        result.is_err(),
        "frame seed recovery must reject EventSeq::MAX"
    );
}
