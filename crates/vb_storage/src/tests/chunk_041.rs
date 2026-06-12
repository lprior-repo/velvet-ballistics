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


// ========================================================================
// vb-1cwhx: RecoveryStamp (wire ID 7) BDD parity
// ========================================================================

/// Given a recovery-stamp record (kind 7) with explicit payload
/// When the record is encoded with `MAGIC_RECOVERY_STAMP` and decoded
/// Then the envelope metadata round-trips and the payload deserializes
///      to the original record (and `is_known_record_kind(7)` is true).
#[test]
fn recovery_stamp_record_round_trips_through_decoder() {
    use crate::{MAGIC_RECOVERY_STAMP, MAX_RECOVERY_STAMP_BYTES, RecoveryStampRecord};

    // Sanity: wire id 7 must be admitted by the durable kind validator.
    assert!(
        crate::codec::is_known_record_kind(7),
        "is_known_record_kind(7) must be true for RecoveryStamp"
    );

    // Sanity: MAGIC_RECOVERY_STAMP must not collide with MAGIC_WORKFLOW_SOURCE.
    assert_ne!(
        MAGIC_RECOVERY_STAMP, MAGIC_WORKFLOW_SOURCE,
        "MAGIC_RECOVERY_STAMP must be distinct from MAGIC_WORKFLOW_SOURCE (VBSR)"
    );

    let stamp = RecoveryStampRecord {
        run: RunId::new(42),
        last_seq: EventSeq::new(7),
        written_at_ms: 1_700_000_000_000,
    };

    let encoded = encode_record(
        MAGIC_RECOVERY_STAMP,
        RecordKind::RecoveryStamp,
        stamp.last_seq.get(),
        &stamp,
        MAX_RECOVERY_STAMP_BYTES,
    )
    .expect("encoding RecoveryStamp with MAGIC_RECOVERY_STAMP must succeed");
    assert!(
        encoded.len() > RECORD_HEADER_BYTES,
        "encoded record exceeds header"
    );

    let (envelope, decoded) = decode_record::<RecoveryStampRecord>(
        &encoded,
        MAGIC_RECOVERY_STAMP,
        MAX_RECOVERY_STAMP_BYTES,
    )
    .expect("decoding RecoveryStamp with MAGIC_RECOVERY_STAMP must succeed");
    assert_eq!(envelope.magic, MAGIC_RECOVERY_STAMP);
    assert_eq!(envelope.record_kind, RecordKind::RecoveryStamp.id());
    assert_eq!(envelope.record_kind, 7);
    assert_eq!(decoded, stamp);
}


/// Given a typed `recovery_stamp_key(run, seq)` encoding
/// When the resulting bytes are classified by `try_key_prefix` and decoded
///      by `decode_storage_key`
/// Then the prefix is `KeyPrefix::RecoveryStamp` (`0x40`) and the decoded
///      `StorageKey::RecoveryStamp` matches the input.
#[test]
fn recovery_stamp_key_prefix_round_trips() {
    use crate::keys::{KeyPrefix, recovery_stamp_key, try_key_prefix};

    let run = RunId::new(99);
    let seq = EventSeq::new(13);
    let bytes = recovery_stamp_key(run, seq).expect("recovery_stamp_key must succeed");
    assert_eq!(bytes.len(), crate::constants::RECOVERY_STAMP_KEY_BYTES);
    assert_eq!(
        bytes[0],
        crate::constants::PREFIX_RECOVERY_STAMP,
        "recovery_stamp key prefix must be 0x40"
    );
    assert_eq!(
        try_key_prefix(&bytes).expect("prefix must classify"),
        KeyPrefix::RecoveryStamp
    );

    let decoded = crate::keys::decode_storage_key(&bytes)
        .expect("decode_storage_key must round-trip recovery_stamp key");
    assert_eq!(
        decoded,
        StorageKey::RecoveryStamp { run, seq },
        "decoded StorageKey must equal the encoded (run, seq)"
    );
}


/// Given any other record kind (e.g. WorkflowSource=1, Snapshot=30, Blob=40)
/// When the encoder is invoked with `MAGIC_RECOVERY_STAMP`
/// Then the encoder rejects the request with
///      `JournalError::RecordKindFamilyMismatch` (no cross-family
///      kind smuggling is possible).
#[test]
fn recovery_stamp_magic_rejects_other_record_kinds() {
    use crate::{MAGIC_RECOVERY_STAMP, MAX_RECOVERY_STAMP_BYTES, WorkflowSourceRecord};

    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
        source: b"x".to_vec(),
    };
    let err = encode_record(
        MAGIC_RECOVERY_STAMP,
        RecordKind::WorkflowSource,
        0,
        &source,
        MAX_RECOVERY_STAMP_BYTES,
    )
    .expect_err("WorkflowSource under MAGIC_RECOVERY_STAMP must fail");
    assert!(
        matches!(err, JournalError::RecordKindFamilyMismatch { .. }),
        "expected RecordKindFamilyMismatch, got {err:?}"
    );
}
