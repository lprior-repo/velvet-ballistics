#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
//! Miri-testable harnesses proving decode functions handle truncated/malformed data safely.
//!
//! These tests verify that no UB occurs on malformed input - all decode functions
//! must return errors rather than panic or cause UB.

#![forbid(unsafe_code)]

use std::panic::catch_unwind;

use crate::JournalEvent;
use crate::{
    codec::{decode_record, decode_record_header, verify_digest_match},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES},
    error::JournalError,
    records::RecordKind,
    types::EventSeq,
};
use vb_core::{RunId, WorkflowDigest};

fn panic_free_decode_header(input: &[u8]) -> Result<(), JournalError> {
    let result = catch_unwind(|| {
        decode_record_header(input, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
    });
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            panic!("decode_record_header panicked on input len {}", input.len());
        }
    }
}

fn panic_free_decode_record(input: &[u8]) -> Result<(), JournalError> {
    let result = catch_unwind(|| {
        decode_record::<JournalEvent>(input, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
    });
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            panic!("decode_record panicked on input len {}", input.len());
        }
    }
}

fn panic_free_verify_digest(payload: &[u8], digest: [u8; 32]) -> Result<(), JournalError> {
    let result = catch_unwind(|| verify_digest_match(payload, digest));
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            panic!("verify_digest_match panicked");
        }
    }
}

#[test]
fn decode_record_header_on_empty_input_returns_error() {
    let result = panic_free_decode_header(&[]);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "empty input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_on_single_byte_returns_error() {
    let result = panic_free_decode_header(&[0x00]);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "single byte must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_on_30_bytes_returns_unexpected_eof() {
    let partial = vec![0u8; 30];
    let result = panic_free_decode_header(&partial);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "30-byte partial input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_on_59_bytes_returns_unexpected_eof() {
    let partial = vec![0u8; RECORD_HEADER_BYTES - 1];
    let result = panic_free_decode_header(&partial);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "59-byte partial input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_valid_header_truncated_payload_returns_error() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };
    let encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    let full_payload_len = encoded.len() - RECORD_HEADER_BYTES;
    assert!(
        full_payload_len > 10,
        "payload should be larger than 10 bytes"
    );
    let truncated = &encoded[..RECORD_HEADER_BYTES + 10];
    let result = panic_free_decode_record(truncated);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "truncated payload must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_corrupted_postcard_returns_error_not_ub() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([4; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    for i in RECORD_HEADER_BYTES..encoded.len() {
        encoded[i] = 0xFF;
    }
    let result = panic_free_decode_record(&encoded);
    assert!(
        !result.is_ok(),
        "corrupted postcard must not succeed - no UB allowed, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_all_ff_payload_returns_error_not_ub() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    for i in RECORD_HEADER_BYTES..encoded.len() {
        encoded[i] = 0xFF;
    }
    encoded[RECORD_HEADER_BYTES] = 0xFF;
    let result = panic_free_decode_record(&encoded);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "all-0xFF payload digest mismatch must yield PayloadDigestMismatch, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_corrupted_header_crc_returns_error_not_ub() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([6; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    if let Some(byte) = encoded.get_mut(crate::constants::CRC_OFFSET) {
        *byte = byte.wrapping_add(1);
    }
    let result = panic_free_decode_record(&encoded);
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "corrupt CRC must yield HeaderChecksumMismatch, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_truncated_to_exactly_header_returns_error() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([7; 32]),
    };
    let encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    let header_only = &encoded[..RECORD_HEADER_BYTES];
    let result = panic_free_decode_record(header_only);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "header-only (no payload) must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn verify_digest_match_on_mismatched_digest_returns_error() {
    let payload = b"hello world";
    let wrong_digest: [u8; 32] = blake3::hash(b"something completely different").into();
    let result = panic_free_verify_digest(payload, wrong_digest);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "mismatched digest must yield PayloadDigestMismatch, got {:?}",
        result
    );
}

#[test]
fn verify_digest_match_on_empty_payload_mismatched_returns_error() {
    let payload: &[u8] = &[];
    let wrong_digest: [u8; 32] = blake3::hash(b"not empty").into();
    let result = panic_free_verify_digest(payload, wrong_digest);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "empty payload with wrong digest must yield PayloadDigestMismatch, got {:?}",
        result
    );
}

#[test]
fn verify_digest_match_on_empty_payload_correct_returns_ok() {
    let payload: &[u8] = &[];
    let correct_digest: [u8; 32] = blake3::hash(&[]).into();
    let result = panic_free_verify_digest(payload, correct_digest);
    assert!(
        result.is_ok(),
        "correct digest on empty payload should pass"
    );
}

#[test]
fn decode_record_on_zero_byte_input_returns_error() {
    let result = panic_free_decode_record(&[]);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "zero-byte input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_single_zero_byte_returns_error() {
    let result = panic_free_decode_record(&[0x00]);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "single zero byte must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_corrupted_magic_returns_error_not_ub() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([8; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    encoded[0] = 0xFF;
    encoded[1] = 0xFF;
    encoded[2] = 0xFF;
    encoded[3] = 0xFF;
    let result = panic_free_decode_record(&encoded);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "corrupt magic must yield BadMagic, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_truncated_at_header_boundary_returns_unexpected_eof() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    let truncated_len = RECORD_HEADER_BYTES + 1;
    let truncated = &encoded[..truncated_len];
    let result = panic_free_decode_record(truncated);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "truncated at header+1 must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_valid_header_and_partial_payload_returns_unexpected_eof() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([9; 32]),
    };
    let full = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    let header_plus_half = &full[..RECORD_HEADER_BYTES + 5];
    let result = panic_free_decode_record(header_plus_half);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "header + partial payload must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_payload_digest_mismatch_returns_error_not_ub() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([10; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    let last_payload_byte = encoded.len() - 1;
    encoded[last_payload_byte] = encoded[last_payload_byte].wrapping_add(1);
    let result = panic_free_decode_record(&encoded);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "digest mismatch must yield PayloadDigestMismatch, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_on_valid_encoded_header_returns_ok() {
    let payload = b"test payload";
    let header = crate::codec::encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        42,
        payload,
        1024,
    )
    .expect("encode_record_header must succeed");
    let result = panic_free_decode_header(&header);
    assert!(
        result.is_ok(),
        "valid header must decode successfully, got {:?}",
        result
    );
}

#[test]
fn decode_record_on_wrong_magic_returns_error() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([11; 32]),
    };
    let mut encoded = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode must succeed");
    encoded[0] = 0xFF;
    encoded[1] = 0xFF;
    encoded[2] = 0xFF;
    encoded[3] = 0xFF;
    let result = panic_free_decode_record(&encoded);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "wrong magic must yield BadMagic, got {:?}",
        result
    );
}
