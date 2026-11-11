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
