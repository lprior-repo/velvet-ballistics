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
