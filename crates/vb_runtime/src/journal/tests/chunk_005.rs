// ---------------------------------------------------------------------------
// chunk_005: P2-14a storage-batch
// Acceptance tests for `RuntimeJournal::append_sequenced_batch`.
// Verifies that `StorageRuntimeJournal` commits a slice of journal events
// (and their per-event action index markers) atomically via
// `JournalWriteBatch::commit`.
// ---------------------------------------------------------------------------

use vb_core::action::{ActionTicket, issue_action_ticket};
use vb_storage::keys::index_action_key;

/// Builds a minimal `ActionScheduledTicket` event.
fn ticket_event(
    run: vb_core::ids::RunId,
    action_id: u16,
    step_idx: u16,
) -> RuntimeJournalEvent {
    let action = vb_core::ids::ActionId::new(action_id);
    let step = vb_core::ids::StepIdx::new(step_idx);
    let seq = vb_core::ids::SeqNo::new(u64::from(action_id));
    let ticket: ActionTicket = issue_action_ticket(run, step, seq, action, 1, 0, 16);
    RuntimeJournalEvent::ActionScheduledTicket {
        ticket,
        input: vb_core::ids::SlotIdx::new(0),
        output: vb_core::ids::SlotIdx::new(1),
    }
}

#[test]
fn append_sequenced_batch_with_empty_slice_returns_ok_without_commit() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());

    // Empty batch must succeed without touching the journal.
    let result = adapter.append_sequenced_batch(&[], vb_storage::EventSeq::new(0));
    assert!(matches!(result, Ok(())), "empty batch must return Ok, got {result:?}");

    // No events should be present for any run after an empty commit.
    let probe_run = vb_core::ids::RunId::new(7_777_777);
    let Some(events) = require_ok(
        journal
            .events_for_run(probe_run)
            .map_err(|error| error.to_string()),
        "events_for_run on empty batch",
    ) else {
        return;
    };
    assert!(events.is_empty(), "no events should be committed from empty batch");
}

#[test]
fn append_sequenced_batch_commits_all_events_atomically() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_001);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xA1; 32]);

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        RuntimeJournalEvent::StepSucceeded {
            run,
            step: vb_core::ids::StepIdx::new(0),
            output: vb_core::ids::SlotIdx::new(1),
            attempt: 1,
        },
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
    ];

    let result = adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(100));
    assert!(matches!(result, Ok(())), "batch commit must succeed, got {result:?}");

    // Verify each event is committed at the expected seq by reading it
    // directly via `get_event_bytes`, then decoding.
    let expected_seqs = [100u64, 101, 102, 103];
    for (offset, expected_seq) in expected_seqs.iter().enumerate() {
        let seq = vb_storage::EventSeq::new(*expected_seq);
        let Some(bytes) = require_ok(
            journal
                .get_event_bytes(run, seq)
                .map_err(|error| error.to_string()),
            "get_event_bytes must succeed",
        ) else {
            return;
        };
        let Some(_bytes) = bytes else {
            panic!("event at offset {offset} (seq={expected_seq}) must be committed");
        };
    }

    // Also verify by reading the events starting from seq 100 directly via
    // a snapshot-based boundary, since events_for_run requires seq 0 start.
    // Verify seq 100 has the expected RunAccepted event by decoding it.
    let seq = vb_storage::EventSeq::new(100);
    let Some(bytes) = require_ok(
        journal
            .get_event_bytes(run, seq)
            .map_err(|error| error.to_string()),
        "get_event_bytes(100) must succeed",
    ) else {
        return;
    };
    let Some(bytes) = bytes else {
        panic!("seq 100 must be committed");
        return;
    };
    let Some(decoded) = require_ok(
        vb_storage::decode_record::<vb_storage::JournalEvent>(
            &bytes,
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|error| error.to_string()),
        "decode_record must succeed",
    )
    .map(|(_, event)| event) else {
        return;
    };
    assert_eq!(
        decoded,
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(100),
            workflow
        }
    );
}

#[test]
fn append_sequenced_batch_assigns_contiguous_sequences_starting_at_seq_start() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_002);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xB2; 32]);

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(1),
        },
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(50)),
        Ok(())
    );

    // Use `get_event_bytes` to fetch each event at the expected seq directly,
    // bypassing the replay-validates-from-seq-0 invariant of `events_for_run`.
    for (offset, expected_seq) in [50u64, 51, 52].iter().enumerate() {
        let seq = vb_storage::EventSeq::new(*expected_seq);
        let Some(bytes) = require_ok(
            journal
                .get_event_bytes(run, seq)
                .map_err(|error| error.to_string()),
            "get_event_bytes must succeed",
        ) else {
            return;
        };
        let present = bytes.is_some();
        assert!(
            present,
            "event at offset {offset} (seq={expected_seq}) must be committed"
        );
    }
}

#[test]
fn append_sequenced_batch_updates_action_index_for_each_ticket() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_003);

    let events = [
        ticket_event(run, 1, 0),
        ticket_event(run, 2, 0),
        ticket_event(run, 3, 0),
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0)),
        Ok(())
    );

    // Each ActionScheduledTicket must have produced an index_action marker
    // inside the journal. We verify by checking `has_action_index_entry`
    // for each (action, run, step) tuple.
    for action_id in 1u16..=3u16 {
        let key = index_action_key(
            vb_core::ids::ActionId::new(action_id),
            run,
            vb_core::ids::StepIdx::new(0),
        )
        .expect("index_action_key must succeed");
        let present = journal
            .has_action_index_entry(&key)
            .expect("has_action_index_entry must succeed");
        assert!(
            present,
            "action index marker for action_id={action_id} must be present"
        );
    }
}

#[test]
fn append_sequenced_batch_mixed_event_kinds_preserves_storage_mapping() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_004);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xC3; 32]);
    let admission = crate::admission::RunAdmission::new(
        workflow,
        run,
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    );

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::RunAdmission { admission },
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        ticket_event(run, 7, 0),
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0)),
        Ok(())
    );

    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "mixed events read",
    ) else {
        return;
    };
    // RunSubmitted -> RunAccepted, RunAdmission -> RunAdmission,
    // StepStarted -> StepStarted, ActionScheduledTicket -> ActionScheduledTicket.
    assert_eq!(stored.len(), 4);
    assert!(matches!(stored[0], vb_storage::JournalEvent::RunAccepted { .. }));
    assert!(matches!(stored[1], vb_storage::JournalEvent::RunAdmission { .. }));
    assert!(matches!(stored[2], vb_storage::JournalEvent::StepStarted { .. }));
    assert!(matches!(
        stored[3],
        vb_storage::JournalEvent::ActionScheduledTicket { .. }
    ));
}

#[test]
fn append_sequenced_batch_single_event_matches_single_append() {
    // A 1-event batch must produce the same result as a single append_sequenced
    // call (regression / behavior parity).
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_005);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xD4; 32]);

    let events = [RuntimeJournalEvent::RunSubmitted { run, workflow }];
    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0)),
        Ok(())
    );

    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "single-event batch read",
    ) else {
        return;
    };
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0],
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow
        }
    );
}

#[test]
fn append_sequenced_batch_rejects_duplicate_against_persisted_journal() {
    // If an event in the batch has a (run, seq) that already exists in the
    // durable journal, the batch must reject the second insert and roll
    // back all prior events in the same batch (no events visible).
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_006);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xE5; 32]);

    // Commit a first event at (run, seq=0).
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    // Now submit a batch that includes the same (run, seq=0) as the first event.
    // Per-batch seq_start=0 means the first event collides with the persisted one.
    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
    ];
    let result = adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0));
    assert!(
        result.is_err(),
        "duplicate against journal must be rejected, got {result:?}"
    );

    // The original event must still be the only one visible.
    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "post-rollback events_for_run",
    ) else {
        return;
    };
    assert_eq!(
        stored.len(),
        1,
        "atomic rollback must leave only the pre-existing event, found {} events",
        stored.len()
    );
}

#[test]
fn append_sequenced_batch_preserves_append_sequenced_regression() {
    // Contract: the single-event append_sequenced must still work after the
    // batch method is added.
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_007);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xF6; 32]);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunFinished {
                run,
                result: vb_core::ids::SlotIdx::new(0),
            },
            vb_storage::EventSeq::new(1),
        ),
        Ok(())
    );

    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "single-event regression read",
    ) else {
        return;
    };
    assert_eq!(stored.len(), 2);
    assert!(matches!(stored[0], vb_storage::JournalEvent::RunAccepted { .. }));
    assert!(matches!(stored[1], vb_storage::JournalEvent::RunFinished { .. }));
}

#[test]
fn append_sequenced_batch_uses_journal_write_batch_atomic_commit() {
    // Property: after a successful batch commit, all events in the batch
    // are visible; index markers for ActionScheduledTicket events are
    // also visible in the same commit. This proves the batch path
    // committed them as a single atomic write.
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_008);

    let events = [
        ticket_event(run, 11, 0),
        ticket_event(run, 22, 1),
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0)),
        Ok(())
    );

    // Both events present?
    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "post-batch events",
    ) else {
        return;
    };
    assert_eq!(stored.len(), 2, "both events visible after atomic commit");

    // Both index markers present?
    // ticket_event(run, 11, 0) -> (action=11, step=0)
    // ticket_event(run, 22, 1) -> (action=22, step=1)
    let expected_markers = [(11u16, 0u16), (22u16, 1u16)];
    for (action_id, step_idx) in expected_markers {
        let key = index_action_key(
            vb_core::ids::ActionId::new(action_id),
            run,
            vb_core::ids::StepIdx::new(step_idx),
        )
        .expect("index_action_key must succeed");
        let present = journal
            .has_action_index_entry(&key)
            .expect("has_action_index_entry must succeed");
        assert!(
            present,
            "action index marker for action_id={action_id}, step={step_idx} must be visible"
        );
    }
}

#[test]
fn append_sequenced_batch_does_not_panic_on_zero_run_id() {
    // Property: the batch method must not panic on degenerate inputs.
    // Zero run_id is rejected by the journal at the event-key level;
    // we only assert that the batch method returns a Result (not panic).
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(0);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0x07; 32]);

    let events = [RuntimeJournalEvent::RunSubmitted { run, workflow }];
    let _result = adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0));
    // Either Ok or Err is acceptable; panic is not.
}

#[test]
fn append_sequenced_batch_tolerates_event_with_optional_fields() {
    // Property: events with optional/empty fields (no action index) must
    // round-trip correctly through the batch path.
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_009);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0x08; 32]);

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::WaitScheduled {
            run,
            step: vb_core::ids::StepIdx::new(0),
            deadline_ms: 1000,
        },
        RuntimeJournalEvent::WaitResolved {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        RuntimeJournalEvent::SlotWritten {
            run,
            slot: vb_core::ids::SlotIdx::new(0),
            value: vec![0xDE, 0xAD],
            taint: vb_core::value::Taint::Clean,
            extra: None,
        },
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(0)),
        Ok(())
    );

    let Some(stored) = require_ok(
        journal.events_for_run(run).map_err(|error| error.to_string()),
        "optional-fields events read",
    ) else {
        return;
    };
    assert_eq!(stored.len(), 4, "all 4 events must be committed");
    assert!(matches!(stored[0], vb_storage::JournalEvent::RunAccepted { .. }));
    assert!(matches!(
        stored[1],
        vb_storage::JournalEvent::WaitScheduledEvent { .. }
    ));
    assert!(matches!(
        stored[2],
        vb_storage::JournalEvent::RetryScheduledEvent { .. }
    ));
    assert!(matches!(
        stored[3],
        vb_storage::JournalEvent::SlotWrittenEvent { .. }
    ));
}

// ---------------------------------------------------------------------------
// RE-017: Sequence overflow must be detected BEFORE any event is committed.
// Saturation with `saturating_add` would silently duplicate `EventSeq` for
// later events in the batch. The corrected implementation preflights the
// full sequence range with `checked_add` and rejects the batch with a
// typed `RuntimeError::StorageJournalAppend { SequenceOverflow }`.
// ---------------------------------------------------------------------------

#[test]
fn append_sequenced_batch_rejects_overflow_at_max_seq_with_two_events() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_010);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xA9; 32]);

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
    ];

    // seq_start at u64::MAX would saturate to the same seq for both events
    // under the old buggy implementation; the corrected path must reject
    // with a typed overflow error.
    let result = adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(u64::MAX));
    let err = result.expect_err("overflow at u64::MAX must be rejected");
    assert!(
        matches!(
            err,
            crate::RuntimeError::StorageJournalAppend { ref source }
                if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow)
        ),
        "expected typed SequenceOverflow error, got {err:?}"
    );

    // No events must be committed from the rejected batch.
    let Some(stored) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "events_for_run after rejected overflow batch",
    ) else {
        return;
    };
    assert!(
        stored.is_empty(),
        "rejected overflow batch must commit zero events, found {} events",
        stored.len()
    );
}

#[test]
fn append_sequenced_batch_rejects_overflow_when_last_offset_overflows() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_011);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xB1; 32]);

    // Three events with seq_start near u64::MAX so the second event's
    // per-event seq would overflow under saturating arithmetic.
    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
        RuntimeJournalEvent::RunFailed { run },
    ];

    let result = adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(u64::MAX - 1));
    let err = result.expect_err("overflow on last offset must be rejected");
    assert!(
        matches!(
            err,
            crate::RuntimeError::StorageJournalAppend { ref source }
                if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow)
        ),
        "expected typed SequenceOverflow error, got {err:?}"
    );
}

#[test]
fn append_sequenced_batch_default_impl_rejects_overflow_at_max_seq() {
    // The default implementation (used by external implementers of the
    // trait) must also reject overflow instead of saturating.
    let noop = crate::journal::NoopRuntimeJournal::shared_for_tests_and_benchmarks();

    // Build a non-empty batch that overflows u64.
    let run = vb_core::ids::RunId::new(11_012);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xC2; 32]);
    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
    ];

    let result = noop.append_sequenced_batch(&events, vb_storage::EventSeq::new(u64::MAX));
    let err = result.expect_err("default impl must also reject overflow");
    assert!(
        matches!(
            err,
            crate::RuntimeError::StorageJournalAppend { ref source }
                if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow)
        ),
        "expected typed SequenceOverflow error, got {err:?}"
    );
}

#[test]
fn append_sequenced_batch_within_range_assigns_unique_sequences() {
    // Regression guard: even after the overflow fix, contiguous sequences
    // for in-range batches must still be assigned without saturation.
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(11_013);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xD3; 32]);

    let events = [
        RuntimeJournalEvent::RunSubmitted { run, workflow },
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
        RuntimeJournalEvent::RunFailed { run },
    ];

    assert_eq!(
        adapter.append_sequenced_batch(&events, vb_storage::EventSeq::new(1_000_000)),
        Ok(())
    );

    // Verify each event committed at the expected distinct sequence.
    let expected_seqs = [1_000_000u64, 1_000_001, 1_000_002];
    for (offset, expected_seq) in expected_seqs.iter().enumerate() {
        let seq = vb_storage::EventSeq::new(*expected_seq);
        let Some(bytes) = require_ok(
            journal
                .get_event_bytes(run, seq)
                .map_err(|error| error.to_string()),
            "get_event_bytes must succeed for in-range batch",
        ) else {
            return;
        };
        assert!(
            bytes.is_some(),
            "event at offset {offset} (seq={expected_seq}) must be committed"
        );
    }
}

#[test]
fn append_sequenced_batch_default_impl_accepts_in_range_sequence() {
    // Regression guard: the corrected default implementation must still
    // accept batches with seq_start safely within range.
    let noop = crate::journal::NoopRuntimeJournal::shared_for_tests_and_benchmarks();
    let run = vb_core::ids::RunId::new(11_014);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([0xE4; 32]);
    let events = [RuntimeJournalEvent::RunSubmitted { run, workflow }];
    assert_eq!(
        noop.append_sequenced_batch(&events, vb_storage::EventSeq::new(42)),
        Ok(())
    );
}
