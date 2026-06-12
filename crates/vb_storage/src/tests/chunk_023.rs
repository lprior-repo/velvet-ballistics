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
fn adversarial_decode_header_len_not_60_returns_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(6),
        seq: EventSeq::new(0),
        workflow: test_digest(6),
    };
    let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::HeaderLengthMismatch { found }) = result else {
        panic!("expected mismatch, got {:?}", result)
    };
    assert_eq!(found, 48);
}

#[test]
fn adversarial_decode_payload_len_above_limit_returns_too_large() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(7),
        seq: EventSeq::new(0),
        workflow: test_digest(7),
    };
    let encoded =
        encode_and_patch_field(&event, RecordKind::RunAccepted, 12, &9999u32.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 100);
    let Err(JournalError::PayloadTooLarge { len, max }) = result else {
        panic!("expected PayloadTooLarge, got {:?}", result)
    };
    assert_eq!(len, 9999);
    assert_eq!(max, 100);
}

#[test]
fn adversarial_decode_corrupt_header_crc_returns_checksum_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(8),
        seq: EventSeq::new(0),
        workflow: test_digest(8),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    if let Some(b) = encoded.get_mut(57) {
        *b ^= 0x80;
    }
    assert!(matches!(
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::HeaderChecksumMismatch)
    ));
}

#[test]
fn adversarial_decode_corrupt_payload_digest_returns_digest_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(9),
        seq: EventSeq::new(0),
        workflow: test_digest(9),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    if let Some(b) = encoded.get_mut(61) {
        *b ^= 0xFF;
    }
    assert!(matches!(
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::PayloadDigestMismatch)
    ));
}

#[test]
fn adversarial_decode_truncated_before_full_header_returns_unexpected_eof() {
    let truncated = [0u8; 45];
    assert!(matches!(
        decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::UnexpectedEof)
    ));
}

#[test]
fn adversarial_decode_truncated_before_full_payload_returns_unexpected_eof() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(10),
        seq: EventSeq::new(0),
        workflow: test_digest(10),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    let truncated = encoded.get(..62).expect("slice");
    assert!(matches!(
        decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::UnexpectedEof)
    ));
}

// =========================================================================
// Section: Adversarial Key Encoding Tests
// =========================================================================

#[test]
fn adversarial_key_prefix_isolation_proves_different_prefixes() {
    let digest = [0xAB; 32];
    let ws = workflow_source_key(digest).expect("ws");
    let ci = compiled_ir_key(digest).expect("ci");
    let bl = blob_key(digest).expect("bl");
    assert_ne!(ws[0], ci[0]);
    assert_ne!(ws[0], bl[0]);
    assert_eq!(ws[1..], ci[1..]);
    assert_eq!(ws[1..], bl[1..]);
}

#[test]
fn adversarial_key_wrong_endianness_produces_different_keys() {
    let key = run_header_key(RunId::new(1)).expect("key");
    let mut le = [0u8; 9];
    le[0] = PREFIX_RUN_HEADER;
    le[1..9].copy_from_slice(&1u64.to_le_bytes());
    assert_ne!(key.as_slice(), le.as_slice());
    assert_eq!(key[1..9], 1u64.to_be_bytes());
}

#[test]
fn adversarial_key_no_collision_different_runs_same_seq() {
    let k1 = run_event_key(RunId::new(100), EventSeq::new(5)).expect("k1");
    let k2 = run_event_key(RunId::new(200), EventSeq::new(5)).expect("k2");
    assert_ne!(k1.as_slice(), k2.as_slice());
}

#[test]
fn adversarial_key_no_collision_same_run_different_seq() {
    let k1 = run_event_key(RunId::new(100), EventSeq::new(0)).expect("k1");
    let k2 = run_event_key(RunId::new(100), EventSeq::new(1)).expect("k2");
    assert_ne!(k1.as_slice(), k2.as_slice());
}

#[test]
fn adversarial_key_no_collision_different_digests() {
    assert_ne!(
        blob_key([1u8; 32]).expect("k1").as_slice(),
        blob_key([2u8; 32]).expect("k2").as_slice()
    );
}

// =========================================================================
// Section: Adversarial Journal / Replay Tests
// =========================================================================

#[test]
fn adversarial_append_duplicate_sequence_rejected_with_exact_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let run = RunId::new(50);
    assert!(
        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1)
            })
            .is_ok()
    );
    let result = journal.append_journaled(&JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        attempt: 1,
    });
    let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
        panic!("expected DuplicateEvent, got {:?}", result)
    };
    assert_eq!(r, run);
    assert_eq!(s, EventSeq::new(0));
}
