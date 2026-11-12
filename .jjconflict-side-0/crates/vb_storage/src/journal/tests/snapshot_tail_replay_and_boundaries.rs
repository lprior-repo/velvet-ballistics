// =========================================================================
// Edge case: snapshot with post-snapshot events starts replay after snapshot
// =========================================================================

#[test]
fn events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10300);
    let workflow = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);

    // Write a snapshot at seq 2
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("put snapshot");

    // Only write events at seq 3, 4, 5 (as if snapshot-covered events were trimmed)
    for seq in 3u64..6 {
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(seq as u16),
            attempt: 1,
        };
        journal
            .append_unfsynced(&event)
            .expect("append should succeed");
    }

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(
        replayed.len(),
        3,
        "should replay 3 events starting after snapshot seq"
    );
    assert_eq!(replayed[0].seq().get(), 3);
    assert_eq!(replayed[1].seq().get(), 4);
    assert_eq!(replayed[2].seq().get(), 5);
}

#[test]
fn events_for_run_bounded_rejects_over_limit() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10301);
    for seq in 0u16..2 {
        journal
            .append_unfsynced(&make_step_started(run, u64::from(seq), seq))
            .map_err(|err| err.to_string())?;
    }

    let Some(limit) = EventReplayLimit::new(1) else {
        return Err("non-zero replay limit was rejected".to_owned());
    };
    let result = journal.events_for_run_bounded(run, limit);
    assert!(
        matches!(
            result,
            Err(JournalError::TooManyEvents {
                run: found_run,
                limit: 1,
                observed: 2,
            }) if found_run == run
        ),
        "bounded replay must fail closed when more events are present"
    );
    Ok(())
}

#[test]
fn events_for_run_detects_missing_first_tail_event_after_snapshot() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10302);
    let workflow = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unfsynced(&make_step_started(run, 4, 4))
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap {
                expected,
                actual,
            }) if expected == EventSeq::new(3) && actual == EventSeq::new(4)
        ),
        "missing first event after the durable snapshot must not be laundered"
    );
    Ok(())
}

#[test]
fn events_for_run_without_snapshot_rejects_missing_initial_sequence() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10303);
    journal
        .append_unfsynced(&make_step_started(run, 4, 4))
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap { expected, actual })
                if expected == EventSeq::new(0) && actual == EventSeq::new(4)
        ),
        "runs without snapshots must start at seq 0, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_corrupt_latest_snapshot_before_skipping_events() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10304);
    let workflow = WorkflowDigest::from_bytes([0x45; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unfsynced(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let mut value = journal
        .run_snapshot
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "snapshot record should exist".to_owned())?
        .to_vec();
    corrupt_magic_preserving_crc(&mut value);
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "corrupt latest snapshot must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay()
-> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10306);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: WorkflowDigest::from_bytes([0x47; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unfsynced(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let mut value = journal
        .run_snapshot
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "snapshot record should exist".to_owned())?
        .to_vec();
    let last = value
        .last_mut()
        .ok_or_else(|| "snapshot record must be non-empty".to_owned())?;
    *last ^= 0x01;
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupt latest snapshot payload digest must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_latest_snapshot_postcard_decode_failure_before_tail_replay()
-> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10307);
    journal
        .append_unfsynced(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let invalid_payload = vec![0xFF_u8];
    let value = crate::codec::encode_record(
        MAGIC_SNAPSHOT,
        crate::records::RecordKind::Snapshot,
        2,
        &invalid_payload,
        MAX_SNAPSHOT_BYTES,
    )
    .map_err(|err| err.to_string())?;
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::PostcardDecodeFailed(_))),
        "invalid latest snapshot payload must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_skips_corrupt_pre_snapshot_event_by_key_range() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10305);
    for seq in 0_u64..3 {
        journal
            .append_unfsynced(&make_step_started(run, seq, seq as u16))
            .map_err(|err| err.to_string())?;
    }
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([0x46; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;

    let key = crate::keys::run_event_key(run, EventSeq::new(0)).map_err(|err| err.to_string())?;
    let mut value = journal
        .events
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "event record should exist".to_owned())?
        .to_vec();
    corrupt_magic_preserving_crc(&mut value);
    journal
        .events
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let replayed = journal.events_for_run(run).map_err(|err| err.to_string())?;
    assert_eq!(replayed, vec![make_step_started(run, 2, 2)]);
    Ok(())
}

// =========================================================================
// Edge case: compiled_ir stores multiple distinct digests
// =========================================================================

#[test]
fn compiled_ir_stores_multiple_distinct_digests() {
    let (_temp, journal) = temp_journal();
    let ir_v1 = b"compiled-v1".to_vec();
    let digest_v1 = WorkflowDigest::from_bytes(blake3::hash(&ir_v1).into());
    let record_v1 = CompiledIrRecord {
        digest: digest_v1,
        ir: ir_v1.clone(),
    };
    journal.put_compiled_ir(&record_v1).expect("put v1");

    let ir_v2 = b"compiled-v2".to_vec();
    let digest_v2 = WorkflowDigest::from_bytes(blake3::hash(&ir_v2).into());
    let record_v2 = CompiledIrRecord {
        digest: digest_v2,
        ir: ir_v2.clone(),
    };
    journal.put_compiled_ir(&record_v2).expect("put v2");

    let loaded_v1 = journal
        .compiled_ir(digest_v1)
        .expect("get v1")
        .expect("present");
    let loaded_v2 = journal
        .compiled_ir(digest_v2)
        .expect("get v2")
        .expect("present");
    assert_eq!(loaded_v1.ir, ir_v1);
    assert_eq!(loaded_v2.ir, ir_v2);
}

// =========================================================================
// Edge case: many distinct runs with one event each
// =========================================================================

#[test]
fn many_runs_one_event_each_are_isolated() {
    let (_temp, journal) = temp_journal();
    let count: u64 = 20;
    for i in 0u64..count {
        let run = RunId::new(11000_u64.saturating_add(i));
        let event = make_event(run, 0);
        journal
            .append_unfsynced(&event)
            .expect("append should succeed");
    }

    for i in 0u64..count {
        let run = RunId::new(11000_u64.saturating_add(i));
        let replayed = journal.events_for_run(run).expect("replay");
        assert_eq!(replayed.len(), 1, "run {} should have 1 event", i);
        assert_eq!(replayed[0].run_id(), run);
    }
}

// =========================================================================
// Edge case: large batch write of 500 events
// =========================================================================

#[test]
fn large_batch_500_events_roundtrips() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(12000);
    let count: u64 = 500;

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
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), count as usize);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.seq().get(), i as u64);
    }
}

// =========================================================================
// Edge case: boundary sequence numbers (0 and 1)
// =========================================================================

#[test]
fn boundary_sequence_numbers_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(13000);
    let digest = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal.append_unfsynced(&e0).expect("append seq 0");

    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal.append_unfsynced(&e1).expect("append seq 1");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].seq().get(), 0);
    assert_eq!(replayed[1].seq().get(), 1);
}

// =========================================================================
// Edge case: blob with 16 KiB payload roundtrips
// =========================================================================

#[test]
fn blob_large_payload_roundtrips() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xFE_u8; 16384];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };
    journal.put_blob(&record).expect("put should succeed");
    let loaded = journal
        .blob(digest)
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded.bytes, payload);
}

// =========================================================================
// Edge case: batch append_event rejects duplicate key within same batch
// =========================================================================
//
// SA-003: prior behavior allowed duplicate `(run, seq)` insertions
// within a single `JournalWriteBatch`, relying on Fjall's
// last-write-wins semantics to collapse them at commit time. That
// silent overwrite dropped the first event's value without
// signaling an error. The fixed behavior rejects the second
// append with `DuplicateStagedKey` so callers can detect the
// intra-batch collision explicitly.

#[test]
fn batch_append_event_rejects_intra_batch_duplicate() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(14000);
    let event = make_event(run, 0);

    let mut batch = journal.batch();
    batch.append_event(&event).expect("first batch append");

    let second = batch.append_event(&event);
    assert!(
        matches!(second, Err(JournalError::DuplicateStagedKey { .. })),
        "second same-batch append must reject with DuplicateStagedKey, got {:?}",
        second
    );

    // The rejection must not mutate the prior staged event: the
    // batch still contains exactly one event and commits cleanly.
    assert_eq!(
        batch.len(),
        1,
        "rejected append must not stage a second event"
    );
    batch.commit().expect("commit should succeed");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(
        replayed.len(),
        1,
        "duplicate must be rejected, leaving a single durable event"
    );
}

// =========================================================================
// Edge case: batch with snapshot and event for same run
// =========================================================================

#[test]
fn batch_with_snapshot_and_event_for_same_run() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(15000);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: vec![1, 2, 3],
        taint: vec![],
    };
    let event = make_event(run, 1);

    let mut batch = journal.batch();
    batch.put_snapshot(&snapshot).expect("batch snapshot");
    batch.append_event(&event).expect("batch event");
    batch.commit().expect("commit");

    let loaded_snap = journal
        .snapshot(run, EventSeq::new(0))
        .expect("get snapshot")
        .expect("present");
    assert_eq!(loaded_snap.slots, vec![1, 2, 3]);
    // Snapshot at seq 0 covers events 0..0; event at seq 1 is after snapshot
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
}

// =========================================================================
// Edge case: workflow source with 128 KiB payload
// =========================================================================

#[test]
fn workflow_source_near_max_bytes_is_accepted() {
    let (_temp, journal) = temp_journal();
    let source = vec![0x42u8; 131072];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };
    journal
        .put_workflow_source(&record)
        .expect("put should succeed for 128 KiB");
    let loaded = journal
        .workflow_source(digest)
        .expect("get")
        .expect("present");
    assert_eq!(loaded.source.len(), source.len());
}

// =========================================================================
// Edge case: run header with max field values
// =========================================================================

#[test]
fn run_header_with_max_field_values() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(u64::MAX);
    let digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(u32::MAX),
        compiled_digest: digest,
        status: u8::MAX,
        accepted_at_ms: u64::MAX,
    };
    journal.put_run_header(&record).expect("put should succeed");
    let loaded = journal.run_header(run).expect("get").expect("present");
    assert_eq!(loaded.run, run);
    assert_eq!(loaded.workflow_id, vb_core::WorkflowId::new(u32::MAX));
    assert_eq!(loaded.status, u8::MAX);
    assert_eq!(loaded.accepted_at_ms, u64::MAX);
}

// =========================================================================
// Edge case: empty run headers list
// =========================================================================

#[test]
fn run_headers_empty_journal_returns_empty_vec() {
    let (_temp, journal) = temp_journal();
    let headers = journal.run_headers().expect("run_headers on empty journal");
    assert_eq!(headers.len(), 0, "empty journal should have no headers");
}

// =========================================================================
// Edge case: snapshot with populated slots and empty taint
// =========================================================================

#[test]
fn snapshot_with_populated_slots_empty_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(16000);
    let workflow = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);
    let slots = vec![0x01_u8; 256];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow,
        slots: slots.clone(),
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("put");
    let loaded = journal
        .snapshot(run, EventSeq::new(10))
        .expect("get")
        .expect("present");
    assert_eq!(loaded.slots, slots);
    assert!(loaded.taint.is_empty());
}

// =========================================================================
// Edge case: snapshot with empty slots and populated taint
// =========================================================================

#[test]
fn snapshot_with_empty_slots_populated_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(16001);
    let workflow = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
    let taint = vec![0xFF_u8; 128];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow,
        slots: vec![],
        taint: taint.clone(),
    };
    journal.put_snapshot(&snapshot).expect("put");
    let loaded = journal
        .snapshot(run, EventSeq::new(5))
        .expect("get")
        .expect("present");
    assert!(loaded.slots.is_empty());
    assert_eq!(loaded.taint, taint);
}

