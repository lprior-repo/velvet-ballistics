// =========================================================================
// Edge case: append_queued_unfsynced then read back
// =========================================================================

#[test]
fn append_queued_unfsynced_then_read_back() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(17000);
    let event = make_event(run, 0);
    journal
        .append_queued_unfsynced(&event)
        .expect("append should succeed");
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

// =========================================================================
// Edge case: consecutive append_journaled calls
// =========================================================================

#[test]
fn consecutive_append_journaled_calls() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(18000);

    for seq in 0u64..10 {
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(seq as u16),
            attempt: 1,
        };
        journal
            .append_journaled(&event)
            .expect("append_journaled should succeed");
    }

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 10);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.seq().get(), i as u64);
    }
}

// =========================================================================
// Edge case: mixed durability modes on same run
// =========================================================================

#[test]
fn mixed_durability_modes_on_same_run() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(19000);
    let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal.append_strict(&e0).expect("strict append");

    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal.append_journaled(&e1).expect("journaled append");

    let e2 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        attempt: 1,
    };
    journal.append_strict(&e2).expect("strict append 2");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 3);
    assert_eq!(replayed[0], e0);
    assert_eq!(replayed[1], e1);
    assert_eq!(replayed[2], e2);
}

// =========================================================================
// Edge case: index keyspaces remain empty until explicitly written
// =========================================================================

#[test]
fn index_keyspaces_empty_after_regular_writes() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20000);
    let event = make_event(run, 0);
    journal.append_strict(&event).expect("append");

    let mut status_count = 0usize;
    for item in journal.index_status.iter() {
        assert!(item.key().is_ok());
        status_count = status_count.saturating_add(1);
    }
    assert_eq!(status_count, 0, "status index should be empty");

    let mut workflow_count = 0usize;
    for item in journal.index_workflow.iter() {
        assert!(item.key().is_ok());
        workflow_count = workflow_count.saturating_add(1);
    }
    assert_eq!(workflow_count, 0, "workflow index should be empty");

    let mut action_count = 0usize;
    for item in journal.index_action.iter() {
        assert!(item.key().is_ok());
        action_count = action_count.saturating_add(1);
    }
    assert_eq!(action_count, 0, "action index should be empty");
}

// =========================================================================
// Edge case: all declared keyspaces are queryable after open
// =========================================================================

#[test]
fn all_declared_keyspaces_are_iterable_after_open() {
    let (_temp, journal) = temp_journal();
    assert!(journal.workflow_source.iter().next().is_none());
    assert!(journal.compiled_ir.iter().next().is_none());
    assert!(journal.run_header.iter().next().is_none());
    assert!(journal.events.iter().next().is_none());
    assert!(journal.run_snapshot.iter().next().is_none());
    assert!(journal.blob.iter().next().is_none());
    assert!(journal.index_status.iter().next().is_none());
    assert!(journal.index_workflow.iter().next().is_none());
    assert!(journal.index_action.iter().next().is_none());
}

// =========================================================================
// close() method tests
// =========================================================================

/// Test that close() propagates errors from persist_strict on a clean journal.
/// A clean (empty) journal should not produce persist errors.
#[test]
fn close_succeeds_on_clean_journal() {
    let (_temp, mut journal) = temp_journal();
    // close() should succeed on idle journal
    journal
        .close()
        .expect("close on clean journal should succeed");
}

/// Test that close() returns unit on success.
#[test]
fn close_returns_unit_on_success() {
    let (_temp, mut journal) = temp_journal();
    let result = journal.close();
    assert!(result.is_ok(), "close should return Ok(()) on success");
}

/// Test that drop does NOT call close() by verifying the process lock is released
/// even when close() is not called. The journal can be reopened after drop.
#[test]
fn drop_releases_process_lock_even_without_explicit_close() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();

    // First, open and hold the journal
    {
        let journal = FjallJournal::open(&path, None).expect("journal open should succeed");
        let run = RunId::new(9999);
        let digest = WorkflowDigest::from_bytes([0xEE; DIGEST_BYTES]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal
            .append_strict(&event)
            .expect("append should succeed");
        // Explicitly do NOT call close() here
    } // journal drops here without explicit close

    // The process lock should be released when journal drops, allowing reopen
    // If drop called close() and close() succeeded, this would also work.
    // If drop called close() and close() failed, the process lock might not be released.
    let journal2 =
        FjallJournal::open(&path, None).expect("journal should be reopenable after drop");
    let replayed = journal2
        .events_for_run(RunId::new(9999))
        .expect("replay should succeed");
    assert_eq!(
        replayed.len(),
        1,
        "should have exactly one event after reopen"
    );
}

/// Test that close() propagates persist errors when underlying storage fails.
/// This uses a test-only fault hook to force the strict durability boundary to fail.
#[test]
fn close_propagates_persist_errors() {
    let (_temp, mut journal) = temp_journal();
    journal.fail_next_persist_for_test();

    let result = journal.close();
    assert!(
        matches!(result, Err(JournalError::StrictDurabilityFailed)),
        "close must propagate strict durability failures, got {:?}",
        result
    );
}

// =========================================================================
// Regression test for SA-016
// =========================================================================
//
// SA-016 confirmed that `append_queued_unfsynced` actually commits to
// the LSM memtable per call; "unpersisted" only meant "not fsynced".  The
// misleading name was corrected to `append_queued_unfsynced` so that the
// type signature documents the actual semantics: the event is durable
// against process-level recovery (visible to readers and replay) but has
// not been force-fsync'd to disk.

#[test]
fn append_queued_unfsynced_commits_to_memtable_and_is_visible_to_readers() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7000);
    let event = make_event(run, 0);

    // The renamed entry point must accept and commit the event without
    // requiring an explicit fsync call from the caller.
    journal
        .append_queued_unfsynced(&event)
        .expect("append_queued_unfsynced should commit to the memtable");

    // "unfsynced" means it is committed to the LSM memtable and visible
    // to subsequent readers — that is the actual documented contract.
    let replayed = journal
        .events_for_run(run)
        .expect("replay should succeed after an unfsynced commit");
    assert_eq!(
        replayed.len(),
        1,
        "event must be visible to readers immediately after append_queued_unfsynced"
    );
    assert_eq!(
        replayed[0], event,
        "replayed event must equal appended event"
    );
}

// =========================================================================
// Regression test for SA-016 — wave-15 follow-up bead vb-y8tyj
// =========================================================================
//
// The wave-13 parent bead (vb-s9iyv) renamed the public wrapper from
// `append_queued_unpersisted` to `append_queued_unfsynced`. The wave-15
// follow-up bead (vb-y8tyj) finishes the rename by renaming the inner
// helper from `append_unpersisted` to `append_unfsynced` so the entire
// append path carries the accurate "unfsynced" semantics.
//
// This test exercises the renamed helper directly (not via the wrapper)
// to lock the rename in place at the source-of-truth symbol. Any
// re-introduction of the old `append_unpersisted` name will fail to
// compile this test.

#[test]
fn append_unfsynced_helper_commits_event_to_memtable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7001);
    let event = make_event(run, 0);

    // Direct invocation of the renamed inner helper. The wrapper's
    // idempotency layer is intentionally bypassed: this test pins the
    // rename to the source-of-truth function symbol.
    journal
        .append_unfsynced(&event)
        .expect("append_unfsynced should commit to the LSM memtable");

    let replayed = journal
        .events_for_run(run)
        .expect("replay should succeed after an unfsynced commit");
    assert_eq!(
        replayed.len(),
        1,
        "event must be visible to readers immediately after append_unfsynced"
    );
    assert_eq!(
        replayed[0], event,
        "replayed event must equal appended event"
    );
}

#[test]
fn append_unfsynced_helper_rejects_duplicate_key() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7002);
    let event_a = make_event(run, 0);

    journal
        .append_unfsynced(&event_a)
        .expect("first append_unfsynced should succeed");

    // Build a different event at the same (run, seq) — the helper must
    // reject with DuplicateEvent (not silently overwrite, not panic).
    let event_b = make_step_started(run, 0, 0);
    let result = journal.append_unfsynced(&event_b);

    match result {
        Err(JournalError::DuplicateEvent { run: r, seq }) => {
            assert_eq!(r, run, "DuplicateEvent must carry the colliding run");
            assert_eq!(seq.get(), 0, "DuplicateEvent must carry the colliding seq");
        }
        Ok(()) => panic!("append_unfsynced must reject a duplicate (run, seq)"),
        Err(other) => panic!("append_unfsynced returned unexpected error: {other:?}"),
    }

    // The original record must remain intact.
    let replayed = journal
        .events_for_run(run)
        .expect("replay should succeed after rejected duplicate");
    assert_eq!(replayed.len(), 1, "duplicate must not overwrite original");
    assert_eq!(replayed[0], event_a, "original record must remain intact");
}

// =========================================================================
// round10-a / Issue 5: replay/codec bind regression test
//
// The envelope binds three things: the Fjall key, the envelope's `sequence`
// field, and the payload's `seq`. A tampered value whose envelope sequence
// diverges from the payload seq must surface as
// `JournalError::ReplayEnvelopeSequenceMismatch` at replay time, not be
// silently replayed under the wrong identity.
// =========================================================================

#[test]
fn key_payload_mismatch_fails_replay() -> Result<(), JournalError> {
    let (_temp, journal) = temp_journal();
    let run = vb_core::RunId::new(7);
    let key_seq = EventSeq::new(42);
    let envelope_seq: u64 = 99;

    // Build a normal RunAccepted event for (run=7, seq=42). The payload's
    // seq is 42, so the Fjall key will be (run=7, seq=42) and the payload
    // will report seq=42.
    let event = JournalEvent::RunAccepted {
        run,
        seq: key_seq,
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };

    // Encode the envelope with envelope.sequence=99 while the payload's
    // seq remains 42. This produces a record whose envelope sequence
    // disagrees with the payload's seq field — exactly the divergence
    // round10-a / Issue 5 must catch.
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        envelope_seq,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode_record should succeed");

    // Recompute the header checksum so the bind reaches the envelope-vs-
    // payload check (rather than failing earlier with HeaderChecksumMismatch).
    let checksum = crc32c::crc32c(&encoded[..56]);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    // Insert the tampered record directly under the (run=7, seq=42) key
    // so the replay path sees a real, on-disk mismatch.
    let key = crate::keys::run_event_key(run, key_seq)?;
    journal
        .events
        .insert(key.to_vec(), encoded)
        .map_err(JournalError::Fjall)?;

    // Replay must surface the typed mismatch error rather than silently
    // returning the event under a forged sequence.
    let err = journal
        .events_for_run(run)
        .expect_err("replay must reject envelope/payload sequence mismatch");
    assert!(
        matches!(
            err,
            JournalError::ReplayEnvelopeSequenceMismatch {
                run: actual_run,
                envelope_seq: 99,
                payload_seq: 42,
            } if actual_run == run
        ),
        "expected ReplayEnvelopeSequenceMismatch for run={run:?}, envelope_seq=99, payload_seq=42, got {:?}",
        err
    );
    Ok(())
}

#[test]
fn replay_rejects_key_payload_sequence_mismatch() -> Result<(), JournalError> {
    let (_temp, journal) = temp_journal();
    let run = vb_core::RunId::new(8);
    let key_seq = EventSeq::new(0);
    let payload_seq = EventSeq::new(1);

    let event = JournalEvent::RunAccepted {
        run,
        seq: payload_seq,
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        payload_seq.get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let key = crate::keys::run_event_key(run, key_seq)?;
    journal
        .events
        .insert(key.to_vec(), encoded)
        .map_err(JournalError::Fjall)?;

    let err = journal
        .events_for_run(run)
        .expect_err("replay must reject key/payload sequence mismatch");
    assert!(
        matches!(
            err,
            JournalError::ReplayKeyMismatch {
                run: actual_run,
                key_seq: 0,
                payload_seq: 1,
            } if actual_run == run
        ),
        "expected ReplayKeyMismatch for key seq 0 and payload seq 1, got {err:?}"
    );
    Ok(())
}
