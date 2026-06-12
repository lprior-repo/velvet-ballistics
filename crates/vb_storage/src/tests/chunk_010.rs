#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn put_run_header_stores_and_retrieves() {
    // Given an open journal and a run header record
    // When put_run_header is called
    // Then the record can be retrieved by run id
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let record = RunHeaderRecord {
        run: RunId::new(123),
        workflow_id: WorkflowId::new(456),
        compiled_digest: WorkflowDigest::from_bytes([8; 32]),
        status: 1,
        accepted_at_ms: 1700000000,
    };
    journal
        .put_run_header(&record)
        .expect("journal.put_run_header must succeed");

    let retrieved = journal
        .run_header(RunId::new(123))
        .expect("run_header lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn put_compiled_ir_stores_and_retrieves() {
    // Given an open journal and a compiled IR record
    // When put_compiled_ir is called
    // Then the record can be retrieved by digest
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let record = crate::accepted_compiled_ir_record_for_test(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let digest = record.digest;
    journal
        .put_compiled_ir(&record)
        .expect("journal.put_compiled_ir must succeed");

    let retrieved = journal
        .compiled_ir(digest)
        .expect("compiled_ir lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn put_compiled_ir_rejects_forged_digest() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let valid = crate::accepted_compiled_ir_record_for_test(b"direct-forgery".to_vec());
    let forged_digest = WorkflowDigest::from_bytes([0xA5; DIGEST_BYTES]);
    let forged = CompiledIrRecord {
        digest: forged_digest,
        ir: valid.ir,
        ..Default::default()
    };

    assert!(matches!(
        journal.put_compiled_ir(&forged),
        Err(JournalError::ArtifactChecksumMismatch)
    ));
    assert!(
        journal
            .compiled_ir(forged_digest)
            .expect("compiled_ir lookup should succeed")
            .is_none(),
        "forged compiled IR must not be persisted"
    );
}

#[test]
fn put_compiled_ir_rejects_accepted_artifact_envelope_trailing_bytes() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut record = crate::accepted_compiled_ir_record_for_test(b"outer-trailing".to_vec());
    let declared_end = record.ir.len();
    record.ir.push(0xE7);

    let result = journal.put_compiled_ir(&record);

    let Err(JournalError::UnexpectedTrailingBytes {
        declared_end: found_declared_end,
        actual_len,
    }) = result
    else {
        panic!("put_compiled_ir must reject AcceptedArtifact trailing bytes, got {result:?}");
    };
    assert_eq!(found_declared_end, declared_end);
    assert_eq!(actual_len, record.ir.len());
    assert!(
        journal
            .compiled_ir(record.digest)
            .expect("compiled_ir lookup should succeed")
            .is_none(),
        "trailing-byte AcceptedArtifact must not be persisted"
    );
}

#[test]
fn compiled_ir_read_revalidates_persisted_record() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let valid = crate::accepted_compiled_ir_record_for_test(b"read-corrupt".to_vec());
    let corrupt = CompiledIrRecord {
        digest: valid.digest,
        ir: vec![0xCA, 0xFE],
        ..Default::default()
    };
    let key = compiled_ir_key(corrupt.digest.as_bytes())
        .expect("compiled_ir key construction should succeed");
    let value = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &corrupt,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("compiled_ir record encoding should succeed");

    journal
        .compiled_ir
        .insert(key.to_vec(), value)
        .expect("direct fixture insert should succeed");

    assert!(matches!(
        journal.compiled_ir(corrupt.digest),
        Err(JournalError::PostcardDecodeFailed)
    ));
}

#[test]
fn compiled_ir_rejects_workflow_parts_inner_trailing_bytes() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut record = crate::accepted_compiled_ir_record_for_test(b"inner-trailing".to_vec());
    let mut artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&record.ir).expect("AcceptedArtifact should decode");
    let declared_end = artifact.ir.len();
    artifact.ir.push(0x7E);
    record.ir = postcard::to_allocvec(&artifact).expect("AcceptedArtifact should encode");
    let key = compiled_ir_key(record.digest.as_bytes())
        .expect("compiled_ir key construction should succeed");
    let value = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("compiled_ir record encoding should succeed");

    journal
        .compiled_ir
        .insert(key.to_vec(), value)
        .expect("direct fixture insert should succeed");
    let result = journal.compiled_ir(record.digest);

    let Err(JournalError::UnexpectedTrailingBytes {
        declared_end: found_declared_end,
        actual_len,
    }) = result
    else {
        panic!("compiled_ir must reject WorkflowParts trailing bytes, got {result:?}");
    };
    assert_eq!(found_declared_end, declared_end);
    assert_eq!(actual_len, declared_end.saturating_add(1));
}

#[test]
fn put_blob_stores_and_retrieves() {
    // Given an open journal and a blob record
    // When put_blob is called
    // Then the record can be retrieved by digest
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let blob_bytes = vec![1, 2, 3, 4, 5];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    journal
        .put_blob(&record)
        .expect("journal.put_blob must succeed");

    let retrieved = journal.blob(digest).expect("blob lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn put_snapshot_stores_and_retrieves() {
    // Given an open journal and a run snapshot
    // When put_snapshot is called
    // Then the snapshot can be retrieved by run and seq
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let snapshot = RunSnapshot {
        run: RunId::new(88),
        seq: EventSeq::new(10),
        workflow: WorkflowDigest::from_bytes([7; 32]),
        slots: vec![1, 2, 3],
        taint: Vec::new(),
    };
    journal
        .put_snapshot(&snapshot)
        .expect("journal.put_snapshot must succeed");

    let retrieved = journal
        .snapshot(RunId::new(88), EventSeq::new(10))
        .expect("snapshot lookup should succeed");
    assert_eq!(retrieved, Some(snapshot));
}
