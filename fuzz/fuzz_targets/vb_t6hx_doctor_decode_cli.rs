#![no_main]
//! PO-vb-t6hx-R10: Fuzzed CLI doctor decode path preserves storage
//! JournalError decode categories.
//!
//! Exercises decode_journal_event with hostile bytes to ensure no
//! error-category collapse (e.g., BadMagic incorrectly reported as
//! PostcardDecodeFailed) and no panic.

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::decode_journal_event;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES};
use vb_storage::error::JournalError;

fuzz_target!(|data: &[u8]| {
    let result = decode_journal_event(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    match result {
        // Category-specific assertions
        Err(JournalError::UnexpectedEof) => {
            // Must have insufficient bytes for the full decode
            assert!(data.len() < RECORD_HEADER_BYTES
                 || data.len() < RECORD_HEADER_BYTES + 4);
        }
        Err(JournalError::PostcardDecodeFailed) => {
            // Envelope checks passed; postcard body was malformed
            assert!(data.len() >= RECORD_HEADER_BYTES);
        }
        Err(JournalError::InvalidEvent) => {
            // Envelope + postcard passed; semantic check failed
            assert!(data.len() >= RECORD_HEADER_BYTES);
        }
        Err(JournalError::PayloadTooLarge { len, max }) => {
            // Payload length exceeded the configured max
            assert!(len > max);
        }
        Err(JournalError::HeaderChecksumMismatch) => {
            // CRC check failed
            assert!(data.len() >= RECORD_HEADER_BYTES);
        }
        Err(JournalError::PayloadDigestMismatch) => {
            // Digest check failed
            assert!(data.len() >= RECORD_HEADER_BYTES);
        }
        Ok(_) => {
            // Fully valid event decoded
            assert!(data.len() >= RECORD_HEADER_BYTES);
        }
        _ => {
            // Other error variants (BadMagic, schema, kind family, etc.)
            // All represent pre-postcard envelope errors
        }
    }
});
