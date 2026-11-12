// =========================================================================
// Keyspace isolation — different run IDs don't interfere
// =========================================================================

#[test]
fn events_for_run_isolates_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);

    // Write events for run A
    let events_a = [make_event(run_a, 0), make_step_started(run_a, 1, 0)];
    journal
        .append_strict_batch(&events_a)
        .expect("batch A should succeed");

    // Write events for run B
    let events_b = [
        make_event(run_b, 0),
        make_step_started(run_b, 1, 0),
        make_step_started(run_b, 2, 1),
    ];
    journal
        .append_strict_batch(&events_b)
        .expect("batch B should succeed");

    // Replay run A: should only get run A events
    let replayed_a = journal
        .events_for_run(run_a)
        .expect("replay A should succeed");
    assert_eq!(replayed_a.len(), 2, "run A should have exactly 2 events");
    for event in &replayed_a {
        assert_eq!(event.run_id(), run_a, "replayed event must belong to run A");
    }

    // Replay run B: should only get run B events
    let replayed_b = journal
        .events_for_run(run_b)
        .expect("replay B should succeed");
    assert_eq!(replayed_b.len(), 3, "run B should have exactly 3 events");
    for event in &replayed_b {
        assert_eq!(event.run_id(), run_b, "replayed event must belong to run B");
    }
}

#[test]
fn run_headers_isolate_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(10);
    let run_b = RunId::new(20);
    let digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

    journal
        .put_run_header(&RunHeaderRecord {
            run: run_a,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        })
        .expect("put A should succeed");

    journal
        .put_run_header(&RunHeaderRecord {
            run: run_b,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: 200,
        })
        .expect("put B should succeed");

    let header_a = journal
        .run_header(run_a)
        .expect("get A should succeed")
        .expect("A present");
    let header_b = journal
        .run_header(run_b)
        .expect("get B should succeed")
        .expect("B present");

    assert_eq!(header_a.workflow_id, WorkflowId::new(1));
    assert_eq!(header_a.status, 1);
    assert_eq!(header_b.workflow_id, WorkflowId::new(2));
    assert_eq!(header_b.status, 2);
}

#[test]
fn snapshots_isolate_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(50);
    let run_b = RunId::new(60);
    let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

    journal
        .put_snapshot(&RunSnapshot {
            run: run_a,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![0u8],
            taint: vec![],
        })
        .expect("put A should succeed");

    journal
        .put_snapshot(&RunSnapshot {
            run: run_b,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![1u8, 2u8, 3u8],
            taint: vec![0u8],
        })
        .expect("put B should succeed");

    let snap_a = journal
        .snapshot(run_a, EventSeq::new(1))
        .expect("get A")
        .expect("present");
    let snap_b = journal
        .snapshot(run_b, EventSeq::new(1))
        .expect("get B")
        .expect("present");

    assert_eq!(snap_a.run, run_a);
    assert_eq!(snap_a.slots.len(), 1);
    assert_eq!(snap_b.run, run_b);
    assert_eq!(snap_b.slots.len(), 3);
}

// =========================================================================
// Sequential event ordering
// =========================================================================

#[test]
fn events_for_run_returns_events_in_sequence_order() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);

    let events: Vec<JournalEvent> = (0..5)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new(i as u16),
            attempt: 1,
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 5);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            i as u64,
            "event at index {} should have seq {}",
            i,
            i
        );
    }
}

#[test]
fn events_for_run_rejects_sequence_gap() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);

    // Write seq 0 and seq 2 (gap at seq 1)
    let e0 = make_event(run, 0);
    let e2 = make_event(run, 2);
    journal.append_unfsynced(&e0).expect("append 0");
    journal.append_unfsynced(&e2).expect("append 2");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::SequenceGap { .. })),
        "sequence gap must be detected during replay, got {:?}",
        result
    );
}

#[test]
fn events_for_run_returns_empty_for_unknown_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .events_for_run(RunId::new(99999))
        .expect("replay should succeed");
    assert_eq!(result.len(), 0, "unknown run should have zero events");
}

// =========================================================================
// Duplicate event rejection
// =========================================================================

#[test]
fn append_strict_rejects_duplicate_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);
    let event = make_event(run, 0);

    journal
        .append_strict(&event)
        .expect("first append should succeed");
    let result = journal.append_strict(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate event must be rejected, got {:?}",
        result
    );
}

#[test]
fn append_journaled_succeeds_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(600);
    let event = make_event(run, 0);

    journal
        .append_journaled(&event)
        .expect("append_journaled should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

#[test]
fn append_journaled_rejects_invalid_event_without_persisting() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(601);
    let event = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(1),
        attempt: 0,
    };

    let result = journal.append_journaled(&event);

    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "invalid event must be rejected before storage, got {:?}",
        result
    );
    assert_eq!(
        journal
            .get_event_bytes(run, EventSeq::new(0))
            .expect("query"),
        None
    );
}

#[test]
fn append_strict_batch_rejects_invalid_event_without_partial_commit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(602);
    let events = [
        make_event(run, 0),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 0,
        },
    ];

    let result = journal.append_strict_batch(&events);

    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "invalid batch event must fail closed, got {:?}",
        result
    );
    assert_eq!(
        journal
            .get_event_bytes(run, EventSeq::new(0))
            .expect("query seq0"),
        None
    );
    assert_eq!(
        journal
            .get_event_bytes(run, EventSeq::new(1))
            .expect("query seq1"),
        None
    );
}

#[test]
fn append_strict_batch_writes_all_events() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let events = [
        make_event(run, 0),
        make_step_started(run, 1, 0),
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 3);
    assert_eq!(replayed[0], events[0]);
    assert_eq!(replayed[1], events[1]);
    assert_eq!(replayed[2], events[2]);
}

#[test]
fn append_strict_batch_on_empty_is_ok() {
    let (_temp, journal) = temp_journal();
    let result = journal.append_strict_batch(&[]);
    assert!(result.is_ok(), "empty batch should succeed");
}

// =========================================================================
// Declared keyspaces
// =========================================================================

#[test]
fn declared_keyspaces_count_matches_opened_keyspaces() {
    let declared = FjallJournal::declared_keyspaces();
    assert_eq!(declared.len(), 12, "there should be 12 declared keyspaces");
    let (_temp, _journal) = temp_journal();
    // If we got here, all keyspaces opened successfully
}

// =========================================================================
// Verify content digest
// =========================================================================

#[test]
fn verify_content_digest_accepts_valid() {
    let content = b"some bytes";
    let hash = blake3::hash(content);
    let result = verify_content_digest(content, hash.as_bytes());
    assert!(result.is_ok(), "valid content digest should pass");
}

#[test]
fn verify_content_digest_rejects_mismatch() {
    let content = b"some bytes";
    let wrong = blake3::hash(b"other bytes");
    let result = verify_content_digest(content, wrong.as_bytes());
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "mismatched digest should fail"
    );
}

// =========================================================================
// Multiple event types round-trip through journal
// =========================================================================

#[test]
fn all_event_variant_roundtrip_through_journal() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(800);
    let digest = WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: vb_core::ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            action: vb_core::ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), events.len());
    for (i, (original, replayed_event)) in events.iter().zip(replayed.iter()).enumerate() {
        assert_eq!(original, replayed_event, "event at index {} mismatch", i);
    }
}

