// Manual QA smoke test for vb-5h50 trimming
// Run with: cargo test -p vb_storage --test manual_qa_smoke -- --nocapture

use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};
use vb_storage::constants::DIGEST_BYTES;
use vb_storage::{EventSeq, JournalEvent, RunHeaderRecord, RunSnapshot};
use vb_storage::{FjallJournal, TrimError, TrimPolicy, TrimStatus};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("temp_journal: tempdir creation must succeed");
    let journal = FjallJournal::open(temp.path(), None)
        .expect("temp_journal: journal open must succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

fn make_step_started(run: RunId, seq: u64, step: u16, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt,
    }
}

fn make_run_finished(run: RunId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(0),
        attempt,
    }
}

#[test]
fn smoke_happy_path_trim() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1);
    let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..6)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1, i as u16)
            }
        })
        .collect();
    journal.append_strict_batch(&events)
        .expect("smoke_happy_path_trim: batch append must succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot)
        .expect("smoke_happy_path_trim: put snapshot must succeed");

    let result = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("smoke_happy_path_trim: trim must succeed in happy path");
    println!("Happy path trim result: {:?}", result);
    assert_eq!(result.status, TrimStatus::Trimmed);
    assert_eq!(result.deleted_count, 3);

    let remaining = journal.events_for_run(run)
        .expect("smoke_happy_path_trim: events_for_run must succeed");
    println!("Remaining events after trim: {}", remaining.len());
    // Snapshot at seq 3 covers events 0..3; events 4,5 remain for replay
    assert_eq!(remaining.len(), 2);
}

#[test]
fn smoke_retention_policy_blocks() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let workflow_id = WorkflowId::new(1);
    let digest = WorkflowDigest::from_bytes([0xCD; DIGEST_BYTES]);

    let events = vec![
        make_event(run, 0),
        make_step_started(run, 1, 0, 1),
        make_run_finished(run, 2, 1),
    ];
    journal.append_strict_batch(&events)
        .expect("smoke_retention_policy_blocks: batch append must succeed");

    let header = RunHeaderRecord {
        run,
        workflow_id,
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 1000,
    };
    journal.put_run_header(&header)
        .expect("smoke_retention_policy_blocks: put_run_header must succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot)
        .expect("smoke_retention_policy_blocks: put_snapshot must succeed");

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 5,
    };
    let result = journal.trim_events_for_run(run, policy);
    println!("Retention block result: {:?}", result);
    assert!(matches!(
        result,
        Err(TrimError::RetentionPolicyBlocks { .. })
    ));
}

#[test]
fn smoke_no_snapshot_fails_closed() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);

    let events = vec![make_event(run, 0), make_step_started(run, 1, 0, 1)];
    journal.append_strict_batch(&events)
        .expect("smoke_no_snapshot_fails_closed: batch append must succeed");

    let result = journal.trim_events_for_run(run, TrimPolicy::default());
    println!("No snapshot result: {:?}", result);
    assert!(matches!(result, Err(TrimError::NoDurableSnapshot { .. })));
}

#[test]
fn smoke_idempotency() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);
    let digest = WorkflowDigest::from_bytes([0xEF; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..4)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1, i as u16)
            }
        })
        .collect();
    journal.append_strict_batch(&events)
        .expect("smoke_idempotency: batch append must succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot)
        .expect("smoke_idempotency: put_snapshot must succeed");

    let r1 = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("smoke_idempotency: first trim must succeed");
    println!("First trim: {:?}", r1);
    assert_eq!(r1.status, TrimStatus::Trimmed);

    let r2 = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("smoke_idempotency: second trim must succeed");
    println!("Second trim: {:?}", r2);
    assert_eq!(r2.status, TrimStatus::NoOp);
    assert_eq!(r2.deleted_count, 0);
}
