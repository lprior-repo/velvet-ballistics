// =========================================================================
// Write/read round-trip tests
// =========================================================================

#[test]
fn workflow_source_roundtrip() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow: hello_world".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };
    journal
        .put_workflow_source(&record)
        .expect("put should succeed");
    let loaded = journal.workflow_source(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("workflow source should be found");
    };
    assert_eq!(found.source, source);
    assert_eq!(found.digest, digest);
}

#[test]
fn workflow_source_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let result = journal
        .workflow_source(missing)
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing digest should return None");
}

#[test]
fn compiled_ir_roundtrip() {
    let (_temp, journal) = temp_journal();
    let ir = b"compiled-artifact-bytes".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: ir.clone(),
    };
    journal
        .put_compiled_ir(&record)
        .expect("put should succeed");
    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("compiled IR should be found");
    };
    assert_eq!(found.ir, ir);
}

#[test]
fn compiled_ir_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0x00; DIGEST_BYTES]);
    let result = journal.compiled_ir(missing).expect("lookup should succeed");
    assert_eq!(result, None, "missing digest should return None");
}

#[test]
fn run_header_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(12345);
    let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(7),
        compiled_digest: digest,
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    journal.put_run_header(&record).expect("put should succeed");
    let loaded = journal.run_header(run).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("run header should be found");
    };
    assert_eq!(found.run, run);
    assert_eq!(found.workflow_id, WorkflowId::new(7));
    assert_eq!(found.status, 2);
    assert_eq!(found.accepted_at_ms, 1700000000000);
}

#[test]
fn run_header_returns_none_for_missing_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .run_header(RunId::new(999))
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing run should return None");
}

#[test]
fn snapshot_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(55);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow,
        slots: vec![0u8, 1u8],
        taint: vec![0u8, 0u8],
    };
    journal.put_snapshot(&snapshot).expect("put should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(10))
        .expect("get should succeed");
    let Some(found) = loaded else {
        panic!("snapshot should be found");
    };
    assert_eq!(found.run, run);
    assert_eq!(found.seq, EventSeq::new(10));
    assert_eq!(found.slots.len(), 2);
    assert_eq!(found.workflow, workflow);
}

#[test]
fn blob_roundtrip() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };
    journal.put_blob(&record).expect("put should succeed");
    let loaded = journal.blob(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("blob should be found");
    };
    assert_eq!(found.bytes, payload);
}

#[test]
fn blob_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .blob([0; DIGEST_BYTES])
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing blob should return None");
}

#[test]
fn blob_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3];
    let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
    let record = BlobRecord {
        digest: wrong_digest,
        bytes: payload,
    };
    let result = journal.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "digest mismatch must be rejected, got {:?}",
        result
    );
}

#[test]
fn workflow_source_read_rejects_payload_digest_not_matching_requested_key() {
    let (_temp, journal) = temp_journal();
    let requested_source = b"requested workflow".to_vec();
    let requested_digest = WorkflowDigest::from_bytes(blake3::hash(&requested_source).into());
    let stored_source = b"stored under the wrong key".to_vec();
    let stored_digest = WorkflowDigest::from_bytes(blake3::hash(&stored_source).into());
    let record = WorkflowSourceRecord {
        digest: stored_digest,
        source: stored_source,
    };
    let key = crate::keys::workflow_source_key(requested_digest.as_bytes())
        .expect("workflow source key should encode");
    let value = crate::codec::encode_record(
        MAGIC_WORKFLOW_SOURCE,
        crate::RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )
    .expect("forged workflow source envelope should encode");
    journal
        .workflow_source
        .insert(key.to_vec(), value)
        .expect("raw corrupt workflow source insert should succeed");

    let result = journal.workflow_source(requested_digest);

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "workflow source read must rebind requested digest key, got {result:?}"
    );
}

#[test]
fn workflow_source_read_rejects_tampered_source_when_record_digest_matches_key() {
    let (_temp, journal) = temp_journal();
    let trusted_source = b"trusted workflow source".to_vec();
    let requested_digest = WorkflowDigest::from_bytes(blake3::hash(&trusted_source).into());
    let record = WorkflowSourceRecord {
        digest: requested_digest,
        source: b"tampered workflow source".to_vec(),
    };
    let key = crate::keys::workflow_source_key(requested_digest.as_bytes())
        .expect("workflow source key should encode");
    let value = crate::codec::encode_record(
        MAGIC_WORKFLOW_SOURCE,
        crate::RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )
    .expect("forged workflow source envelope should encode");
    journal
        .workflow_source
        .insert(key.to_vec(), value)
        .expect("raw tampered workflow source insert should succeed");

    let result = journal.workflow_source(requested_digest);

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "workflow source read must hash source bytes even when record digest matches key, got {result:?}"
    );
}

#[test]
fn compiled_ir_read_rejects_payload_content_not_matching_digest() {
    let (_temp, journal) = temp_journal();
    let trusted_ir = b"trusted compiled ir".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&trusted_ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: b"tampered compiled ir".to_vec(),
    };
    let key = crate::keys::compiled_ir_key(digest.as_bytes()).expect("compiled ir key encodes");
    let value = crate::codec::encode_record(
        MAGIC_COMPILED_ARTIFACT,
        crate::RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("forged compiled ir envelope should encode");
    journal
        .compiled_ir
        .insert(key.to_vec(), value)
        .expect("raw corrupt compiled ir insert should succeed");

    let result = journal.compiled_ir(digest);

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "compiled IR read must verify decoded content digest, got {result:?}"
    );
}

#[test]
fn compiled_ir_for_source_digest_rejects_scanned_key_record_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let source_digest = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);
    let record = accepted_artifact_record_for_source(source_digest)
        .expect("accepted artifact should encode");
    let wrong_key_digest = WorkflowDigest::from_bytes([0x99; DIGEST_BYTES]);
    let key = crate::keys::compiled_ir_key(wrong_key_digest.as_bytes())
        .expect("compiled ir key should encode");
    let value = crate::codec::encode_record(
        MAGIC_COMPILED_ARTIFACT,
        crate::RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("forged compiled ir envelope should encode");
    journal
        .compiled_ir
        .insert(key.to_vec(), value)
        .expect("raw compiled ir insert should succeed");

    let result = journal.compiled_ir_for_source_digest(source_digest);

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "compiled_ir_for_source_digest must validate scanned key↔record digest before source match, got {result:?}"
    );
}

#[test]
fn blob_read_rejects_payload_content_not_matching_digest() {
    let (_temp, journal) = temp_journal();
    let trusted_bytes = b"trusted blob".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&trusted_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: b"tampered blob".to_vec(),
    };
    let key = crate::keys::blob_key(digest).expect("blob key encodes");
    let value = crate::codec::encode_record(
        MAGIC_BLOB,
        crate::RecordKind::Blob,
        0,
        &record,
        MAX_BLOB_BYTES,
    )
    .expect("forged blob envelope should encode");
    journal
        .blob
        .insert(key.to_vec(), value)
        .expect("raw corrupt blob insert should succeed");

    let result = journal.blob(digest);

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "blob read must verify decoded content digest, got {result:?}"
    );
}

#[test]
fn snapshot_read_rejects_payload_sequence_not_matching_requested_key() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(77);
    let requested_seq = EventSeq::new(3);
    let stored_seq = EventSeq::new(4);
    let snapshot = RunSnapshot {
        run,
        seq: stored_seq,
        workflow: WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]),
        slots: vec![1, 2, 3],
        taint: vec![0],
    };
    let key = crate::keys::run_snapshot_key(run, requested_seq).expect("snapshot key encodes");
    let value = crate::codec::encode_record(
        MAGIC_SNAPSHOT,
        crate::RecordKind::Snapshot,
        stored_seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("forged snapshot envelope should encode");
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .expect("raw corrupt snapshot insert should succeed");

    let result = journal.snapshot(run, requested_seq);

    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap {
                expected,
                actual
            }) if expected == requested_seq && actual == stored_seq
        ),
        "snapshot read must rebind requested (run, seq) key, got {result:?}"
    );
}

#[test]
fn snapshot_read_rejects_envelope_only_sequence_mismatch_with_exact_fields() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(78);
    let seq = EventSeq::new(5);
    let envelope_seq = 9005;
    let snapshot = RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0x78; DIGEST_BYTES]),
        slots: vec![1, 2, 3],
        taint: vec![0],
    };
    let key = crate::keys::run_snapshot_key(run, seq).expect("snapshot key encodes");
    let value = crate::codec::encode_record(
        MAGIC_SNAPSHOT,
        crate::RecordKind::Snapshot,
        envelope_seq,
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("forged snapshot envelope should encode");
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .expect("raw snapshot insert should succeed");

    let result = journal.snapshot(run, seq);

    match result {
        Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run: got_run,
            envelope_seq: got_envelope_seq,
            payload_seq,
        }) => {
            assert_eq!(got_run, run);
            assert_eq!(got_envelope_seq, envelope_seq);
            assert_eq!(payload_seq, seq.get());
        }
        other => panic!(
            "snapshot envelope-only mismatch must return exact ReplayEnvelopeSequenceMismatch, got {other:?}"
        ),
    }
}

#[test]
fn run_header_read_rejects_payload_run_not_matching_requested_key() {
    let (_temp, journal) = temp_journal();
    let requested_run = RunId::new(81);
    let stored_run = RunId::new(82);
    let record = RunHeaderRecord {
        run: stored_run,
        workflow_id: WorkflowId::new(7),
        compiled_digest: WorkflowDigest::from_bytes([0x82; DIGEST_BYTES]),
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    let key = crate::keys::run_header_key(requested_run).expect("run header key encodes");
    let value = crate::codec::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunHeader,
        stored_run.get(),
        &record,
        MAX_RUN_HEADER_BYTES,
    )
    .expect("forged run header envelope should encode");
    journal
        .run_header
        .insert(key.to_vec(), value)
        .expect("raw corrupt run header insert should succeed");

    let result = journal.run_header(requested_run);

    assert!(
        matches!(
            result,
            Err(JournalError::WrongRun { expected, actual })
                if expected == requested_run && actual == stored_run
        ),
        "run header read must rebind requested run key, got {result:?}"
    );
}

#[test]
fn run_header_read_rejects_envelope_only_sequence_mismatch_with_exact_fields() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(83);
    let envelope_seq = 9083;
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(7),
        compiled_digest: WorkflowDigest::from_bytes([0x83; DIGEST_BYTES]),
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    let key = crate::keys::run_header_key(run).expect("run header key encodes");
    let value = crate::codec::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunHeader,
        envelope_seq,
        &record,
        MAX_RUN_HEADER_BYTES,
    )
    .expect("forged run header envelope should encode");
    journal
        .run_header
        .insert(key.to_vec(), value)
        .expect("raw run header insert should succeed");

    let result = journal.run_header(run);

    match result {
        Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run: got_run,
            envelope_seq: got_envelope_seq,
            payload_seq,
        }) => {
            assert_eq!(got_run, run);
            assert_eq!(got_envelope_seq, envelope_seq);
            assert_eq!(payload_seq, run.get());
        }
        other => panic!(
            "run_header envelope-only mismatch must return exact ReplayEnvelopeSequenceMismatch, got {other:?}"
        ),
    }
}

#[test]
fn run_headers_scan_rejects_payload_run_not_matching_scanned_key() {
    let (_temp, journal) = temp_journal();
    let key_run = RunId::new(91);
    let payload_run = RunId::new(92);
    let record = RunHeaderRecord {
        run: payload_run,
        workflow_id: WorkflowId::new(9),
        compiled_digest: WorkflowDigest::from_bytes([0x92; DIGEST_BYTES]),
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    let key = crate::keys::run_header_key(key_run).expect("run header key encodes");
    let value = crate::codec::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunHeader,
        payload_run.get(),
        &record,
        MAX_RUN_HEADER_BYTES,
    )
    .expect("forged run header envelope should encode");
    journal
        .run_header
        .insert(key.to_vec(), value)
        .expect("raw corrupt run header insert should succeed");

    let result = journal.run_headers();

    assert!(
        matches!(
            result,
            Err(JournalError::WrongRun { expected, actual })
                if expected == key_run && actual == payload_run
        ),
        "run_headers scan must rebind scanned key to payload, got {result:?}"
    );
}

#[test]
fn run_headers_scan_rejects_envelope_only_sequence_mismatch_with_exact_fields() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(93);
    let envelope_seq = 9093;
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(9),
        compiled_digest: WorkflowDigest::from_bytes([0x93; DIGEST_BYTES]),
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    let key = crate::keys::run_header_key(run).expect("run header key encodes");
    let value = crate::codec::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunHeader,
        envelope_seq,
        &record,
        MAX_RUN_HEADER_BYTES,
    )
    .expect("forged run header envelope should encode");
    journal
        .run_header
        .insert(key.to_vec(), value)
        .expect("raw run header insert should succeed");

    let result = journal.run_headers();

    match result {
        Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run: got_run,
            envelope_seq: got_envelope_seq,
            payload_seq,
        }) => {
            assert_eq!(got_run, run);
            assert_eq!(got_envelope_seq, envelope_seq);
            assert_eq!(payload_seq, run.get());
        }
        other => panic!(
            "run_headers scan envelope-only mismatch must return exact ReplayEnvelopeSequenceMismatch, got {other:?}"
        ),
    }
}

