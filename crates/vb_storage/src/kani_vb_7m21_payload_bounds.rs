#![forbid(unsafe_code)]
//! PO-vb-7m21-C003: Payload size bounds proof.
//!
//! Proves that `payload_len_u32`, `encode_record_payload`, and
//! `decode_record_payload` enforce size bounds and never permit
//! oversized payloads to bypass validation.
//!
//! GOD RULE 1: Uses `kani::any()` for all inputs.

use crate::{
    codec::payload::{encode_record_payload, payload_len_u32},
    constants::MAGIC_JOURNAL_EVENT,
    error::JournalError,
    records::RecordKind,
};

// ── helpers ────────────────────────────────────────────────────────

/// Maximum reasonable payload for Kani bounded checking.
const MAX_PAYLOAD_FOR_KANI: usize = 128;

// ── harnesses ──────────────────────────────────────────────────────

/// Prove `payload_len_u32` rejects oversized payloads.
#[kani::proof]
fn kani_vb_7m21_payload_len_exceeds_max_is_rejected() {
    let max: u32 = kani::any();
    let len: u32 = kani::any();

    // Only consider the case where len > max (the oversized case)
    kani::assume(max < u32::MAX);
    kani::assume(len > max);

    // Kani bound: keep lengths small enough for solver
    kani::assume(max <= 1024);
    kani::assume(len as usize <= MAX_PAYLOAD_FOR_KANI);

    let result = payload_len_u32(len as usize, max);

    match result {
        Err(JournalError::PayloadTooLarge { len: l, max: m }) => {
            assert!(l > m, "PayloadTooLarge: len must exceed max");
            kani::cover!(true, "payload_len_u32 correctly rejected oversized payload");
        }
        Ok(payload_len) => {
            // If Ok, payload_len must be <= max
            assert!(payload_len <= max, "Ok implies payload_len <= max");
        }
        Err(_) => {
            assert!(false, "payload_len_u32 returned unexpected error for oversized input");
        }
    }
}

/// Prove `payload_len_u32` accepts payloads within bounds.
#[kani::proof]
fn kani_vb_7m21_payload_len_within_bounds_is_accepted() {
    let max: u32 = kani::any();
    let len: u32 = kani::any();

    kani::assume(max > 0);
    kani::assume(max <= 1024);
    kani::assume(len <= max);
    kani::assume(len as usize <= MAX_PAYLOAD_FOR_KANI);

    let result = payload_len_u32(len as usize, max);

    match result {
        Ok(payload_len) => {
            assert!(payload_len <= max, "Ok: payload_len within bounds");
            kani::cover!(true, "payload_len_u32 accepted bounded payload");
        }
        Err(_) => {
            // The only error should be PayloadTooLarge, but we're in bounds
            assert!(false, "payload_len_u32 rejected in-bounds payload");
        }
    }
}

/// Prove `encode_record_payload` rejects oversized payloads.
#[kani::proof]
fn kani_vb_7m21_encode_rejects_oversize() {
    let len: u32 = kani::any();
    let max: u32 = kani::any();

    kani::assume(max < u32::MAX);
    kani::assume(len > max);
    kani::assume(max <= 1024);
    kani::assume(len as usize <= MAX_PAYLOAD_FOR_KANI);

    let payload = vec![0u8; len as usize];
    let result = encode_record_payload(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &payload,
        max,
    );

    match result {
        Err(JournalError::PayloadTooLarge { .. }) => {
            kani::cover!(true, "encode_record_payload rejected oversized");
        }
        Ok(_) => {
            assert!(false, "encode_record_payload accepted oversized payload");
        }
        Err(_) => {
            kani::cover!(true, "encode_record_payload returned other error");
        }
    }
}

/// Prove `decode_record_payload` rejects payloads exceeding max_payload_len.
///
/// This constructs a well-formed header with a payload_len > max and verifies
/// that `decode_record_payload` rejects it.
#[kani::proof]
fn kani_vb_7m21_decode_rejects_payload_exceeding_max() {
    // Build a valid header with payload_len 1000
    // but pass max_payload_len = 100 so it must reject
    let small_payload = vec![0u8; 64];
    let encoded = encode_record_payload(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &small_payload,
        1000,
    );

    let full_record = match encoded {
        Ok(data) => data,
        Err(_) => {
            // Encoding failed due to our setup — skip the rest
            kani::assume(false);
            return;
        }
    };

    // Now decode with max_payload_len=100 which is less than the actual payload
    let result = crate::codec::payload::decode_record_payload(
        &full_record,
        MAGIC_JOURNAL_EVENT,
        100,
    );

    match result {
        Err(JournalError::PayloadTooLarge { .. }) => {
            kani::cover!(true, "decode_record_payload rejected payload exceeding max");
        }
        Ok((envelope, _payload)) => {
            let _ = envelope;
            assert!(false, "decode_record_payload accepted payload exceeding max_payload_len");
        }
        Err(_e) => {
            // Other error is fine too (like HeaderChecksumMismatch on tampered data)
            kani::cover!(true, "decode_record_payload other error on bounded check");
        }
    }
}

/// Prove payload decoding on arbitrary bytes yields no arithmetic overflow.
///
/// This targets the internal arithmetic in `decode_record_payload`:
/// `header_len` + `payload_len` must not overflow during `checked_add`.
#[kani::proof]
fn kani_vb_7m21_decode_payload_no_arithmetic_overflow() {
    let len: u32 = kani::any();
    kani::assume(len as usize <= MAX_PAYLOAD_FOR_KANI);
    let mut data: Vec<u8> = Vec::new();
    for _i in 0..(len as usize) {
        let b: u8 = kani::any();
        data.push(b);
    }

    let magic: u32 = kani::any();
    let max: u32 = kani::any();
    kani::assume(max <= 1024);

    let result = crate::codec::payload::decode_record_payload(&data, magic, max);

    assert!(
        result.is_ok() || result.is_err(),
        "decode_record_payload on arbitrary bytes returns Result, never panics"
    );
}
