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

// =========================================================================
// Section: Adversarial Postcard / Encoding Edge Cases
// =========================================================================

#[test]
fn adversarial_valid_header_garbage_postcard_returns_decode_failed() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(13),
        seq: EventSeq::new(0),
        workflow: test_digest(13),
    };
    let mut enc = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    if let Some(b) = enc.get_mut(60) {
        *b = 0xFF;
    }
    let digest_bytes = *blake3::hash(&enc[60..]).as_bytes();
    enc.get_mut(24..56)
        .expect("digest")
        .copy_from_slice(&digest_bytes);
    let cs = crc32c::crc32c(&enc[..56]);
    enc[56] = (cs & 0xFF) as u8;
    enc[57] = ((cs >> 8) & 0xFF) as u8;
    enc[58] = ((cs >> 16) & 0xFF) as u8;
    enc[59] = ((cs >> 24) & 0xFF) as u8;
    assert!(matches!(
        decode_record::<JournalEvent>(&enc, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::PostcardDecodeFailed)
    ));
}

#[test]
fn adversarial_run_header_wrong_magic_returns_bad_magic() {
    let record = RunHeaderRecord {
        run: RunId::new(123),
        workflow_id: WorkflowId::new(456),
        compiled_digest: test_digest(8),
        status: 1,
        accepted_at_ms: 1700000000,
    };
    let enc = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunHeader,
        record.run.get(),
        &record,
        MAX_RUN_HEADER_BYTES,
    )
    .expect("ok");
    assert!(matches!(
        decode_record::<RunHeaderRecord>(&enc, MAGIC_BLOB, MAX_RUN_HEADER_BYTES),
        Err(JournalError::BadMagic { .. })
    ));
}

#[test]
fn adversarial_decode_empty_returns_unexpected_eof() {
    assert!(matches!(
        decode_record::<JournalEvent>(&[][..], MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::UnexpectedEof)
    ));
}

#[test]
fn adversarial_encode_empty_blob_succeeds() {
    assert!(
        encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &BlobRecord {
                digest: [0; 32],
                bytes: vec![]
            },
            MAX_BLOB_BYTES
        )
        .is_ok()
    );
}

#[test]
fn adversarial_encode_empty_source_succeeds() {
    assert!(
        encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &WorkflowSourceRecord {
                digest: test_digest(0),
                source: vec![]
            },
            MAX_WORKFLOW_SOURCE_BYTES
        )
        .is_ok()
    );
}

#[test]
fn adversarial_encode_empty_ir_succeeds() {
    assert!(
        encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &CompiledIrRecord {
                digest: test_digest(0),
                ir: vec![],
                ..Default::default()
            },
            MAX_COMPILED_IR_BYTES
        )
        .is_ok()
    );
}

#[test]
fn journal_error_diagnostic_codes_are_unique() {
    let errors = [
        JournalError::KeyCapacity,
        JournalError::WriteLockPoisoned,
        JournalError::QueueCapacity,
        JournalError::QueueFull,
        JournalError::SequenceOverflow,
        JournalError::HeaderChecksumMismatch,
        JournalError::PayloadDigestMismatch,
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
        JournalError::DuplicateEvent {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        },
        JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        },
        JournalError::SequenceGap {
            expected: EventSeq::new(0),
            actual: EventSeq::new(1),
        },
        JournalError::BadMagic { found: 0 },
        JournalError::UnsupportedSchemaVersion { version: 0 },
        JournalError::MigrationRequired { from: 0, to: 1 },
        JournalError::UnknownRecordKind { kind: 0 },
        JournalError::RecordKindFamilyMismatch { magic: 0, kind: 0 },
        JournalError::HeaderLengthMismatch { found: 0 },
        JournalError::PayloadTooLarge { len: 0, max: 0 },
        JournalError::TooManyEvents {
            run: RunId::new(1),
            limit: 1,
            observed: 2,
        },
        JournalError::ReplayAllocationFailed {
            run: RunId::new(1),
            requested: 1,
        },
    ];
    let mut seen = std::collections::BTreeSet::new();
    for err in &errors {
        let code = err.diagnostic_code();
        assert!(seen.insert(code), "duplicate diagnostic code: {code}");
    }
    assert_eq!(seen.len(), errors.len());
}

#[test]
fn journal_error_diagnostic_code_fjall() {
    // Fjall and Encode variants hold external errors; we verify via KeyCapacity
    assert_eq!(
        JournalError::KeyCapacity.diagnostic_code(),
        DiagnosticCode::new(0x4003)
    );
}

#[test]
fn journal_error_diagnostic_code_duplicate_event() {
    assert_eq!(
        JournalError::DuplicateEvent {
            run: RunId::new(42),
            seq: EventSeq::new(7),
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x4004)
    );
}

#[test]
fn journal_error_diagnostic_code_write_lock_poisoned() {
    assert_eq!(
        JournalError::WriteLockPoisoned.diagnostic_code(),
        DiagnosticCode::new(0x4005)
    );
}

#[test]
fn journal_error_diagnostic_code_queue_capacity() {
    assert_eq!(
        JournalError::QueueCapacity.diagnostic_code(),
        DiagnosticCode::new(0x4006)
    );
}

#[test]
fn journal_error_diagnostic_code_queue_full() {
    assert_eq!(
        JournalError::QueueFull.diagnostic_code(),
        DiagnosticCode::new(0x4007)
    );
}
