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

#[test]
fn latest_durable_snapshot_seq_does_not_decode_snapshot_value() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x903);

    let good_snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(7),
        workflow: WorkflowDigest::from_bytes([0x71; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&good_snapshot)
        .expect("put_snapshot should succeed");

    let poison_key = crate::keys::run_snapshot_key(run, EventSeq::new(42))
        .expect("run_snapshot_key should succeed");
    let poison_value: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
    journal
        .run_snapshot
        .insert(poison_key.to_vec(), poison_value)
        .expect("direct insert should succeed");

    let latest = journal
        .latest_durable_snapshot_seq(run)
        .expect("key-only lookup must not decode the poison value");
    assert_eq!(
        latest,
        Some(EventSeq::new(42)),
        "highest seq in the prefix must be returned even when its value is undecodable; \
         a decode_record-based loop would have surfaced BadMagic instead"
    );
}

#[test]
fn latest_durable_snapshot_seq_rejects_overlong_snapshot_key() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x904);
    let digest = WorkflowDigest::from_bytes([0x72; DIGEST_BYTES]);

    let good_snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(7),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&good_snapshot)
        .expect("put_snapshot should succeed");

    let mut overlong_key = crate::keys::run_snapshot_key(run, EventSeq::new(42))
        .expect("run_snapshot_key should succeed")
        .to_vec();
    overlong_key.push(0xAA);
    journal
        .run_snapshot
        .insert(overlong_key.clone(), vec![0xDE, 0xAD, 0xBE, 0xEF])
        .expect("overlong snapshot key insert should succeed");

    let err = journal
        .latest_durable_snapshot_seq(run)
        .expect_err("overlong snapshot key must fail closed");
    assert!(
        matches!(err, TrimError::IncompleteTrim { deleted_count: 0 }),
        "overlong snapshot key must surface IncompleteTrim with deleted_count=0, got {err:?}"
    );
    let loaded_good_snapshot = journal
        .snapshot(run, EventSeq::new(7))
        .expect("good snapshot lookup should succeed after failed trim lookup")
        .expect("good snapshot must remain after failed trim lookup");
    assert_eq!(
        loaded_good_snapshot, good_snapshot,
        "failed trim lookup must not alter the existing durable snapshot"
    );
    let poisoned_snapshot = journal
        .run_snapshot
        .get(overlong_key.as_slice())
        .expect("overlong snapshot key lookup should succeed after failed trim lookup");
    assert!(
        poisoned_snapshot.is_some(),
        "failed trim lookup must not mutate the overlong snapshot key"
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
