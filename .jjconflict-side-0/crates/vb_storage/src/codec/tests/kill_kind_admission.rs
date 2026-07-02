#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used
)]
//! Unit tests for RecordKind::RunKilled (kind 28) storage admission.
//!
//! Covers behaviors B42-B53 from the cancel/kill lattice test plan.
//! These tests verify that kind 28 is properly admitted into the journal
//! event family but rejected from snapshot, blob, and unknown families.

use super::*;
use crate::{
    JournalEvent,
    constants::{MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    records::RecordKind,
    types::EventSeq,
};
use vb_core::RunId;

use crate::codec::validation::{
    is_known_record_kind, unknown_record_kind_value, validate_kind_family, validate_known_kind,
};

// =============================================================================
// B42: RecordKind::RunKilled.id() == 28
// =============================================================================

#[test]
fn record_kind_run_killed_id_is_28() {
    assert_eq!(
        RecordKind::RunKilled.id(),
        28,
        "RecordKind::RunKilled.id() must equal 28 per durable storage contract"
    );
}

// =============================================================================
// B43: is_known_record_kind(28) returns true
// =============================================================================

#[test]
fn is_known_record_kind_28_returns_true() {
    assert!(
        is_known_record_kind(28),
        "kind 28 must be recognized as a known record kind"
    );
}

// =============================================================================
// B44: validate_kind_family(MAGIC_JOURNAL_EVENT, 28) returns Ok
// =============================================================================

#[test]
fn validate_kind_family_journal_event_28_returns_ok() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 28);
    assert!(
        result.is_ok(),
        "kind 28 must be accepted for MAGIC_JOURNAL_EVENT, got {:?}",
        result
    );
}

// =============================================================================
// B45: validate_kind_family(MAGIC_SNAPSHOT, 28) returns Err
// =============================================================================

#[test]
fn validate_kind_family_snapshot_28_returns_rejection() {
    let result = validate_kind_family(MAGIC_SNAPSHOT, 28);
    assert!(
        matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_SNAPSHOT,
                kind: 28
            })
        ),
        "kind 28 must be rejected for MAGIC_SNAPSHOT, got {:?}",
        result
    );
}

// =============================================================================
// B46: validate_kind_family(MAGIC_BLOB, 28) returns Err
// =============================================================================

#[test]
fn validate_kind_family_blob_28_returns_rejection() {
    let result = validate_kind_family(MAGIC_BLOB, 28);
    assert!(
        matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_BLOB,
                kind: 28
            })
        ),
        "kind 28 must be rejected for MAGIC_BLOB, got {:?}",
        result
    );
}

// =============================================================================
// B47: encode_record for RunKilled produces valid bytes
// =============================================================================

#[test]
fn encode_record_runkilled_produces_valid_bytes() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        attempt: 1,
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunKilled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(result.is_ok(), "RunKilled event must encode successfully");
    let bytes = result.unwrap();
    assert!(
        !bytes.is_empty(),
        "encoded RunKilled must produce non-empty bytes"
    );
    assert!(
        bytes.len() > RECORD_HEADER_BYTES,
        "encoded RunKilled must be larger than header alone"
    );
}

// =============================================================================
// B48: decode_record round-trip for RunKilled
// =============================================================================

#[test]
fn decode_record_runkilled_roundtrip() {
    let original = JournalEvent::RunKilled {
        run: RunId::new(99),
        seq: EventSeq::new(5),
        attempt: 3,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunKilled,
        original.seq().get(),
        &original,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode RunKilled should succeed");

    let (envelope, decoded) =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
            .expect("decode RunKilled should succeed");

    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunKilled.id());
    assert_eq!(envelope.sequence, 5);
    assert_eq!(
        decoded, original,
        "RunKilled must round-trip through encode/decode"
    );
}

// =============================================================================
// B49: decode_journal_event validates RunKilled
// =============================================================================

#[test]
fn decode_journal_event_runkilled_passes_validation() {
    let original = JournalEvent::RunKilled {
        run: RunId::new(10),
        seq: EventSeq::new(1),
        attempt: 2,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunKilled,
        original.seq().get(),
        &original,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode RunKilled should succeed");

    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
            .expect("decode_journal_event RunKilled should succeed with valid fields");

    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(decoded, original);
}

// =============================================================================
// B49-ext: decode_journal_event rejects RunKilled with RunId(0)
// =============================================================================

#[test]
fn decode_journal_event_runkilled_zero_run_rejected() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
    };
    // RunId(0) is valid for serialization, but is_valid() returns false
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunKilled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode should succeed even for zero-run (checked at validate level)");

    let result = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "decode_journal_event must reject RunKilled with RunId(0), got {:?}",
        result
    );
}

// =============================================================================
// B49-ext: decode_journal_event rejects RunKilled with attempt=0
// =============================================================================

#[test]
fn decode_journal_event_runkilled_zero_attempt_rejected() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 0,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunKilled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode should succeed even for zero-attempt");

    let result = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "decode_journal_event must reject RunKilled with attempt=0, got {:?}",
        result
    );
}

// =============================================================================
// B50: validate_known_kind(28) returns Ok
// =============================================================================

#[test]
fn validate_known_kind_28_returns_ok() {
    let result = validate_known_kind(28);
    assert!(
        result.is_ok(),
        "validate_known_kind(28) must return Ok, got {:?}",
        result
    );
}

// =============================================================================
// B51: unknown_record_kind_value(28) returns None
// =============================================================================

#[test]
fn unknown_record_kind_value_28_returns_none() {
    let result = unknown_record_kind_value(28);
    assert_eq!(
        result, None,
        "unknown_record_kind_value(28) must return None (28 is known)"
    );
}

// =============================================================================
// B52: is_known_record_kind(29) returns true
// =============================================================================

#[test]
fn is_known_record_kind_29_returns_true() {
    assert!(
        is_known_record_kind(29),
        "kind 29 must be recognized as AskTimedOut"
    );
}

// =============================================================================
// B52-ext: is_known_record_kind(0xFFFF) returns false
// =============================================================================

#[test]
fn is_known_record_kind_0xffff_returns_false() {
    assert!(
        !is_known_record_kind(0xFFFF),
        "kind 0xFFFF must NOT be recognized as a known record kind"
    );
}

// =============================================================================
// B53: validate_kind_family(MAGIC_JOURNAL_EVENT, 29) returns Ok
// =============================================================================

#[test]
fn validate_kind_family_journal_event_29_returns_ok() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 29);
    assert!(
        result.is_ok(),
        "kind 29 must be accepted for MAGIC_JOURNAL_EVENT, got {:?}",
        result
    );
}

// =============================================================================
// Boundary tests: validate_kind_family for various magic/kind combinations
// =============================================================================

#[test]
fn validate_kind_family_accepts_kind_10_with_journal_magic() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 10);
    assert!(
        result.is_ok(),
        "kind 10 (RunAccepted) must be accepted for MAGIC_JOURNAL_EVENT, got {:?}",
        result
    );
}

#[test]
fn validate_kind_family_accepts_kind_27_with_journal_magic() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 27);
    assert!(
        result.is_ok(),
        "kind 27 (RunAnswered) must be accepted for MAGIC_JOURNAL_EVENT, got {:?}",
        result
    );
}

#[test]
fn validate_kind_family_accepts_kind_29_with_journal_magic() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 29);
    assert!(
        result.is_ok(),
        "kind 29 (AskTimedOut) must be accepted for MAGIC_JOURNAL_EVENT, got {:?}",
        result
    );
}

#[test]
fn validate_kind_family_rejects_kind_9_with_journal_magic() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 9);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind 9 must be rejected (below journal range), got {:?}",
        result
    );
}

#[test]
fn validate_kind_family_rejects_kind_0_with_journal_magic() {
    let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 0);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind 0 must be rejected for any magic, got {:?}",
        result
    );
}

#[test]
fn validate_kind_family_rejects_unknown_magic() {
    let result = validate_kind_family(0xDEAD_BEEF, 28);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "unknown magic must reject even known kind 28, got {:?}",
        result
    );
}

// =============================================================================
// is_known_record_kind boundary tests
// =============================================================================

#[test]
fn is_known_record_kind_0_returns_false() {
    assert!(!is_known_record_kind(0));
}

#[test]
fn is_known_record_kind_1_returns_true() {
    assert!(is_known_record_kind(1));
}

#[test]
fn is_known_record_kind_3_returns_true() {
    assert!(is_known_record_kind(3));
}

#[test]
fn is_known_record_kind_10_returns_true() {
    assert!(is_known_record_kind(10));
}

#[test]
fn is_known_record_kind_27_returns_true() {
    assert!(is_known_record_kind(27));
}

#[test]
fn is_known_record_kind_30_returns_true() {
    assert!(is_known_record_kind(30));
}

#[test]
fn is_known_record_kind_40_returns_true() {
    assert!(is_known_record_kind(40));
}

#[test]
fn is_known_record_kind_50_returns_true() {
    assert!(is_known_record_kind(50));
}

#[test]
fn is_known_record_kind_51_returns_false() {
    assert!(!is_known_record_kind(51));
}

// =============================================================================
// unknown_record_kind_value: coherence with is_known_record_kind
// =============================================================================

#[test]
fn unknown_record_kind_value_29_returns_none() {
    let result = unknown_record_kind_value(29);
    assert_eq!(result, None);
}

#[test]
fn unknown_record_kind_value_0_returns_some_0() {
    let result = unknown_record_kind_value(0);
    assert_eq!(result, Some(0));
}

#[test]
fn unknown_record_kind_value_known_returns_none_for_kind_1() {
    let result = unknown_record_kind_value(1);
    assert_eq!(result, None);
}

// =============================================================================
// validate_known_kind: rejects unknown kinds
// =============================================================================

#[test]
fn validate_known_kind_accepts_kind_29() {
    let result = validate_known_kind(29);
    assert!(
        result.is_ok(),
        "validate_known_kind must accept AskTimedOut kind 29, got {:?}",
        result
    );
}

#[test]
fn validate_known_kind_accepts_kind_10() {
    let result = validate_known_kind(10);
    assert!(result.is_ok(), "validate_known_kind(10) must return Ok");
}

// =============================================================================
// Regression: kind 28 does not weaken key invariants
// =============================================================================

#[test]
fn kind_28_does_not_affect_known_kinds_elsewhere() {
    // Ensure adding kind 28 didn't accidentally allow adjacent un-admitted kinds
    assert!(!is_known_record_kind(4));
    assert!(!is_known_record_kind(5));
    assert!(!is_known_record_kind(9));
    // kind 31 (WaitResolved) is admitted by RE-009; keep verifying other reserved/unknown kinds stay rejected.
    assert!(!is_known_record_kind(39));
    assert!(!is_known_record_kind(41));
    assert!(!is_known_record_kind(49));
}
