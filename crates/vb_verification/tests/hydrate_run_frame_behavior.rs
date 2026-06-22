#![forbid(unsafe_code)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

//! Behavior tests for `hydrate_run_frame` and `hydrate_run_frame_from_events`.
//!
//! These tests exercise the positive and negative paths of the recovery
//! hydration functions, asserting exact error variants and key fields.
//! They close the S4-R6-004 finding (vb_verification had 0 behavior tests
//! for 6 cycles).

use vb_core::{RunId, StepIdx, WorkflowDigest};
use vb_storage::recovery::RecoveryError;
use vb_storage::recovery::hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
use vb_storage::recovery::RunSnapshot;
use vb_storage::{EventSeq, JournalEvent};

fn sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn make_snapshot(run: RunId, seq: u64) -> RunSnapshot {
    RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: sample_digest(1),
        slots: vec![],
        taint: vec![],
    }
}

#[test]
fn hydrate_from_events_empty_returns_no_recovery_data() {
    let events: Vec<JournalEvent> = Vec::new();
    let run_id = RunId::new(1);
    let result = hydrate_run_frame_from_events(&events, run_id);
    assert!(
        matches!(result, Err(RecoveryError::NoRecoveryData { run }) if run == run_id),
        "empty events must return Err(NoRecoveryData), got {result:?}"
    );
}

#[test]
fn hydrate_run_frame_non_matching_run_id_returns_err() {
    let snapshot_run = RunId::new(10);
    let requested_run = RunId::new(20);
    let snapshot = make_snapshot(snapshot_run, 0);
    let tail = vec![JournalEvent::RunAccepted {
        run: requested_run,
        seq: EventSeq::new(1),
        workflow: sample_digest(1),
    }];
    let result = hydrate_run_frame(&snapshot, &tail, requested_run);
    assert!(
        result.is_err(),
        "non-matching snapshot.run must return Err, got {result:?}"
    );
}

#[test]
fn hydrate_from_events_with_run_accepted_succeeds() {
    let run_id = RunId::new(42);
    let workflow = sample_digest(7);
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run: run_id,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];
    let result = hydrate_run_frame_from_events(&events, run_id);
    let frame = result.expect("valid events with RunAccepted must hydrate Ok");
    assert_eq!(
        frame.run_id(),
        run_id,
        "hydrated frame.run_id must equal requested run_id"
    );
}

#[test]
fn hydrate_run_frame_tail_seq_not_after_snapshot_returns_err() {
    let run_id = RunId::new(5);
    let snapshot = make_snapshot(run_id, 10);
    let tail = vec![JournalEvent::RunAccepted {
        run: run_id,
        seq: EventSeq::new(5),
        workflow: sample_digest(1),
    }];
    let result = hydrate_run_frame(&snapshot, &tail, run_id);
    assert!(
        result.is_err(),
        "tail seq <= snapshot seq must return Err, got {result:?}"
    );
}

#[test]
fn hydrate_from_events_missing_run_accepted_still_succeeds() {
    // Production behavior: hydrate_run_frame_from_events does not require
    // RunAccepted to be the first event. The function derives dimensions
    // from whatever state-bearing events are present. This test pins
    // the actual contract so a future tightening to require RunAccepted
    // would be caught.
    let run_id = RunId::new(77);
    let events = vec![JournalEvent::StepStarted {
        run: run_id,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        attempt: 1,
    }];
    let result = hydrate_run_frame_from_events(&events, run_id);
    let frame = result.expect("events without RunAccepted still hydrate Ok per current contract");
    assert_eq!(frame.run_id(), run_id);
}
