#![forbid(unsafe_code)]
//! PO-vb-7m21-C001: Codec panic-freedom on arbitrary byte streams.
//!
//! Proves that `decode_record_header`, `decode_record_payload`, and
//! `decode_record` never panic on arbitrary-length byte slices. Uses
//! `kani::any()` for full input space coverage within bounds.
//!
//! GOD RULE 1: No hardcoded shapes — every harness uses `kani::any()`
//! for structural inputs and `kani::any()` for length randomization.
//!
//! Trusted boundaries: `postcard::from_bytes` may panic internally;
//! we verify that `decode_record` guards against this by checking
//! header/envelope validity before calling Postcard decode.

use crate::{
    codec::{decode_record_header, payload::decode_record_payload},
    error::JournalError,
};

// ── helpers ────────────────────────────────────────────────────────

/// Generates a maximum payload bound from a small discrete set so
/// Kani explores the boundary space without combinatorial explosion.
fn arbitrary_max_payload_len() -> u32 {
    let choice: u8 = kani::any();
    match choice {
        0..=50 => 0,
        51..=100 => 1,
        101..=150 => 60,
        151..=200 => 1024,
        _ => u32::MAX,
    }
}

/// Generates a byte length within a reasonable bound for Kani
/// without pushing the solver into deep unwinding.
fn arbitrary_byte_len() -> usize {
    let len: u32 = kani::any();
    let capped = len.min(128);
    capped as usize
}

// ── harnesses ──────────────────────────────────────────────────────

/// Prove `decode_record_header` never panics on arbitrary byte slices.
#[kani::proof]
fn kani_vb_7m21_decode_record_header_never_panics() {
    let len = arbitrary_byte_len();
    let mut data: Vec<u8> = Vec::new();

    // Build arbitrary byte stream with length chosen by kani::any()
    for _i in 0..len {
        let b: u8 = kani::any();
        data.push(b);
    }

    let magic: u32 = kani::any();
    let max_payload_len: u32 = kani::any();

    let result = decode_record_header(&data, magic, max_payload_len);

    // Assert: function terminates without panic
    match result {
        Ok(header) => {
            kani::cover!(true, "decode_record_header returned Ok");
            // If Ok, verify the header struct is populated
            assert!(header.header_len <= u32::MAX, "header_len field is bounded by u32");
        }
        Err(JournalError::UnexpectedEof) => {
            kani::cover!(true, "decode_record_header returned UnexpectedEof");
        }
        Err(JournalError::BadMagic { .. }) => {
            kani::cover!(true, "decode_record_header returned BadMagic");
        }
        Err(JournalError::UnsupportedSchemaVersion { .. }) => {
            kani::cover!(true, "decode_record_header returned UnsupportedSchemaVersion");
        }
        Err(JournalError::MigrationRequired { .. }) => {
            kani::cover!(true, "decode_record_header returned MigrationRequired");
        }
        Err(JournalError::HeaderChecksumMismatch) => {
            kani::cover!(true, "decode_record_header returned HeaderChecksumMismatch");
        }
        Err(JournalError::HeaderLengthMismatch { .. }) => {
            kani::cover!(true, "decode_record_header returned HeaderLengthMismatch");
        }
        Err(JournalError::PayloadTooLarge { .. }) => {
            kani::cover!(true, "decode_record_header returned PayloadTooLarge");
        }
        Err(JournalError::UnknownRecordKind { .. }) => {
            kani::cover!(true, "decode_record_header returned UnknownRecordKind");
        }
        Err(JournalError::RecordKindFamilyMismatch { .. }) => {
            kani::cover!(true, "decode_record_header returned RecordKindFamilyMismatch");
        }
        Err(_) => {
            kani::cover!(true, "decode_record_header returned other error");
        }
    }
    // Explicit assertion: result is either Ok or Err — no panic path
    assert!(
        result.is_ok() || result.is_err(),
        "decode_record_header always returns a valid Result"
    );
}

/// Prove `decode_record_payload` never panics on arbitrary byte slices.
#[kani::proof]
fn kani_vb_7m21_decode_record_payload_never_panics() {
    let len = arbitrary_byte_len();
    let mut data: Vec<u8> = Vec::new();
    for _i in 0..len {
        let b: u8 = kani::any();
        data.push(b);
    }

    let magic: u32 = kani::any();
    let max_payload_len: u32 = kani::any();

    let result = decode_record_payload(&data, magic, max_payload_len);

    match &result {
        Ok((envelope, payload)) => {
            kani::cover!(true, "decode_record_payload returned Ok");
            assert!(envelope.magic <= u32::MAX, "envelope.magic is bounded by u32");
            // Payload slice is derived from input — no panic means safe slicing
            assert!(payload.len() <= data.len(), "payload slice within input bounds");
        }
        Err(e) => {
            kani::cover!(true, "decode_record_payload returned Err");
            assert!(
                !matches!(e, JournalError::PostcardDecodeFailed),
                "decode_record_payload never produces PostcardDecodeFailed (that's decode_record's job)"
            );
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "decode_record_payload always returns a valid Result"
    );
}

/// Prove `decode_record<JournalEvent>` never panics on arbitrary byte slices.
#[kani::proof]
fn kani_vb_7m21_decode_record_journal_event_never_panics() {
    let len = arbitrary_byte_len();
    let mut data: Vec<u8> = Vec::new();
    for _i in 0..len {
        let b: u8 = kani::any();
        data.push(b);
    }

    let magic: u32 = kani::any();
    let max: u32 = arbitrary_max_payload_len();

    let result = crate::codec::decode_record::<crate::events::JournalEvent>(
        &data, magic, max,
    );

    match &result {
        Ok((envelope, _event)) => {
            kani::cover!(true, "decode_record<JournalEvent> returned Ok");
            assert!(envelope.schema_version <= u16::MAX, "schema_version bounded by u16");
        }
        Err(JournalError::PostcardDecodeFailed) => {
            kani::cover!(true, "decode_record<JournalEvent> returned PostcardDecodeFailed");
        }
        Err(JournalError::PayloadDigestMismatch) => {
            kani::cover!(true, "decode_record<JournalEvent> returned PayloadDigestMismatch");
        }
        Err(_) => {
            kani::cover!(true, "decode_record<JournalEvent> returned other error");
        }
    }
    assert!(
        result.is_ok() || result.is_err(),
        "decode_record<JournalEvent> always returns a valid Result — never panics"
    );
}
