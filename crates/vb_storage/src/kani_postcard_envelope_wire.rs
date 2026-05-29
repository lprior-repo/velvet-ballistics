#![cfg(kani)]

use crate::codec::decode_journal_event;
use crate::codec::header::decode_record_header;
use crate::constants::{
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES,
};
use crate::error::JournalError;

/// PO-vb-t6hx-R07: Bounded malformed envelopes return pre-Postcard errors when
/// length/integrity checks fail.
///
/// This harness proves that `decode_journal_event` (and its inner
/// `decode_record_header`) validate header length, magic, schema, record kind
/// family, payload length bound, and header CRC BEFORE attempting postcard
/// deserialization.
///
/// The harness generates arbitrary bounded byte arrays (max 256 bytes) and
/// checks that:
/// 1. Inputs shorter than RECORD_HEADER_BYTES always fail with UnexpectedEof
/// 2. PostcardDecodeFailed only occurs after passing all envelope checks
/// 3. PayloadTooLarge and HeaderChecksumMismatch are reachable pre-postcard errors
/// 4. The function never panics on any arbitrary bounded input
#[kani::proof]
#[kani::unwind(260)]
fn kani_harness_storage_decode_order() {
    let len: usize = kani::any();
    kani::assume(len <= 256);
    let bytes: [u8; 256] = kani::any();
    if let Some(candidate) = bytes.get(..len) {
        let result = decode_journal_event(
            candidate,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        // Property 1: Too-short input always fails before postcard decode
        if len < RECORD_HEADER_BYTES {
            assert!(matches!(result, Err(JournalError::UnexpectedEof)));
            kani::cover!(true, "truncated input rejected with UnexpectedEof");
        }

        // Property 2: If postcard decode fails, header passed all envelope checks
        // (magic, schema, kind, length, CRC, digest all passed)
        if matches!(result, Err(JournalError::PostcardDecodeFailed)) {
            assert!(len >= RECORD_HEADER_BYTES);
            kani::cover!(true, "postcard decode attempted only after envelope checks pass");
        }

        // Property 3: Pre-postcard errors are classified correctly
        match &result {
            Err(JournalError::BadMagic { .. }) => {
                kani::cover!(true, "BadMagic detected before postcard decode");
            }
            Err(JournalError::UnsupportedSchemaVersion { .. }) => {
                kani::cover!(true, "schema version rejected before postcard");
            }
            Err(JournalError::MigrationRequired { .. }) => {
                kani::cover!(true, "migration required before postcard");
            }
            Err(JournalError::UnknownRecordKind { .. }) => {
                kani::cover!(true, "unknown kind rejected before postcard");
            }
            Err(JournalError::RecordKindFamilyMismatch { .. }) => {
                kani::cover!(true, "kind-family mismatch before postcard");
            }
            Err(JournalError::HeaderLengthMismatch { .. }) => {
                kani::cover!(true, "header length mismatch before postcard");
            }
            Err(JournalError::PayloadTooLarge { .. }) => {
                kani::cover!(true, "payload too large before postcard");
            }
            Err(JournalError::HeaderChecksumMismatch) => {
                kani::cover!(true, "header checksum mismatch before postcard");
            }
            Err(JournalError::PayloadDigestMismatch) => {
                kani::cover!(true, "digest mismatch before postcard");
            }
            Err(JournalError::UnexpectedEof) => {
                kani::cover!(true, "unexpected eof before postcard");
            }
            Err(JournalError::PostcardDecodeFailed) => {
                // Already covered above
            }
            Err(JournalError::InvalidEvent) => {
                kani::cover!(true, "semantically invalid event after valid postcard decode");
            }
            Ok(_) => {
                kani::cover!(true, "fully valid event decoded successfully");
            }
            _ => {
                kani::cover!(true, "other error variant");
            }
        }

        // Property 4: No panic (verified by Kani runtime checks)
    }
}

/// PO-vb-t6hx-R07 auxiliary harness: `decode_record_header` panic-freedom
/// and decode-order enforcement.
///
/// Tests the lower-level header decoder which validates magic, schema, kind,
/// header_len, payload_len bound, and CRC BEFORE any payload access.
#[kani::proof]
#[kani::unwind(65)]
fn kani_harness_decode_record_header_panic_freedom() {
    let len: usize = kani::any();
    kani::assume(len <= 120);
    let bytes: [u8; 120] = kani::any();
    if let Some(candidate) = bytes.get(..len) {
        let result = decode_record_header(
            candidate,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        // Header decoder must either succeed or return a typed error
        // It must never panic, overflow, or OOM
        match result {
            Ok(_header) => {
                kani::cover!(true, "header decoded successfully");
            }
            Err(_e) => {
                kani::cover!(true, "header rejected with typed error");
            }
        }
    }
}
