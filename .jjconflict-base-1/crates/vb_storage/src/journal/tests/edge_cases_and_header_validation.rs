// =========================================================================
// Missing key lookups return correct error / None
// =========================================================================

#[test]
fn compiled_ir_returns_none_for_unwritten_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);
    let result = journal.compiled_ir(missing).expect("lookup should succeed");
    assert_eq!(result, None, "missing compiled IR should return None");
}

#[test]
fn snapshot_returns_none_for_missing_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .snapshot(RunId::new(99999), EventSeq::new(0))
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing snapshot should return None");
}

#[test]
fn blob_returns_none_for_unwritten_digest() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .blob([0x99; DIGEST_BYTES])
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing blob should return None");
}

// =========================================================================
// compiled_ir with digest verification
// =========================================================================

#[test]
fn compiled_ir_roundtrip_large_payload() {
    let (_temp, journal) = temp_journal();
    let ir = vec![0x42u8; 65536];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: ir.clone(),
    };
    journal
        .put_compiled_ir(&record)
        .expect("put should succeed");
    let loaded = journal
        .compiled_ir(digest)
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded.ir, ir);
}

// =========================================================================
// persist_strict succeeds on idle journal
// =========================================================================

#[test]
fn persist_strict_succeeds_without_prior_writes() {
    let (_temp, journal) = temp_journal();
    journal
        .persist_strict()
        .expect("persist_strict on idle journal should succeed");
}

// =========================================================================
// Edge case: write and read single event
// =========================================================================

#[test]
fn write_and_read_single_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7777);
    let digest = WorkflowDigest::from_bytes([0xDD; DIGEST_BYTES]);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal
        .append_strict(&event)
        .expect("append should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1, "should have exactly one event");
    assert_eq!(replayed[0], event, "replayed event should match original");
}

// =========================================================================
// Edge case: write batch of 100 events, read all back in order
// =========================================================================

#[test]
fn write_batch_100_events_read_all_in_order() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(8888);
    let count: u64 = 100;

    let events: Vec<JournalEvent> = (0..count)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new((i % 256) as u16),
            attempt: 1,
        })
        .collect();

    journal
        .append_strict_batch(&events)
        .expect("batch append should succeed");

    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        replayed.len(),
        count as usize,
        "should have exactly 100 events"
    );

    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            i as u64,
            "event at index {} should have seq {}",
            i,
            i
        );
        assert_eq!(event.run_id(), run, "all events should belong to same run");
        assert_eq!(
            *event, events[i],
            "event at index {} should match original",
            i
        );
    }
}

// =========================================================================
// Edge case: read from empty journal
// =========================================================================

#[test]
fn read_from_empty_journal_returns_nothing() {
    let (_temp, journal) = temp_journal();

    // All read operations on an empty journal should return None or empty
    let events = journal
        .events_for_run(RunId::new(1))
        .expect("events_for_run on empty journal should succeed");
    assert_eq!(events.len(), 0, "empty journal should have no events");

    let source = journal
        .workflow_source(WorkflowDigest::from_bytes([0; DIGEST_BYTES]))
        .expect("workflow_source on empty journal should succeed");
    assert_eq!(
        source, None,
        "empty journal should return None for workflow source"
    );

    let ir = journal
        .compiled_ir(WorkflowDigest::from_bytes([0; DIGEST_BYTES]))
        .expect("compiled_ir on empty journal should succeed");
    assert_eq!(ir, None, "empty journal should return None for compiled IR");

    let header = journal
        .run_header(RunId::new(1))
        .expect("run_header on empty journal should succeed");
    assert_eq!(
        header, None,
        "empty journal should return None for run header"
    );

    let snap = journal
        .snapshot(RunId::new(1), EventSeq::new(0))
        .expect("snapshot on empty journal should succeed");
    assert_eq!(snap, None, "empty journal should return None for snapshot");

    let blob = journal
        .blob([0; DIGEST_BYTES])
        .expect("blob on empty journal should succeed");
    assert_eq!(blob, None, "empty journal should return None for blob");

    let headers = journal
        .run_headers()
        .expect("run_headers on empty journal should succeed");
    assert_eq!(headers.len(), 0, "empty journal should have no run headers");
}

// =========================================================================
// Edge case: header validation - corrupted magic in stored record
// =========================================================================

#[test]
fn header_validation_rejects_corrupted_magic_in_stored_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9001);
    let event = make_event(run, 0);
    journal
        .append_strict(&event)
        .expect("append should succeed");

    // Directly corrupt the magic bytes in the underlying keyspace
    let key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("key construction should succeed");
    let mut value = journal
        .events
        .get(key.as_slice())
        .expect("fjall get should succeed")
        .expect("event should exist")
        .to_vec();

    // Overwrite the first 4 bytes (magic) with garbage
    let garbage_magic: u32 = 0xDEAD_BEEF;
    let magic_bytes = garbage_magic.to_le_bytes();
    if let Some(slice) = value.get_mut(0..4) {
        slice.copy_from_slice(&magic_bytes);
    }
    // Recompute CRC so the header passes CRC check but has wrong magic
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    // Write back corrupted value
    journal
        .events
        .insert(key.to_vec(), value)
        .expect("insert should succeed");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "corrupted magic in stored event must yield BadMagic, got {:?}",
        result
    );
}

#[test]
fn events_for_run_rejects_header_payload_kind_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp.path(), None)?;
    let run = RunId::new(9004);
    let event = JournalEvent::AskTimedOutEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let value = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let key = crate::keys::run_event_key(run, EventSeq::new(0))?;
    journal.events.insert(key.to_vec(), value)?;

    match journal.events_for_run(run) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskAnswered.id()
            && payload_kind == crate::RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        Ok(_) | Err(_) => Err(Box::new(JournalError::InvalidEvent)),
    }
}

// =========================================================================
// Adversarial replay parity: AskTimedOut(29) ↔ AskAnswered(18) swap.
// =========================================================================

/// The replay path (`events_for_run`) must reject a stored record whose
/// envelope kind is `AskTimedOut(29)` while the payload is an
/// `AskAnsweredEvent`. Without parity enforcement, a tampered record would
/// reach the recovery code as an "answer" instead of a "timeout" and
/// silently diverge from the runtime state. The diagnostic must name both
/// the envelope and the payload kind.
#[test]
fn events_for_run_rejects_ask_timed_out_envelope_with_ask_answered_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp.path(), None)?;
    let run = RunId::new(9101);
    let event = JournalEvent::AskAnsweredEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let value = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let key = crate::keys::run_event_key(run, EventSeq::new(0))?;
    journal.events.insert(key.to_vec(), value)?;

    match journal.events_for_run(run) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskTimedOut.id()
            && payload_kind == crate::RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "events_for_run must reject AskTimedOut(29) envelope carrying AskAnswered payload, got {other:?}"
            );
            Err(Box::new(JournalError::InvalidEvent))
        }
    }
}

/// Symmetric inverse: a stored record whose envelope kind is
/// `AskAnswered(18)` while the payload is an `AskTimedOutEvent` must also be
/// rejected by the replay path. This is the parity direction exercised by
/// the existing `events_for_run_rejects_header_payload_kind_mismatch` test,
/// but is repeated here under the explicit AskTimedOut ↔ AskAnswered
/// contract so the storage journal parity/API contract is unambiguously
/// covered for both swap directions through every public path.
#[test]
fn events_for_run_rejects_ask_answered_envelope_with_ask_timed_out_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp.path(), None)?;
    let run = RunId::new(9102);
    let event = JournalEvent::AskTimedOutEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let value = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let key = crate::keys::run_event_key(run, EventSeq::new(0))?;
    journal.events.insert(key.to_vec(), value)?;

    match journal.events_for_run(run) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskAnswered.id()
            && payload_kind == crate::RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "events_for_run must reject AskAnswered(18) envelope carrying AskTimedOut payload, got {other:?}"
            );
            Err(Box::new(JournalError::InvalidEvent))
        }
    }
}

// =========================================================================
// Edge case: header validation - wrong schema version in stored record
// =========================================================================

#[test]
fn header_validation_rejects_wrong_schema_version_in_stored_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9002);
    let event = make_event(run, 0);
    journal
        .append_strict(&event)
        .expect("append should succeed");

    let key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("key construction should succeed");
    let mut value = journal
        .events
        .get(key.as_slice())
        .expect("fjall get should succeed")
        .expect("event should exist")
        .to_vec();

    // Overwrite schema version (offset 4, u16 LE) with a future version
    let future_version: u16 = 99;
    let version_bytes = future_version.to_le_bytes();
    if let Some(slice) = value.get_mut(4..6) {
        slice.copy_from_slice(&version_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    journal
        .events
        .insert(key.to_vec(), value)
        .expect("insert should succeed");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
        "wrong schema version in stored event must yield UnsupportedSchemaVersion, got {:?}",
        result
    );
}

// =========================================================================
// Edge case: snapshot round-trip (write snapshot, read back)
// =========================================================================

#[test]
fn snapshot_roundtrip_preserves_all_fields() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7777);
    let workflow = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
    let slots = vec![0x01_u8, 0x02, 0x03, 0x04];
    let taint = vec![0xFF_u8, 0xFE, 0xFD];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(42),
        workflow,
        slots: slots.clone(),
        taint: taint.clone(),
    };

    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(42))
        .expect("snapshot get should succeed")
        .expect("snapshot should exist");

    assert_eq!(loaded.run, run, "run must match");
    assert_eq!(loaded.seq, EventSeq::new(42), "seq must match");
    assert_eq!(loaded.workflow, workflow, "workflow must match");
    assert_eq!(loaded.slots, slots, "slots must match byte-for-byte");
    assert_eq!(loaded.taint, taint, "taint must match byte-for-byte");
}

#[test]
fn snapshot_roundtrip_with_empty_slots_and_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7778);
    let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: vec![],
        taint: vec![],
    };

    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(0))
        .expect("snapshot get should succeed")
        .expect("snapshot should exist");

    assert_eq!(loaded.slots.len(), 0, "slots should be empty");
    assert_eq!(loaded.taint.len(), 0, "taint should be empty");
    assert_eq!(loaded, snapshot, "full snapshot must round-trip");
}

// =========================================================================
// Edge case: source marker round-trip (workflow source)
// =========================================================================

#[test]
fn source_marker_roundtrip_preserves_content_and_digest() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow:\n  name: edge_test\n  steps: []".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    journal
        .put_workflow_source(&record)
        .expect("put_workflow_source should succeed");
    let loaded = journal
        .workflow_source(digest)
        .expect("workflow_source get should succeed")
        .expect("workflow source should exist");

    assert_eq!(
        loaded.source, source,
        "source bytes must round-trip exactly"
    );
    assert_eq!(loaded.digest, digest, "digest must round-trip exactly");
}

#[test]
fn source_marker_roundtrip_with_minimal_source() {
    let (_temp, journal) = temp_journal();
    // Minimal valid source: empty bytes
    let source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    journal
        .put_workflow_source(&record)
        .expect("put_workflow_source should succeed for empty source");
    let loaded = journal
        .workflow_source(digest)
        .expect("workflow_source get should succeed")
        .expect("workflow source should exist");

    assert_eq!(loaded.source.len(), 0, "empty source should round-trip");
    assert_eq!(loaded.source, source);
}

// =========================================================================
// Edge case: multiple persist_strict calls in sequence
// =========================================================================

#[test]
fn multiple_persist_strict_calls_succeed() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9900);
    let event = make_event(run, 0);
    journal.append_strict(&event).expect("first append");
    journal.persist_strict().expect("first persist");
    journal.persist_strict().expect("second persist");
    journal.persist_strict().expect("third persist");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1);
}

/// Pins the strict-fsync boundary (VB-STORAGE-FJALL-001): unfsynced appends
/// are visible to same-handle readers, and after the explicit persist_strict
/// barrier they survive a drop without close() plus a reopen, in order.
/// Fjall's own fsync correctness stays an externally assumed dependency.
#[test]
fn fjall_fsync_boundary() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(4242);
    {
        let journal = FjallJournal::open(&path, None).expect("journal open should succeed");
        let first = make_event(run, 0);
        let second = make_event(run, 1);
        journal
            .append_journaled(&first)
            .expect("unfsynced append should succeed");
        journal
            .append_journaled(&second)
            .expect("unfsynced append should succeed");
        let visible = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            visible.len(),
            2,
            "unfsynced appends must be visible pre-barrier"
        );
        let strict = make_event(run, 2);
        journal
            .append_strict(&strict)
            .expect("strict append should succeed");
        journal
            .persist_strict()
            .expect("explicit barrier should succeed");
    }
    let reopened = FjallJournal::open(&path, None).expect("journal should reopen after drop");
    let replayed = reopened.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        replayed.len(),
        3,
        "all three events must survive the barrier plus reopen"
    );
    assert_eq!(replayed[0].seq(), EventSeq::new(0));
    assert_eq!(replayed[1].seq(), EventSeq::new(1));
    assert_eq!(replayed[2].seq(), EventSeq::new(2));
}

// =========================================================================
// Edge case: interleaved writes to different runs
// =========================================================================

#[test]
fn interleaved_writes_to_different_runs_preserves_ordering() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(10001);
    let run_b = RunId::new(10002);
    let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let replayed_a = journal.events_for_run(run_a).expect("replay A");
    let replayed_b = journal.events_for_run(run_b).expect("replay B");

    assert_eq!(replayed_a.len(), 3, "run A should have 3 events");
    assert_eq!(replayed_b.len(), 2, "run B should have 2 events");
    assert_eq!(replayed_a[0].seq().get(), 0);
    assert_eq!(replayed_a[1].seq().get(), 1);
    assert_eq!(replayed_a[2].seq().get(), 2);
    assert_eq!(replayed_b[0].seq().get(), 0);
    assert_eq!(replayed_b[1].seq().get(), 1);
}

// =========================================================================
// Edge case: batch with single element
// =========================================================================

#[test]
fn append_strict_batch_single_element_roundtrips() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10100);
    let event = make_event(run, 0);
    journal
        .append_strict_batch(&[event.clone()])
        .expect("single-element batch");
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

// =========================================================================
// Edge case: duplicate detection after batch commit
// =========================================================================

#[test]
fn duplicate_event_after_batch_commit_is_rejected() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10200);
    let event = make_event(run, 0);
    journal
        .append_strict_batch(&[event.clone()])
        .expect("batch commit");
    let result = journal.append_strict(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate after batch should be rejected, got {:?}",
        result
    );
}

