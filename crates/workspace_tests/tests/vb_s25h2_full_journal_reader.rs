#![forbid(unsafe_code)]
//! Bead `vb-s25h2` — full-journal reader + recovery equivalence.
//!
//! Master sections: §18 (Fjall Persistence Behavior, persistence invariant 3
//! "Recovery replays snapshots plus tail journal or full journal
//! deterministically"), §29 (Mandatory Function Surface: `vb_storage`,
//! read-path contract for `events_for_run`).
//!
//! Proves `events_for_run_full` returns ALL events from seq 0 even when a
//! durable snapshot exists, that `events_for_run` continues to skip
//! pre-snapshot events by design, that the union of pre-snapshot events and
//! tail events equals the full journal (snapshot+tail equivalence), and that
//! `load_snapshot` distinguishes `RecoveryError::MissingSnapshot` (no record
//! in keyspace) from `RecoveryError::CorruptSnapshot` (present but
//! undecodable).

use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::replay::core::{
    load_snapshot, recover_full_journal, recover_snapshot_plus_tail,
};
use vb_storage::recovery::types::{ActionReplayTracker, RecoveryError, RunSnapshot};
use vb_storage::{
    EventSeq, FjallJournal, JournalEvent, codec::encode_record,
    constants::{DIGEST_BYTES, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES},
    records::RecordKind,
};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; DIGEST_BYTES])
}

fn run_accepted(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: digest(0x42),
    }
}

fn step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn step_succeeded(run: RunId, seq: u64, step: u16, output: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        output: SlotIdx::new(output),
    }
}

fn make_snapshot(run: RunId, seq: u64) -> RunSnapshot {
    RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: digest(0x42),
        slots: vec![0, 1, 2],
        taint: vec![0, 0, 0],
    }
}

/// Writes a contiguous event sequence for a run, starting at seq 0, by
/// appending `RunAccepted` at seq 0, `StepStarted` at seq 1..N, then
/// `StepSucceeded` at seq N..=2N-1. Returns the number of events appended.
fn write_full_run(
    journal: &FjallJournal,
    run: RunId,
    step_count: u16,
) -> Result<usize, vb_storage::JournalError> {
    journal.append_journaled(&run_accepted(run, 0))?;
    for step in 0..step_count {
        journal.append_journaled(&step_started(run, 1 + u64::from(step), step))?;
    }
    for step in 0..step_count {
        let seq = 1 + u64::from(step_count) + u64::from(step);
        journal.append_journaled(&step_succeeded(run, seq, step, step))?;
    }
    Ok(usize::from(step_count) * 2 + 1)
}

#[test]
fn events_for_run_full_returns_pre_snapshot_events() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_001);
    let total = write_full_run(&journal, run, 5)?;
    assert_eq!(total, 11);
    journal.put_snapshot(&make_snapshot(run, 5))?;

    let tail = journal.events_for_run(run)?;
    assert_eq!(tail.len(), 5, "snapshot+tail must skip pre-snapshot events");
    assert_eq!(tail.first().map(|e| e.seq().get()), Some(6));
    assert_eq!(tail.last().map(|e| e.seq().get()), Some(10));

    let full = journal.events_for_run_full(run)?;
    assert_eq!(full.len(), total, "full-journal reader returns every event from seq 0");
    assert_eq!(full.first().map(|e| e.seq().get()), Some(0));
    assert_eq!(full.last().map(|e| e.seq().get()), Some(10));
    Ok(())
}

#[test]
fn snapshot_plus_tail_equals_full_journal_event_set() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_002);
    let total = write_full_run(&journal, run, 4)?;
    assert_eq!(total, 9);
    let snapshot_seq = 4u64;
    journal.put_snapshot(&make_snapshot(run, snapshot_seq))?;

    let full = journal.events_for_run_full(run)?;
    let tail = journal.events_for_run(run)?;
    let pre_snapshot_len = (snapshot_seq as usize) + 1;
    assert_eq!(
        tail.len(),
        total - pre_snapshot_len,
        "tail length must equal total minus pre-snapshot events"
    );

    let mut expected_pre = Vec::with_capacity(pre_snapshot_len);
    expected_pre.push(run_accepted(run, 0));
    for step in 0..snapshot_seq {
        expected_pre.push(step_started(run, 1 + step, step as u16));
    }
    assert_eq!(&full[..pre_snapshot_len], &expected_pre[..]);
    assert_eq!(&full[pre_snapshot_len..], &tail[..]);
    Ok(())
}

#[test]
fn recover_full_journal_includes_pre_snapshot_events() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_003);
    let total = write_full_run(&journal, run, 3)?;
    assert_eq!(total, 7);
    journal.put_snapshot(&make_snapshot(run, 2))?;

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])?;
    assert_eq!(replayed.len(), total);
    match replayed.first() {
        Some(JournalEvent::RunAccepted { run: r, seq, workflow }) => {
            assert_eq!(*r, run);
            assert_eq!(seq.get(), 0);
            assert_eq!(*workflow, digest(0x42));
        }
        other => panic!("first replayed event must be RunAccepted, got {other:?}"),
    }
    Ok(())
}

#[test]
fn recover_snapshot_plus_tail_returns_only_post_snapshot_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_004);
    let total = write_full_run(&journal, run, 4)?;
    assert_eq!(total, 9);
    let snapshot_seq = 5u64;
    let snapshot = make_snapshot(run, snapshot_seq);
    journal.put_snapshot(&snapshot)?;

    let tail = journal.events_for_run(run)?;
    let mut tracker = ActionReplayTracker::new();
    let replayed_tail = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)?;

    let full = journal.events_for_run_full(run)?;
    let pre_snapshot_len = (snapshot_seq as usize) + 1;
    assert_eq!(replayed_tail.len(), total - pre_snapshot_len);
    assert_eq!(replayed_tail.len(), tail.len());
    assert_eq!(&full[pre_snapshot_len..], &replayed_tail[..]);
    Ok(())
}

#[test]
fn load_snapshot_returns_missing_snapshot_for_unknown_run_seq(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_005);
    let result = load_snapshot(&journal, run, EventSeq::new(0));
    match result {
        Err(RecoveryError::MissingSnapshot { run: r, seq }) => {
            assert_eq!(r, run);
            assert_eq!(seq.get(), 0);
        }
        Err(other) => panic!("expected MissingSnapshot, got {other:?}"),
        Ok(snapshot) => panic!("expected MissingSnapshot, got snapshot {snapshot:?}"),
    }
    Ok(())
}

#[test]
fn load_snapshot_returns_corrupt_snapshot_for_unreadable_payload(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_006);
    let snapshot_seq = EventSeq::new(7);
    let invalid_payload = vec![0xFF_u8; 4];
    let value = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot_seq.get(),
        &invalid_payload,
        MAX_SNAPSHOT_BYTES,
    )?;
    let key = vb_storage::keys::run_snapshot_key(run, snapshot_seq)?;
    journal.run_snapshot.insert(key.to_vec(), value)?;

    let result = load_snapshot(&journal, run, snapshot_seq);
    match result {
        Err(RecoveryError::CorruptSnapshot { run: r, seq }) => {
            assert_eq!(r, run);
            assert_eq!(seq.get(), 7);
        }
        Err(other) => panic!("expected CorruptSnapshot, got {other:?}"),
        Ok(snapshot) => panic!("expected CorruptSnapshot, got snapshot {snapshot:?}"),
    }
    Ok(())
}

#[test]
fn missing_snapshot_and_corrupt_snapshot_are_distinct_variants() {
    let run = RunId::new(1);
    let seq = EventSeq::new(0);
    let missing = RecoveryError::MissingSnapshot { run, seq };
    let corrupt = RecoveryError::CorruptSnapshot { run, seq };
    assert_ne!(missing, corrupt);
    assert_eq!(
        RecoveryError::MissingSnapshot { run, seq },
        RecoveryError::MissingSnapshot { run, seq }
    );
    assert_eq!(
        RecoveryError::CorruptSnapshot { run, seq },
        RecoveryError::CorruptSnapshot { run, seq }
    );
}

#[test]
fn load_snapshot_returns_snapshot_when_record_is_present(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_007);
    let snapshot = make_snapshot(run, 3);
    journal.put_snapshot(&snapshot)?;
    let loaded = load_snapshot(&journal, run, EventSeq::new(3))?;
    assert_eq!(loaded.seq, snapshot.seq);
    assert_eq!(loaded.run, snapshot.run);
    assert_eq!(loaded.workflow, snapshot.workflow);
    Ok(())
}

#[test]
fn recover_full_journal_rejects_empty_journal() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20_008);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    match result {
        Err(RecoveryError::NoRecoveryData { run: r }) => assert_eq!(r, run),
        Err(other) => panic!("expected NoRecoveryData, got {other:?}"),
        Ok(events) => panic!("expected NoRecoveryData, got {} events", events.len()),
    }
    Ok(())
}
