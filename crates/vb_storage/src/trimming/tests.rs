#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::*;
use crate::{
    EventSeq, FjallJournal, JournalError, JournalEvent, RunHeaderRecord, RunSnapshot,
    constants::DIGEST_BYTES,
};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn make_run_finished(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(0),
        attempt: 1,
    }
}

fn write_header(journal: &FjallJournal, run: RunId, digest: WorkflowDigest) {
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 0,
    };
    journal
        .put_run_header(&header)
        .expect("header write should succeed");
}

fn write_header_with_workflow(
    journal: &FjallJournal,
    run: RunId,
    workflow_id: WorkflowId,
    digest: WorkflowDigest,
    accepted_at_ms: u64,
) {
    let header = RunHeaderRecord {
        run,
        workflow_id,
        compiled_digest: digest,
        status: 0,
        accepted_at_ms,
    };
    journal
        .put_run_header(&header)
        .expect("header write should succeed");
}

// =========================================================================
// vb-uu31g / SC-005 — batch trim path skips redundant per-run retention check
// =========================================================================

/// Helper: build a fully-terminal run with 3 events and a snapshot at seq 1.
fn write_terminal_run_with_snapshot(
    journal: &FjallJournal,
    run: RunId,
    workflow_id: WorkflowId,
    digest: WorkflowDigest,
    accepted_at_ms: u64,
) {
    let events: Vec<JournalEvent> = (0..3u64)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else if i == 2 {
                make_run_finished(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("terminal run batch should succeed");
    write_header_with_workflow(journal, run, workflow_id, digest, accepted_at_ms);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");
}

/// Helper: build a non-terminal run with 3 events and a snapshot at seq 1.
fn write_non_terminal_run_with_snapshot(
    journal: &FjallJournal,
    run: RunId,
    workflow_id: WorkflowId,
    digest: WorkflowDigest,
    accepted_at_ms: u64,
) {
    let events: Vec<JournalEvent> = (0..3u64)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("non-terminal run batch should succeed");
    write_header_with_workflow(journal, run, workflow_id, digest, accepted_at_ms);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");
}

include!("tests/basic_trim.rs");
include!("tests/snapshot_and_retained.rs");
include!("tests/batch_and_fail_closed.rs");
include!("tests/retention_and_replay.rs");
include!("tests/diagnostics.rs");
