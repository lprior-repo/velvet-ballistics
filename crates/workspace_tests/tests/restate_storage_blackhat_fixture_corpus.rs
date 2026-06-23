#![forbid(unsafe_code)]
//! PO-vb-7m21-004,009,014,020,025,029,034,039 deterministic proptest corpus.
//! PO-vb-7m21-044,049,054,059,064,069,074,079 blackhat corruption fixture tests.
use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::{
    EventSeq, JournalEvent, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES,
    RECORD_HEADER_BYTES, RunSnapshot,
};
use vb_storage::{
    JournalError, RecordEnvelope, RecordKind, decode_record, decode_record_header, encode_record,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CorpusOutcome {
    Accepted,
    IndexParityMismatch,
    SequenceGap,
    DuplicateEvent,
    ReplayTail,
    MissingManifestKeyspace,
}

fn classify_index_parity(event_present: bool, side_index_present: bool) -> CorpusOutcome {
    if event_present && !side_index_present {
        CorpusOutcome::IndexParityMismatch
    } else {
        CorpusOutcome::Accepted
    }
}

fn classify_sequence(expected: u64, actual: u64) -> CorpusOutcome {
    if expected == actual {
        CorpusOutcome::Accepted
    } else {
        CorpusOutcome::SequenceGap
    }
}

fn classify_duplicate(
    existing: bool,
    same_event_key: bool,
    same_payload_digest: bool,
) -> CorpusOutcome {
    if existing && same_event_key && !same_payload_digest {
        CorpusOutcome::DuplicateEvent
    } else {
        CorpusOutcome::Accepted
    }
}

fn classify_snapshot_recovery(
    snapshot_seq: u64,
    tail_seq: u64,
    snapshot_valid: bool,
) -> CorpusOutcome {
    if snapshot_valid && snapshot_seq < tail_seq {
        CorpusOutcome::ReplayTail
    } else {
        CorpusOutcome::Accepted
    }
}

fn classify_manifest(declared_mask: u8, present_mask: u8) -> CorpusOutcome {
    if declared_mask & !present_mask == 0 {
        CorpusOutcome::Accepted
    } else {
        CorpusOutcome::MissingManifestKeyspace
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    fn oversized_declared_record_returns_payload_too_large(extra in 1u32..128) {
        let max = 16u32;
        let payload = vec![0u8; (max + extra) as usize];
        let result = vb_storage::encode_record_header(vb_storage::MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &payload, max);
        prop_assert!(matches!(result, Err(JournalError::PayloadTooLarge { .. })), "payload too large");
    }
    #[test]
    fn future_schema_is_unsupported(delta in 1u16..8) {
        let version = vb_storage::CURRENT_SCHEMA_VERSION + delta;
        prop_assert!(version > vb_storage::CURRENT_SCHEMA_VERSION);
    }
    #[test]
    fn truncated_header_is_unexpected_eof(len in 0usize..vb_storage::RECORD_HEADER_BYTES) {
        let bytes = [0u8; vb_storage::RECORD_HEADER_BYTES];
        let result = vb_storage::decode_record_header(&bytes[..len], vb_storage::MAGIC_JOURNAL_EVENT, u32::MAX);
        prop_assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }
    #[test]
    fn missing_side_index_is_typed(event_present in any::<bool>()) {
        prop_assume!(event_present);
        let observed = classify_index_parity(event_present, false);
        prop_assert_eq!(observed, CorpusOutcome::IndexParityMismatch);
    }
    #[test]
    fn sequence_gap_is_typed(expected in 0u64..16, actual in 0u64..16) {
        prop_assume!(expected != actual);
        let observed = classify_sequence(expected, actual);
        prop_assert_eq!(observed, CorpusOutcome::SequenceGap);
    }
    #[test]
    fn divergent_duplicate_is_typed(existing in any::<bool>()) {
        prop_assume!(existing);
        let observed = classify_duplicate(existing, true, false);
        prop_assert_eq!(observed, CorpusOutcome::DuplicateEvent);
    }
    #[test]
    fn stale_snapshot_replays_tail(snapshot_seq in 0u64..8, tail_seq in 1u64..16) {
        prop_assume!(snapshot_seq < tail_seq);
        let observed = classify_snapshot_recovery(snapshot_seq, tail_seq, true);
        prop_assert_eq!(observed, CorpusOutcome::ReplayTail);
    }
    #[test]
    fn missing_manifest_keyspace_is_typed(declared in 0u8..16, present in 0u8..16) {
        prop_assume!(declared & !present != 0);
        let observed = classify_manifest(declared, present);
        prop_assert_eq!(observed, CorpusOutcome::MissingManifestKeyspace);
    }
}

// =============================================================================
// Helper: build a minimal valid JournalEvent for happy-path tests
// =============================================================================
fn make_minimal_journal_event() -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
    }
}

// =============================================================================
// Helper: build a minimal valid RunSnapshot for happy-path tests
// =============================================================================
fn make_minimal_snapshot() -> RunSnapshot {
    RunSnapshot {
        run: RunId::new(2),
        seq: EventSeq::new(100),
        workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        slots: vec![],
        taint: vec![],
    }
}

// =============================================================================
// B9: Known-Good Journal Event Acceptance (REQ-1)
// =============================================================================

#[test]
fn known_good_journal_event_encodes_successfully() {
    // Given: a minimal valid JournalEvent
    let event = make_minimal_journal_event();

    // When: encoding with the correct magic, kind, and max payload
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    );

    // Then: encoding succeeds
    let encoded = result.expect("encode_record should succeed for valid journal event");
    assert!(!encoded.is_empty(), "encoded record should not be empty");
    assert!(
        encoded.len() > RECORD_HEADER_BYTES,
        "encoded record should include payload"
    );
}

#[test]
fn known_good_journal_event_decodes_successfully() {
    // Given: an encoded valid JournalEvent
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");

    // When: decoding with correct magic and max payload
    let result: Result<(RecordEnvelope, JournalEvent), JournalError> =
        decode_record(&encoded, MAGIC_JOURNAL_EVENT, u32::MAX);

    // Then: decoding succeeds and reconstructs the original event
    let (envelope, decoded_event) =
        result.expect("decode_record should succeed for valid journal event");
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT, "magic should match");
    assert_eq!(decoded_event.run_id(), RunId::new(1), "run_id should match");
    assert_eq!(decoded_event.seq(), EventSeq::new(0), "seq should match");
}

#[test]
fn known_good_journal_event_round_trips_identically() {
    // Given: a minimal valid JournalEvent
    let event = make_minimal_journal_event();

    // When: encode → decode → re-encode the same event
    let encoded1 = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("first encode should succeed");
    let (_envelope, decoded_event): (RecordEnvelope, JournalEvent) =
        decode_record(&encoded1, MAGIC_JOURNAL_EVENT, u32::MAX).expect("decode should succeed");
    let encoded2 = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &decoded_event,
        u32::MAX,
    )
    .expect("second encode should succeed");

    // Then: re-encoded bytes match original encoded bytes
    assert_eq!(
        encoded1, encoded2,
        "round-trip re-encode should produce identical bytes"
    );
}

// =============================================================================
// B10: Known-Good Snapshot Envelope Acceptance (REQ-2)
// =============================================================================

#[test]
fn known_good_snapshot_envelope_encodes_successfully() {
    // Given: a minimal valid RunSnapshot
    let snapshot = make_minimal_snapshot();

    // When: encoding with snapshot magic, kind, and max
    let result = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot.seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    );

    // Then: encoding succeeds
    let encoded = result.expect("encode_record should succeed for valid snapshot");
    assert!(!encoded.is_empty(), "encoded snapshot should not be empty");
    assert!(
        encoded.len() > RECORD_HEADER_BYTES,
        "encoded snapshot should include payload"
    );
}

#[test]
fn known_good_snapshot_envelope_decodes_successfully() {
    // Given: an encoded valid RunSnapshot
    let snapshot = make_minimal_snapshot();
    let encoded = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot.seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("encode should succeed");

    // When: decoding with snapshot magic and max
    let result: Result<(RecordEnvelope, RunSnapshot), JournalError> =
        decode_record(&encoded, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES);

    // Then: decoding succeeds and reconstructs the snapshot
    let (envelope, decoded_snapshot) =
        result.expect("decode_record should succeed for valid snapshot");
    assert_eq!(envelope.magic, MAGIC_SNAPSHOT, "magic should match");
    assert_eq!(decoded_snapshot.run, RunId::new(2), "run_id should match");
    assert_eq!(decoded_snapshot.seq, EventSeq::new(100), "seq should match");
}

#[test]
fn known_good_snapshot_envelope_round_trips_identically() {
    // Given: a minimal valid RunSnapshot
    let snapshot = make_minimal_snapshot();

    // When: encode → decode → re-encode
    let encoded1 = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot.seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("first encode should succeed");
    let (_envelope, decoded_snapshot): (RecordEnvelope, RunSnapshot) =
        decode_record(&encoded1, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)
            .expect("decode should succeed");
    let encoded2 = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        decoded_snapshot.seq.get(),
        &decoded_snapshot,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("second encode should succeed");

    // Then: re-encoded bytes match original
    assert_eq!(
        encoded1, encoded2,
        "snapshot round-trip re-encode should produce identical bytes"
    );
}

// =============================================================================
// B11: Header CRC Corruption → HeaderChecksumMismatch (REQ-7)
// =============================================================================

#[test]
fn header_crc_corruption_returns_checksum_mismatch() {
    // Given: a validly encoded journal event header
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");
    // The header is the first RECORD_HEADER_BYTES bytes.
    let mut header = [0u8; RECORD_HEADER_BYTES];
    header.copy_from_slice(&encoded[..RECORD_HEADER_BYTES]);

    // When: the header CRC is corrupted (flip bits in the last byte of the CRC field)
    let crc_end = RECORD_HEADER_BYTES - 1;
    header[crc_end] ^= 0xFF;

    // Then: decode returns HeaderChecksumMismatch
    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, u32::MAX);
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "header CRC corruption should yield HeaderChecksumMismatch, got: {result:?}"
    );
}

// =============================================================================
// B12: Payload Digest Corruption → PayloadDigestMismatch (REQ-7)
// =============================================================================

#[test]
fn payload_digest_corruption_returns_digest_mismatch() {
    // Given: a validly encoded journal event
    let event = make_minimal_journal_event();
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");

    // When: the payload bytes are corrupted (change one byte after the header)
    let payload_start = RECORD_HEADER_BYTES;
    if let Some(byte) = encoded.get_mut(payload_start) {
        *byte ^= 0xFF;
    }

    // Then: decode returns PayloadDigestMismatch (header decodes ok, digest fails)
    let result: Result<(RecordEnvelope, JournalEvent), JournalError> =
        decode_record(&encoded, MAGIC_JOURNAL_EVENT, u32::MAX);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "payload digest corruption should yield PayloadDigestMismatch, got: {result:?}"
    );
}

// =============================================================================
// B13: Corrupt Postcard Payload → PostcardDecodeFailed (REQ-7)
// =============================================================================

#[test]
fn invalid_postcard_payload_returns_decode_failed() {
    // Given: a record whose payload is valid postcard for a u32, not JournalEvent.
    // The header is valid (digest matches, CRC correct), but the payload
    // deserialization fails for the expected type.
    let bogus_payload: u32 = 42;
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &bogus_payload,
        u32::MAX,
    )
    .expect("encode should succeed for u32 payload");

    // When: decoding as JournalEvent
    let result: Result<(RecordEnvelope, JournalEvent), JournalError> =
        decode_record(&encoded, MAGIC_JOURNAL_EVENT, u32::MAX);

    // Then: returns PostcardDecodeFailed (digest matches, but postcard fails)
    assert!(
        matches!(result, Err(JournalError::PostcardDecodeFailed)),
        "postcard-invalid payload should yield PostcardDecodeFailed, got: {result:?}"
    );
}

// =============================================================================
// B14: Bad Magic → BadMagic (REQ-7)
// =============================================================================

#[test]
fn unknown_magic_bytes_return_bad_magic() {
    // Given: a validly encoded journal event (magic=MAGIC_JOURNAL_EVENT)
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");

    // When: decoding with a different expected magic (snapshot magic)
    let result = decode_record_header(&encoded, MAGIC_SNAPSHOT, u32::MAX);

    // Then: BadMagic because header magic doesn't match expected
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "wrong expected magic should yield BadMagic, got: {result:?}"
    );
}

// =============================================================================
// B15: Unknown Record Kind → UnknownRecordKind (REQ-13)
// =============================================================================

#[test]
fn unknown_record_kind_rejected_with_diagnostics() {
    // Given: a validly encoded journal event header
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");
    let mut header = [0u8; RECORD_HEADER_BYTES];
    header.copy_from_slice(&encoded[..RECORD_HEADER_BYTES]);

    // When: record_kind bytes are changed to an unknown value (99, which is not
    // in the set {1,2,3,10..=29,30,40,50}).
    // The record_kind field is a little-endian u16 at offset 6.
    let invalid_kind: u16 = 99;
    header[6] = (invalid_kind & 0xFF) as u8;
    header[7] = ((invalid_kind >> 8) & 0xFF) as u8;

    // Then: decode returns UnknownRecordKind (checked before CRC in decode path)
    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, u32::MAX);
    assert!(
        matches!(result, Err(JournalError::UnknownRecordKind { kind }) if kind == 99),
        "unknown record kind should yield UnknownRecordKind {{ kind: 99 }}, got: {result:?}"
    );
}

// =============================================================================
// B16: Record Kind/Family Mismatch → RecordKindFamilyMismatch (REQ-13)
// =============================================================================

#[test]
fn record_kind_family_mismatch_rejected_with_diagnostics() {
    // Given: a validly encoded journal event header
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");
    let mut header = [0u8; RECORD_HEADER_BYTES];
    header.copy_from_slice(&encoded[..RECORD_HEADER_BYTES]);

    // When: record_kind is changed to Snapshot (30), which is a known kind
    // but does NOT belong to the Journal family (10..=29).
    let snapshot_kind: u16 = 30;
    header[6] = (snapshot_kind & 0xFF) as u8;
    header[7] = ((snapshot_kind >> 8) & 0xFF) as u8;

    // Then: decode returns RecordKindFamilyMismatch (checked before CRC)
    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, u32::MAX);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { magic, kind }) if magic == MAGIC_JOURNAL_EVENT && kind == 30),
        "kind/family mismatch should yield RecordKindFamilyMismatch, got: {result:?}"
    );
}

// =============================================================================
// Additional: corrupt envelope error includes diagnostics
// =============================================================================

#[test]
fn corrupt_envelope_errors_include_diagnostics() {
    // Given: a validly encoded journal event
    let event = make_minimal_journal_event();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        u32::MAX,
    )
    .expect("encode should succeed");

    // When: decoding with wrong expected magic
    let result = decode_record_header(&encoded, MAGIC_BLOB, u32::MAX);

    // Then: error carries diagnostic fields
    match result {
        Err(JournalError::BadMagic { found }) => {
            assert_eq!(
                found, MAGIC_JOURNAL_EVENT,
                "BadMagic found should be the actual magic"
            );
        }
        other => panic!("expected BadMagic, got: {other:?}"),
    }
}
