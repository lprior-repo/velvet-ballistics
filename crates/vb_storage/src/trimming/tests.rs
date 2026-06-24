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
    assert_eq!(remaining.len(), 4, "should preserve events 6-9");
    for event in &remaining {
        assert!(
            event.seq().get() >= 6,
            "event seq {} should be >= 6 (after snapshot at 5)",
            event.seq().get()
        );
    }
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

// =========================================================================
// vb-1rqz7.27 / SC-007 — zero-delete trim returns NoOp regardless of policy
// =========================================================================

#[test]
fn trim_zero_deletes_returns_noop_when_skip_noop_disabled() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x7B01);
    let digest = WorkflowDigest::from_bytes([0x80; DIGEST_BYTES]);
    write_header(&journal, run, digest);

    // Snapshot at the same seq as the existing event so nothing is trimmable.
    journal
        .append_journaled(&make_event(run, 0))
        .expect("append should succeed");

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .expect("snapshot should succeed");

    let policy = TrimPolicy {
        skip_noop_runs: false,
        ..TrimPolicy::default()
    };
    let result = journal
        .trim_events_for_run(run, policy)
        .expect("trim should succeed");
    assert_eq!(
        result.deleted_count, 0,
        "no events should be deleted when snapshot is ahead"
    );
    assert_eq!(
        result.status,
        TrimStatus::NoOp,
        "vb-1rqz7.27: zero-delete trim must report NoOp regardless of skip_noop_runs"
    );
}

#[test]
fn trim_given_run_with_no_snapshot_returns_error() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);

    let events = [make_event(run, 0), make_step_started(run, 1, 0)];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let result = journal.trim_events_for_run(run, TrimPolicy::default());
    assert!(
        matches!(result, Err(TrimError::NoDurableSnapshot { .. })),
        "should error when no durable snapshot exists, got {:?}",
        result
    );
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
    write_header(&journal, run, digest);

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
    let Some(header_rec) = header else {
        panic!("trim must preserve run header");
    };
    assert_eq!(
        header_rec.run, run,
        "preserved header must match trimmed run"
    );

    let snap = journal
        .snapshot(run, EventSeq::new(2))
        .expect("snapshot lookup should succeed");
    let Some(snapshot) = snap else {
        panic!("trim must preserve snapshot for trimmed run");
    };
    assert_eq!(
        snapshot.seq.get(),
        2,
        "preserved snapshot must have expected seq"
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

// =========================================================================
// vb-1rqz7.29 / SC-004 — latest_durable_snapshot_seq uses key-based lookup
// =========================================================================

#[test]
fn latest_durable_snapshot_seq_reads_max_key_without_decoding_value() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x901);
    let digest = WorkflowDigest::from_bytes([0x91; DIGEST_BYTES]);

    // Multiple snapshots; the key-based lookup must return the highest seq
    // even when only the last-inserted key sorts highest.
    for seq in [3u64, 9, 5, 1, 7] {
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(seq),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("put_snapshot should succeed");
    }

    let latest = journal
        .latest_durable_snapshot_seq(run)
        .expect("latest seq lookup must succeed");
    assert_eq!(
        latest,
        Some(EventSeq::new(9)),
        "key-based lookup must return the highest seq from the snapshot prefix"
    );
}

#[test]
fn latest_durable_snapshot_seq_returns_none_when_no_snapshots() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x902);

    let latest = journal
        .latest_durable_snapshot_seq(run)
        .expect("empty lookup must succeed");
    assert!(
        latest.is_none(),
        "no snapshots must yield None without scanning any value"
    );
}

// =========================================================================
// vb-1rqz7.30 / SC-005 — batch trim computes retention in a single pass
// =========================================================================

#[test]
fn compute_retained_terminal_runs_matches_per_run_check() {
    use vb_core::WorkflowId;

    let (_temp, journal) = temp_journal();
    let workflow_a = WorkflowId::new(0xA1);
    let workflow_b = WorkflowId::new(0xB1);

    // Workflow A: three terminal runs (retain 2 → expect runs 2 and 3 to be retained).
    let headers_a = [
        (RunId::new(0xA001), 100u64),
        (RunId::new(0xA002), 200),
        (RunId::new(0xA003), 300),
    ];
    // Workflow B: one terminal run (retain 2 → expect run to be retained).
    let headers_b = [(RunId::new(0xB001), 400u64)];

    let placeholder = WorkflowDigest::from_bytes([0u8; DIGEST_BYTES]);
    for (run, ts) in headers_a.iter().chain(headers_b.iter()) {
        write_header_with_workflow(&journal, *run, workflow_a, placeholder, *ts);
        journal
            .append_journaled(&make_run_finished(*run, 0))
            .expect("terminal event append");
    }
    // Overwrite B's header workflow_id so it sorts to workflow B.
    for (run, ts) in headers_b.iter() {
        write_header_with_workflow(&journal, *run, workflow_b, placeholder, *ts);
        journal
            .append_journaled(&make_run_finished(*run, 1))
            .expect("terminal event append");
    }

    let policy = TrimPolicy {
        retain_last_n_terminal: 2,
        ..TrimPolicy::default()
    };
    let retained = journal
        .compute_retained_terminal_runs(&policy)
        .expect("retained set should compute");

    // Workflow A: top-2 by accepted_at_ms (run 2 @ 200, run 3 @ 300).
    assert!(retained.contains(&RunId::new(0xA002)));
    assert!(retained.contains(&RunId::new(0xA003)));
    assert!(
        !retained.contains(&RunId::new(0xA001)),
        "oldest workflow-A run must not be retained"
    );
    // Workflow B: only one terminal run, retained.
    assert!(retained.contains(&RunId::new(0xB001)));
}

#[test]
fn compute_retained_terminal_runs_empty_when_retention_zero() {
    let (_temp, journal) = temp_journal();

    let policy = TrimPolicy {
        retain_last_n_terminal: 0,
        ..TrimPolicy::default()
    };
    let retained = journal
        .compute_retained_terminal_runs(&policy)
        .expect("empty retention policy must succeed");
    assert!(
        retained.is_empty(),
        "zero retention policy must yield empty set without scanning"
    );
}

// =========================================================================
// Pure helper: `retained_terminal_runs_top_n` (no I/O)
// =========================================================================

#[test]
fn retained_terminal_runs_top_n_pure_helper_picks_newest_per_workflow() {
    // Pre-grouped terminal runs, ordered arbitrarily. The pure helper
    // sorts each workflow's runs by `accepted_at_ms` descending and
    // takes the top N. No journal, no I/O.
    let workflow_a = WorkflowId::new(0xA_AAAAA);
    let workflow_b = WorkflowId::new(0xB_BBBBB);

    let mut by_workflow: std::collections::BTreeMap<WorkflowId, Vec<(RunId, u64)>> =
        std::collections::BTreeMap::new();
    by_workflow.insert(
        workflow_a,
        vec![
            (RunId::new(0xA_0001), 100),
            (RunId::new(0xA_0002), 300),
            (RunId::new(0xA_0003), 200),
        ],
    );
    by_workflow.insert(
        workflow_b,
        vec![(RunId::new(0xB_0001), 50), (RunId::new(0xB_0002), 75)],
    );

    let retained = FjallJournal::retained_terminal_runs_top_n(by_workflow, 1);
    // Per-workflow top-1 by accepted_at_ms desc:
    //   A: (0xA_0002, 300) — newest
    //   B: (0xB_0002, 75)   — newest
    assert_eq!(retained.len(), 2, "one per workflow");
    assert!(retained.contains(&RunId::new(0xA_0002)));
    assert!(retained.contains(&RunId::new(0xB_0002)));
    assert!(!retained.contains(&RunId::new(0xA_0001)));
    assert!(!retained.contains(&RunId::new(0xA_0003)));
    assert!(!retained.contains(&RunId::new(0xB_0001)));
}

#[test]
fn retained_terminal_runs_top_n_pure_helper_zero_yields_empty() {
    let workflow_a = WorkflowId::new(0xA_AAAAA);
    let mut by_workflow: std::collections::BTreeMap<WorkflowId, Vec<(RunId, u64)>> =
        std::collections::BTreeMap::new();
    by_workflow.insert(
        workflow_a,
        vec![(RunId::new(0xA_0001), 100), (RunId::new(0xA_0002), 200)],
    );
    let retained = FjallJournal::retained_terminal_runs_top_n(by_workflow, 0);
    assert!(retained.is_empty(), "zero retention must short-circuit");
}

#[test]
fn retained_terminal_runs_top_n_pure_helper_top_n_larger_than_set_keeps_all() {
    let workflow_a = WorkflowId::new(0xA_AAAAA);
    let mut by_workflow: std::collections::BTreeMap<WorkflowId, Vec<(RunId, u64)>> =
        std::collections::BTreeMap::new();
    by_workflow.insert(
        workflow_a,
        vec![(RunId::new(0xA_0001), 100), (RunId::new(0xA_0002), 200)],
    );
    let retained = FjallJournal::retained_terminal_runs_top_n(by_workflow, 10);
    assert_eq!(retained.len(), 2, "top-N larger than set keeps all");
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

#[test]
fn trim_all_eligible_runs_batch_skips_retained_and_trims_others() {
    // vb-uu31g / SC-005: regression guard for the batch path. With 5
    // terminal runs in the same workflow and retain_last_n_terminal=3,
    // only the 2 oldest runs (positions 0 and 1 after newest-first sort)
    // should be trimmed. The 3 newest must be left intact because the
    // batch path consults the precomputed retained set and skips them
    // without re-deriving retention per call.
    let (_temp, journal) = temp_journal();
    let workflow_id = WorkflowId::new(0xCAFE);
    let digest = WorkflowDigest::from_bytes([0xC0; DIGEST_BYTES]);

    for run_id in 1u64..=5 {
        write_terminal_run_with_snapshot(
            &journal,
            RunId::new(run_id),
            workflow_id,
            digest,
            run_id * 100,
        );
    }

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 3,
    };
    let results = journal
        .trim_all_eligible_runs(policy)
        .expect("batch trim should succeed");

    let trimmed_runs: std::collections::HashSet<RunId> = results.iter().map(|r| r.run).collect();
    let expected_trimmed: std::collections::HashSet<RunId> =
        [RunId::new(1), RunId::new(2)].into_iter().collect();
    assert_eq!(
        trimmed_runs, expected_trimmed,
        "vb-uu31g / SC-005: batch trim must trim only the 2 oldest terminal \
         runs (runs 1 and 2); runs 3, 4, 5 are retained by the policy and must \
         not be touched. got={trimmed_runs:?}"
    );
}

#[test]
fn trim_all_eligible_runs_batch_trims_non_terminal_alongside_terminal() {
    // vb-uu31g / SC-005: non-terminal runs are never retention-blocked,
    // so they should be trimmed in the same batch as eligible terminal
    // runs. The precomputed retained set only contains the protected
    // terminal runs; everything else flows through trim_events_for_run_inner.
    let (_temp, journal) = temp_journal();
    let workflow_id = WorkflowId::new(0xBEEF);
    let digest = WorkflowDigest::from_bytes([0xBE; DIGEST_BYTES]);

    // 3 terminal runs: oldest (1) trimmable, newest two (2, 3) retained.
    for run_id in 1u64..=3 {
        write_terminal_run_with_snapshot(
            &journal,
            RunId::new(run_id),
            workflow_id,
            digest,
            run_id * 100,
        );
    }
    // 1 non-terminal run that should also be trimmed.
    write_non_terminal_run_with_snapshot(&journal, RunId::new(4), workflow_id, digest, 4_000);

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 2,
    };
    let results = journal
        .trim_all_eligible_runs(policy)
        .expect("batch trim should succeed");

    let trimmed_runs: std::collections::HashSet<RunId> = results.iter().map(|r| r.run).collect();
    let expected_trimmed: std::collections::HashSet<RunId> = [
        RunId::new(1), // oldest terminal — trimmable
        RunId::new(4), // non-terminal — trimmable
    ]
    .into_iter()
    .collect();
    assert_eq!(
        trimmed_runs, expected_trimmed,
        "vb-uu31g / SC-005: batch trim must trim the unprotected oldest \
         terminal run (1) and the non-terminal run (4); runs 2 and 3 must \
         be skipped via the precomputed retained set. got={trimmed_runs:?}"
    );

    // Run 2 (protected terminal) must still be in the retained set on a
    // fresh pre-pass — if the batch had trimmed it, the pre-pass would
    // no longer find its events and would no longer retain it.
    let retained_after = journal
        .compute_retained_terminal_runs(&policy)
        .expect("retained set should recompute");
    assert!(
        retained_after.contains(&RunId::new(2)),
        "vb-uu31g / SC-005: protected terminal run 2 must remain retained \
         after the batch trim (events must still be intact)"
    );
    assert!(
        retained_after.contains(&RunId::new(3)),
        "vb-uu31g / SC-005: protected terminal run 3 must remain retained \
         after the batch trim (events must still be intact)"
    );
    // Run 1 (trimmed) must no longer be retained since its events were
    // deleted by the batch trim.
    assert!(
        !retained_after.contains(&RunId::new(1)),
        "vb-uu31g / SC-005: trimmed terminal run 1 must no longer be retained \
         after its events were deleted by the batch trim"
    );
}

#[test]
fn trim_all_eligible_runs_batch_skip_no_snapshot_still_skipped() {
    // vb-uu31g / SC-005: even with the inner-helper shortcut, runs without
    // a durable snapshot must surface as NoDurableSnapshot and be skipped
    // by the batch loop, never silently trimmed.
    let (_temp, journal) = temp_journal();
    let workflow_id = WorkflowId::new(0xDEAD);
    let digest = WorkflowDigest::from_bytes([0xDE; DIGEST_BYTES]);

    // Run 100: terminal + snapshot → eligible (not in retention top-1).
    write_terminal_run_with_snapshot(&journal, RunId::new(100), workflow_id, digest, 10_000);
    // Run 200: terminal + snapshot → eligible (not in retention top-1).
    write_terminal_run_with_snapshot(&journal, RunId::new(200), workflow_id, digest, 20_000);
    // Run 300: terminal, NO snapshot → newest so it IS in retention top-1,
    // but the batch path must still skip it because it lacks a durable
    // snapshot (must surface NoDurableSnapshot, not silently trim).
    let events: Vec<JournalEvent> = (0..3u64)
        .map(|i| {
            if i == 0 {
                make_event(RunId::new(300), i)
            } else if i == 2 {
                make_run_finished(RunId::new(300), i)
            } else {
                make_step_started(RunId::new(300), i, i as u16 - 1)
            }
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("no-snapshot run batch should succeed");
    write_header_with_workflow(&journal, RunId::new(300), workflow_id, digest, 30_000);
    // deliberately no put_snapshot call

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 1,
    };
    let results = journal
        .trim_all_eligible_runs(policy)
        .expect("batch trim should succeed despite missing snapshot");

    let trimmed_runs: std::collections::HashSet<RunId> = results.iter().map(|r| r.run).collect();
    let expected_trimmed: std::collections::HashSet<RunId> = [
        RunId::new(100), // unprotected, has snapshot → trimmed
        RunId::new(200), // unprotected, has snapshot → trimmed
    ]
    .into_iter()
    .collect();
    assert_eq!(
        trimmed_runs, expected_trimmed,
        "vb-uu31g / SC-005: only the unprotected terminal runs with snapshots \
         (100 and 200) should be trimmed; run 300 must surface NoDurableSnapshot \
         and be skipped, not silently trimmed. got={trimmed_runs:?}"
    );

    // Run 300 (terminal, no snapshot) must NOT be in any post-batch
    // retained set either, because the batch should not have touched
    // it. We verify by inspecting the raw event keyspace: if the batch
    // had silently trimmed, the RunFinished event at seq=2 would be gone.
    let event_key_seq2 = crate::keys::run_event_key(RunId::new(300), EventSeq::new(2))
        .expect("key encode should succeed");
    let still_present = journal
        .events
        .get(event_key_seq2.as_slice())
        .expect("event lookup should succeed");
    assert!(
        still_present.is_some(),
        "vb-uu31g / SC-005: no-snapshot run 300 must keep its RunFinished \
         event intact — a missing entry would mean the batch silently \
         trimmed past the NoDurableSnapshot guard"
    );
}

// =========================================================================
// vb-1rqz7.26 / SC-006 — trim_events_for_run fails closed on malformed keys
// =========================================================================

#[test]
fn trim_events_for_run_fails_closed_on_malformed_event_key() {
    use crate::constants::PREFIX_RUN_EVENT;

    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x7A01);

    let digest = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
    write_header(&journal, run, digest);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot put");

    // Plant a real 17-byte event so the run is recognized as terminal-free.
    let event = make_event(run, 0);
    journal
        .append_journaled(&event)
        .expect("append real event should succeed");

    // Plant a corrupted key (shorter than the 17-byte run-event contract)
    // directly under the same run-event prefix so the trim scan encounters it.
    // Use a properly-encoded event value so the value-decoding scan stays
    // green and the short-key contract violation is the only thing the trim
    // loop sees.
    let mut short_key = [0u8; 9];
    short_key[0] = PREFIX_RUN_EVENT;
    short_key[1..9].copy_from_slice(&run.get().to_be_bytes());
    let real_value = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &make_event(run, 0),
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("event record encode");
    journal
        .events
        .insert(short_key.to_vec(), real_value)
        .expect("malformed short key insert");

    let policy = TrimPolicy::default();
    let err = journal
        .trim_events_for_run(run, policy)
        .expect_err("short key must fail closed");
    assert!(
        matches!(err, TrimError::IncompleteTrim { .. }),
        "short event key must surface as IncompleteTrim, got {err:?}"
    );
}

// =========================================================================
// vb-1rqz7.25 / CC-002 — count_trimmable_events fails closed on malformed keys
// =========================================================================

#[test]
fn trim_eligibility_diagnostic_fails_closed_on_malformed_event_key() {
    use crate::constants::PREFIX_RUN_EVENT;

    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x7A02);

    let digest = WorkflowDigest::from_bytes([0x78; DIGEST_BYTES]);
    write_header(&journal, run, digest);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("snapshot put");

    let event = make_event(run, 0);
    journal.append_journaled(&event).expect("append event");

    let mut short_key = [0u8; 9];
    short_key[0] = PREFIX_RUN_EVENT;
    short_key[1..9].copy_from_slice(&run.get().to_be_bytes());
    let real_value = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &make_event(run, 0),
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("event record encode");
    journal
        .events
        .insert(short_key.to_vec(), real_value)
        .expect("malformed short key insert");

    let policy = TrimPolicy::default();
    let err = journal
        .trim_eligibility_diagnostic(policy)
        .expect_err("short key must fail closed");
    let converted = match err {
        JournalError::Trim(inner) => *inner,
        other => panic!("trim diagnostic must wrap as JournalError::Trim, got {other:?}"),
    };
    assert!(
        matches!(converted, TrimError::IncompleteTrim { .. }),
        "trim diagnostic must surface IncompleteTrim, got {converted:?}"
    );
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
    assert_eq!(remaining.len(), 1, "should preserve event 3 only");
    for event in &remaining {
        assert!(
            event.seq().get() >= 3,
            "event seq {} should be >= 2",
            event.seq().get()
        );
    }
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
    let result = journal.trim_events_for_run(run, policy);
    assert!(
        matches!(result, Err(TrimError::RetentionPolicyBlocks { .. })),
        "recent terminal run should be blocked by retention, got {:?}",
        result
    );
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
    assert_eq!(after_trim.len(), 2, "should preserve events 4-5");
    for (i, event) in after_trim.iter().enumerate() {
        let expected_seq = 4 + i as u64;
        assert_eq!(
            event.seq().get(),
            expected_seq,
            "event at index {} should have seq {}",
            i,
            expected_seq
        );
    }

    // Verify trimmed events are actually gone by trying to read them directly
    for seq in 0..3u64 {
        let key = crate::keys::run_event_key(run, EventSeq::new(seq)).expect("key ok");
        let got = journal.events.get(key).expect("get ok");
        assert!(
            got.is_none(),
            "event seq {} should be deleted after trim, but was found: {:?}",
            seq,
            got
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
fn journal_wrapped_error_delegates_to_inner_diagnostic_code() {
    use crate::error::JournalError;

    let inner = JournalError::WrongRun {
        expected: RunId::new(1),
        actual: RunId::new(2),
    };
    let inner_code = inner.diagnostic_code();
    let err = TrimError::Journal(inner);
    assert_eq!(
        err.diagnostic_code(),
        inner_code,
        "TrimError::Journal must delegate to its inner JournalError diagnostic code"
    );
    assert_ne!(
        err.diagnostic_code(),
        JournalError::FJALL_CODE,
        "delegation must not fall back to the generic FJALL_CODE"
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
    let Some(&TrimEligibility::Eligible {
        run: eligible_run, ..
    }) = eligible
    else {
        panic!(
            "run A should be Eligible in diagnostic, got: {:?}",
            diag.runs
        );
    };
    assert_eq!(eligible_run, run_a, "eligible run must match run A");

    let blocked = diag
        .runs
        .iter()
        .find(|r| matches!(r, TrimEligibility::Blocked { run, .. } if run == &run_b));
    let Some(&TrimEligibility::Blocked {
        run: blocked_run, ..
    }) = blocked
    else {
        panic!(
            "run B should be Blocked in diagnostic, got: {:?}",
            diag.runs
        );
    };
    assert_eq!(blocked_run, run_b, "blocked run must match run B");
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
    let Some((safe_point, trimmable)) = eligible else {
        panic!("run should be Eligible in diagnostic, got {:?}", diag.runs);
    };
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
