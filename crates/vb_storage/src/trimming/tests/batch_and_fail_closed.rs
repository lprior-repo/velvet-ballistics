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
    let event_key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("event key encode should succeed");
    let before_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup before failed trim should succeed")
        .expect("event must exist before failed trim")
        .to_vec();

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
        .insert(short_key.to_vec(), real_value.clone())
        .expect("malformed short key insert");

    let policy = TrimPolicy::default();
    let err = journal
        .trim_events_for_run(run, policy)
        .expect_err("short key must fail closed");
    assert!(
        matches!(err, TrimError::IncompleteTrim { deleted_count: 0 }),
        "short event key must surface IncompleteTrim with deleted_count=0, got {err:?}"
    );
    let after_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup after failed trim should succeed")
        .expect("event must remain after failed trim")
        .to_vec();
    assert_eq!(
        after_event_bytes, before_event_bytes,
        "failed trim must not delete or rewrite the existing event"
    );
    let after_short_key = journal
        .events
        .get(short_key.as_slice())
        .expect("short key lookup after failed trim should succeed")
        .expect("short key must remain after failed trim")
        .to_vec();
    assert_eq!(
        after_short_key, real_value,
        "failed trim must leave the malformed short key bytes unchanged"
    );
}

#[test]
fn trim_events_for_run_fails_closed_on_overlong_event_key() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(0x7A11);

    let digest = WorkflowDigest::from_bytes([0x79; DIGEST_BYTES]);
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
    journal
        .append_journaled(&event)
        .expect("append real event should succeed");
    let event_key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("event key encode should succeed");
    let before_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup before failed trim should succeed")
        .expect("event must exist before failed trim")
        .to_vec();

    let mut overlong_key = crate::keys::run_event_key(run, EventSeq::new(1))
        .expect("run_event_key should succeed")
        .to_vec();
    overlong_key.push(0xAA);
    let real_value = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        1,
        &make_event(run, 1),
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("event record encode");
    journal
        .events
        .insert(overlong_key.clone(), real_value.clone())
        .expect("overlong event key insert should succeed");

    let policy = TrimPolicy {
        skip_noop_runs: true,
        retain_last_n_terminal: 0,
    };
    let err = journal
        .trim_events_for_run(run, policy)
        .expect_err("overlong key must fail closed");
    assert!(
        matches!(err, TrimError::IncompleteTrim { deleted_count: 1 }),
        "overlong event key must surface IncompleteTrim with deleted_count=1, got {err:?}"
    );
    let after_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup after failed trim should succeed")
        .expect("event must remain after failed trim")
        .to_vec();
    assert_eq!(
        after_event_bytes, before_event_bytes,
        "failed trim must not commit queued deletion of the existing event"
    );
    let after_overlong_key = journal
        .events
        .get(overlong_key.as_slice())
        .expect("overlong key lookup after failed trim should succeed")
        .expect("overlong key must remain after failed trim")
        .to_vec();
    assert_eq!(
        after_overlong_key, real_value,
        "failed trim must leave the malformed overlong key bytes unchanged"
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
    let event_key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("event key encode should succeed");
    let before_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup before diagnostic should succeed")
        .expect("event must exist before diagnostic")
        .to_vec();

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
        .insert(short_key.to_vec(), real_value.clone())
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
        matches!(converted, TrimError::IncompleteTrim { deleted_count: 0 }),
        "trim diagnostic must surface IncompleteTrim with deleted_count=0, got {converted:?}"
    );
    let after_event_bytes = journal
        .events
        .get(event_key.as_slice())
        .expect("event lookup after diagnostic should succeed")
        .expect("event must remain after failed diagnostic")
        .to_vec();
    assert_eq!(
        after_event_bytes, before_event_bytes,
        "failed trim diagnostic must not mutate existing event bytes"
    );
    let after_short_key = journal
        .events
        .get(short_key.as_slice())
        .expect("short key lookup after diagnostic should succeed")
        .expect("short key must remain after failed diagnostic")
        .to_vec();
    assert_eq!(
        after_short_key, real_value,
        "failed trim diagnostic must leave malformed key bytes unchanged"
    );
}
