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

fn insert_snapshot_payload_under_key(
    journal: &FjallJournal,
    key_run: RunId,
    key_seq: EventSeq,
    payload: &RunSnapshot,
) {
    let key = crate::keys::run_snapshot_key(key_run, key_seq).expect("snapshot key");
    let value = crate::codec::encode_record(
        crate::constants::MAGIC_SNAPSHOT,
        crate::records::RecordKind::Snapshot,
        payload.seq.get(),
        payload,
        crate::constants::MAX_SNAPSHOT_BYTES,
    )
    .expect("snapshot payload encode");
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .expect("snapshot payload insert");
}

#[test]
fn trim_given_run_with_events_seq_0_to_9_and_snapshot_at_seq_5_trims_0_to_4() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..10u64)
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
        .expect("batch should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    let result = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("trim should succeed");

    assert_eq!(result.run, run);
    assert_eq!(result.deleted_count, 5, "should delete events 0-4");
    assert_eq!(result.cutoff_seq, EventSeq::new(5));
    assert_eq!(result.status, TrimStatus::Trimmed);

    // Snapshot at seq 5 covers events 0..5; replay starts at 6
    let remaining = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        remaining,
        events[6..10].to_vec(),
        "should preserve exact event records 6-9"
    );
}

#[test]
fn trim_given_run_already_trimmed_is_noop() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let digest = WorkflowDigest::from_bytes([0xCD; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..6u64)
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
        .expect("batch should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    let result1 = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("first trim should succeed");
    assert_eq!(result1.deleted_count, 5);
    assert_eq!(result1.status, TrimStatus::Trimmed);

    let result2 = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("second trim should succeed");
    assert_eq!(result2.deleted_count, 0);
    assert_eq!(result2.status, TrimStatus::NoOp);
}

#[test]
fn trim_given_run_with_no_snapshot_returns_error() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);

    let events = [make_event(run, 0), make_step_started(run, 1, 0)];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let err = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect_err("missing durable snapshot must fail with exact run context");
    match err {
        TrimError::NoDurableSnapshot { run: actual } => {
            assert_eq!(actual, run);
            assert_eq!(
                TrimError::NoDurableSnapshot { run: actual }.diagnostic_code(),
                TrimError::NO_DURABLE_SNAPSHOT_CODE
            );
        }
        other => panic!("expected NoDurableSnapshot for run {run:?}, got {other:?}"),
    }
}

#[test]
fn trim_preserves_run_header_and_snapshot() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);
    let digest = WorkflowDigest::from_bytes([0xEF; DIGEST_BYTES]);

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
        .expect("batch should succeed");
    let expected_header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 0,
    };
    journal
        .put_run_header(&expected_header)
        .expect("header write should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("trim should succeed");

    let header = journal
        .run_header(run)
        .expect("header lookup should succeed");
    assert_eq!(
        header,
        Some(expected_header),
        "run header bytes must be preserved exactly"
    );

    let snap = journal
        .snapshot(run, EventSeq::new(2))
        .expect("snapshot lookup should succeed");
    assert_eq!(
        snap,
        Some(snapshot),
        "snapshot payload must be preserved exactly"
    );
}

#[test]
fn trim_all_eligible_runs_skips_runs_without_snapshots() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(500);
    let run_b = RunId::new(600);
    let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

    let events_a = [make_event(run_a, 0), make_step_started(run_a, 1, 0)];
    journal
        .append_strict_batch(&events_a)
        .expect("batch A should succeed");
    write_header(&journal, run_a, digest);

    let events_b = [make_event(run_b, 0), make_step_started(run_b, 1, 0)];
    journal
        .append_strict_batch(&events_b)
        .expect("batch B should succeed");
    write_header(&journal, run_b, digest);

    let snapshot_a = RunSnapshot {
        run: run_a,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot_a)
        .expect("snapshot A should succeed");

    let results = journal
        .trim_all_eligible_runs(TrimPolicy::default())
        .expect("trim_all should succeed");

    assert_eq!(results.len(), 1, "only run A should be trimmed");
    assert_eq!(results[0].run, run_a);
    assert_eq!(results[0].deleted_count, 1);

    let remaining_a = journal
        .events_for_run(run_a)
        .expect("replay A should succeed");
    // Snapshot at seq 1 covers events 0..1; no events remain for replay
    assert_eq!(remaining_a.len(), 0);

    let remaining_b = journal
        .events_for_run(run_b)
        .expect("replay B should succeed");
    assert_eq!(remaining_b.len(), 2, "run B should be untouched");
}

#[test]
fn latest_durable_snapshot_seq_returns_highest_seq() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);

    let snapshots = [
        RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        },
        RunSnapshot {
            run,
            seq: EventSeq::new(1),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        },
        RunSnapshot {
            run,
            seq: EventSeq::new(5),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        },
    ];

    for snap in &snapshots {
        journal.put_snapshot(snap).expect("snapshot should succeed");
    }

    let latest = journal
        .latest_durable_snapshot_seq(run)
        .expect("should succeed");
    assert_eq!(latest, Some(EventSeq::new(5)), "latest should be seq 5");
}

#[test]
fn latest_durable_snapshot_seq_returns_none_for_no_snapshots() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(800);

    let latest = journal
        .latest_durable_snapshot_seq(run)
        .expect("should succeed");
    assert_eq!(latest, None);
}

#[test]
fn latest_durable_snapshot_seq_rejects_payload_run_mismatch() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(801);
    let actual = RunId::new(802);
    let key_seq = EventSeq::new(7);
    let payload = RunSnapshot {
        run: actual,
        seq: key_seq,
        workflow: WorkflowDigest::from_bytes([0x51; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    insert_snapshot_payload_under_key(&journal, run, key_seq, &payload);

    let err = journal
        .latest_durable_snapshot_seq(run)
        .expect_err("payload run mismatch must fail closed");

    match err {
        TrimError::Journal(JournalError::WrongRun { expected, actual }) => {
            assert_eq!(expected, run);
            assert_eq!(actual, RunId::new(802));
            assert_eq!(
                JournalError::WrongRun { expected, actual }.diagnostic_code(),
                JournalError::WRONG_RUN_CODE
            );
        }
        other => panic!("expected WrongRun, got {other:?}"),
    }
}

#[test]
fn latest_durable_snapshot_seq_rejects_payload_seq_mismatch() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(803);
    let key_seq = EventSeq::new(7);
    let payload_seq = EventSeq::new(8);
    let payload = RunSnapshot {
        run,
        seq: payload_seq,
        workflow: WorkflowDigest::from_bytes([0x52; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    insert_snapshot_payload_under_key(&journal, run, key_seq, &payload);

    let err = journal
        .latest_durable_snapshot_seq(run)
        .expect_err("payload seq mismatch must fail closed");

    match err {
        TrimError::Journal(JournalError::SequenceGap { expected, actual }) => {
            assert_eq!(expected, key_seq);
            assert_eq!(actual, payload_seq);
            assert_eq!(
                JournalError::SequenceGap { expected, actual }.diagnostic_code(),
                JournalError::SEQUENCE_GAP_CODE
            );
        }
        other => panic!("expected SequenceGap, got {other:?}"),
    }
}

#[test]
fn trim_preserves_events_at_or_after_snapshot() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(900);
    let digest = WorkflowDigest::from_bytes([0xEF; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..4u64)
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
        .expect("batch should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    let result = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("trim should succeed");
    assert_eq!(result.deleted_count, 2, "should delete events 0-1");
    assert_eq!(result.status, TrimStatus::Trimmed);

    // Snapshot at seq 2 covers events 0..2; only event 3 remains for replay
    let remaining = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        remaining,
        vec![events[3].clone()],
        "should preserve exact event 3 only"
    );
}

#[test]
fn terminal_retention_blocks_recent_terminal_runs() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1000);
    let workflow_id = WorkflowId::new(1);
    let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

    // Create a terminal run: events 0..=3 with RunFinished at seq 3
    let events: Vec<JournalEvent> = (0..4u64)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else if i == 3 {
                make_run_finished(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    write_header_with_workflow(&journal, run, workflow_id, digest, 1000);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    // With retention policy of 5, this run should be blocked because
    // it is the only terminal run for this workflow (position 0 < 5)
    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 5,
    };
    let err = journal
        .trim_events_for_run(run, policy)
        .expect_err("recent terminal run must be blocked by retention");
    match err {
        TrimError::RetentionPolicyBlocks { run: actual } => {
            assert_eq!(actual, run);
            assert_eq!(
                TrimError::RetentionPolicyBlocks { run: actual }.diagnostic_code(),
                TrimError::RETENTION_POLICY_BLOCKS_CODE
            );
        }
        other => panic!("expected RetentionPolicyBlocks for run {run:?}, got {other:?}"),
    }
}

#[test]
fn terminal_retention_allows_older_terminal_runs() {
    let (_temp, journal) = temp_journal();
    let workflow_id = WorkflowId::new(1);
    let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);

    // Create 5 terminal runs for the same workflow, all with snapshots
    for run_id in 1u64..=5 {
        let run = RunId::new(run_id);
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
            .expect("batch should succeed");
        write_header_with_workflow(&journal, run, workflow_id, digest, run_id * 100);
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

    // With retention policy of 3, runs 1 and 2 (oldest) should be trimmable
    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 3,
    };

    // Run 1 (accepted_at_ms=100) is the oldest, should be allowed
    let result = journal
        .trim_events_for_run(RunId::new(1), policy)
        .expect("oldest terminal run should be trimmable");
    assert_eq!(result.status, TrimStatus::Trimmed);
    assert_eq!(result.deleted_count, 1);
}

#[test]
fn non_terminal_runs_ignore_retention_policy() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(2000);
    let workflow_id = WorkflowId::new(2);
    let digest = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);

    // Non-terminal run: events 0..3, no terminal event
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
        .expect("batch should succeed");
    write_header_with_workflow(&journal, run, workflow_id, digest, 2000);

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

    // Even with retention policy, non-terminal runs should trim normally
    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 0,
    };
    let result = journal
        .trim_events_for_run(run, policy)
        .expect("non-terminal run should trim");
    assert_eq!(result.status, TrimStatus::Trimmed);
    assert_eq!(result.deleted_count, 1);
}

#[test]
fn replay_equivalence_after_trim() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(3000);
    let digest = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..6u64)
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
        .expect("batch should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    // Trim removes events 0..2
    let trim_result = journal
        .trim_events_for_run(run, TrimPolicy::default())
        .expect("trim should succeed");
    assert_eq!(trim_result.deleted_count, 3, "should delete events 0-2");

    // After trim, snapshot at seq 3 covers events 0..3;
    // replay yields tail events 4..5
    let after_trim = journal
        .events_for_run(run)
        .expect("replay after trim should succeed");
    assert_eq!(
        after_trim,
        events[4..6].to_vec(),
        "should preserve exact events 4-5"
    );

    // Verify trimmed events are actually gone by trying to read them directly
    for seq in 0..3u64 {
        let key = crate::keys::run_event_key(run, EventSeq::new(seq)).expect("key ok");
        assert!(
            journal.events.get(key).expect("get ok").is_none(),
            "event seq {} should be deleted",
            seq
        );
    }
}

#[test]
fn trim_policy_default_includes_retention() {
    let policy = TrimPolicy::default();
    assert!(policy.skip_noop_runs);
    assert_eq!(policy.retain_last_n_terminal, 10);
}

#[test]
fn no_durable_snapshot_error_has_correct_diagnostic_code() {
    let err = TrimError::NoDurableSnapshot { run: RunId::new(1) };
    assert_eq!(err.diagnostic_code(), TrimError::NO_DURABLE_SNAPSHOT_CODE);
}

#[test]
fn retention_policy_blocks_error_has_correct_diagnostic_code() {
    let err = TrimError::RetentionPolicyBlocks { run: RunId::new(1) };
    assert_eq!(
        err.diagnostic_code(),
        TrimError::RETENTION_POLICY_BLOCKS_CODE
    );
}

#[test]
fn diagnostic_returns_eligible_and_blocked_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(10_000);
    let run_b = RunId::new(10_001);
    let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);

    // Run A: has events and snapshot
    let events_a: Vec<JournalEvent> = (0..6u64)
        .map(|i| {
            if i == 0 {
                make_event(run_a, i)
            } else {
                make_step_started(run_a, i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events_a)
        .expect("batch A should succeed");
    write_header(&journal, run_a, digest);
    let snapshot_a = RunSnapshot {
        run: run_a,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot_a).expect("snapshot A ok");

    // Run B: has events but NO snapshot
    let events_b = [make_event(run_b, 0), make_step_started(run_b, 1, 0)];
    journal
        .append_strict_batch(&events_b)
        .expect("batch B should succeed");
    write_header(&journal, run_b, digest);

    let diag = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("diagnostic should succeed");

    assert_eq!(diag.total_runs, 2, "should report 2 total runs");
    assert_eq!(diag.eligible_runs, 1, "run A should be eligible");
    assert_eq!(diag.blocked_runs, 1, "run B should be blocked");

    let eligible = diag
        .runs
        .iter()
        .find(|r| matches!(r, TrimEligibility::Eligible { run, .. } if run == &run_a));
    assert!(
        eligible.is_some(),
        "run A should be Eligible, got {:?}",
        diag.runs
    );

    let blocked = diag
        .runs
        .iter()
        .find(|r| matches!(r, TrimEligibility::Blocked { run, .. } if run == &run_b));
    assert!(
        blocked.is_some(),
        "run B should be Blocked, got {:?}",
        diag.runs
    );
}

#[test]
fn diagnostic_reports_correct_safe_point_and_trimmable_count() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(11_000);
    let digest = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..10u64)
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
        .expect("batch should succeed");
    write_header(&journal, run, digest);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot ok");

    let diag = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("diagnostic should succeed");

    let eligible = diag.runs.iter().find_map(|r| match r {
        TrimEligibility::Eligible {
            run: r,
            safe_point,
            events_trimmable,
        } if r == &run => Some((*safe_point, *events_trimmable)),
        _ => None,
    });

    assert!(
        eligible.is_some(),
        "run should be eligible, got {:?}",
        diag.runs
    );
    let (safe_point, trimmable) = eligible.unwrap();
    assert_eq!(safe_point, EventSeq::new(5), "safe point should be seq 5");
    assert_eq!(trimmable, 5, "should report 5 trimmable events (0-4)");
    assert_eq!(
        diag.total_events_trimmable, 5,
        "aggregate trimmable should be 5"
    );
}

#[test]
fn diagnostic_blocks_run_without_durable_snapshot() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(12_000);
    let digest = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);

    let events = [make_event(run, 0), make_step_started(run, 1, 0)];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    write_header(&journal, run, digest);
    // No snapshot written

    let diag = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("diagnostic should succeed");

    assert_eq!(diag.total_runs, 1);
    assert_eq!(diag.eligible_runs, 0);
    assert_eq!(diag.blocked_runs, 1);

    let blocked = diag.runs.first().expect("should have one run result");
    assert!(
        matches!(
            blocked,
                TrimEligibility::Blocked {
                    run: r,
                    blocker: TrimBlocker::NoDurableSnapshot,
                } if r == &run

        ),
        "run should be blocked by NoDurableSnapshot, got {:?}",
        blocked
    );
}

#[test]
fn diagnostic_blocks_recent_terminal_run_under_retention() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(13_000);
    let workflow_id = WorkflowId::new(3);
    let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

    // Terminal run: events 0..=3 with RunFinished at seq 3
    let events: Vec<JournalEvent> = (0..4u64)
        .map(|i| {
            if i == 0 {
                make_event(run, i)
            } else if i == 3 {
                make_run_finished(run, i)
            } else {
                make_step_started(run, i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    write_header_with_workflow(&journal, run, workflow_id, digest, 13_000);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot ok");

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 5,
    };
    let diag = journal
        .trim_eligibility_diagnostic(policy)
        .expect("diagnostic should succeed");

    assert_eq!(diag.total_runs, 1);
    assert_eq!(diag.eligible_runs, 0);
    assert_eq!(diag.blocked_runs, 1);

    let blocked = diag.runs.first().expect("should have one run result");
    assert!(
        matches!(
            blocked,
                TrimEligibility::Blocked {
                    run: r,
                    blocker: TrimBlocker::RetentionPolicy {
                        retain_last_n_terminal: 5,
                    },
                } if r == &run

        ),
        "run should be blocked by RetentionPolicy, got {:?}",
        blocked
    );
}

#[test]
fn diagnostic_allows_non_terminal_run_despite_retention() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(14_000);
    let workflow_id = WorkflowId::new(4);
    let digest = WorkflowDigest::from_bytes([0x99; DIGEST_BYTES]);

    // Non-terminal run: no RunFinished event
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
        .expect("batch should succeed");
    write_header_with_workflow(&journal, run, workflow_id, digest, 14_000);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot ok");

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 10,
    };
    let diag = journal
        .trim_eligibility_diagnostic(policy)
        .expect("diagnostic should succeed");

    assert_eq!(diag.total_runs, 1);
    assert_eq!(diag.eligible_runs, 1);
    assert_eq!(diag.blocked_runs, 0);

    let eligible = diag.runs.first().expect("should have one run result");
    match eligible {
        TrimEligibility::Eligible {
            run: r,
            safe_point,
            events_trimmable,
        } => {
            assert_eq!(r, &run);
            assert_eq!(safe_point, &EventSeq::new(1));
            assert_eq!(events_trimmable, &1);
        }

        _ => panic!("non-terminal run should be eligible, got {:?}", eligible),
    }
}

#[test]
fn diagnostic_does_not_delete_events() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(15_000);
    let digest = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..5u64)
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
        .expect("batch should succeed");
    write_header(&journal, run, digest);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot ok");

    let before = journal
        .events_for_run(run)
        .expect("events before diagnostic");
    // Snapshot at seq 3 covers events 0..3; only seq 4 remains for replay
    assert_eq!(before.len(), 1, "events after snapshot should be 1 (seq 4)");

    let _diag = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("diagnostic should succeed");

    let after = journal
        .events_for_run(run)
        .expect("events after diagnostic");
    assert_eq!(
        after.len(),
        before.len(),
        "diagnostic must not delete events, before={} after={}",
        before.len(),
        after.len()
    );
}

#[test]
fn diagnostic_is_idempotent() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(16_000);
    let digest = WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]);

    let events: Vec<JournalEvent> = (0..4u64)
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
        .expect("batch should succeed");
    write_header(&journal, run, digest);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot ok");

    let diag1 = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("first diagnostic should succeed");
    let diag2 = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("second diagnostic should succeed");

    assert_eq!(diag1.total_runs, diag2.total_runs);
    assert_eq!(diag1.eligible_runs, diag2.eligible_runs);
    assert_eq!(diag1.blocked_runs, diag2.blocked_runs);
    assert_eq!(diag1.total_events_trimmable, diag2.total_events_trimmable);
    assert_eq!(diag1.runs.len(), diag2.runs.len());
    for (a, b) in diag1.runs.iter().zip(diag2.runs.iter()) {
        assert_eq!(a, b, "diagnostic results should be identical across calls");
    }
}

#[test]
fn diagnostic_returns_empty_for_empty_journal() {
    let (_temp, journal) = temp_journal();

    let diag = journal
        .trim_eligibility_diagnostic(TrimPolicy::default())
        .expect("diagnostic should succeed");

    assert_eq!(diag.total_runs, 0);
    assert_eq!(diag.eligible_runs, 0);
    assert_eq!(diag.blocked_runs, 0);
    assert_eq!(diag.total_events_trimmable, 0);
    assert!(diag.runs.is_empty());
}
