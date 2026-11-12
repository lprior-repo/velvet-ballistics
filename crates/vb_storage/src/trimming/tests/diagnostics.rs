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
