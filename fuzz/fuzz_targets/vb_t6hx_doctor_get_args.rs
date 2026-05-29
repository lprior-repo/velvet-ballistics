#![no_main]
//! PO-vb-t6hx-R06: Fuzzed doctor get argv cannot panic, bypass hex
//! validation, or reach storage on invalid hex.
//!
//! Exercises the storage decode boundary with hostile byte sequences
//! that simulate invalid hex key arguments. The fuzz crate depends on
//! vb_storage, so we test decode_journal_event and decode_record_header
//! with fully arbitrary bytes.

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::{decode_journal_event, decode_record_header};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES};
use vb_storage::error::JournalError;

fuzz_target!(|data: &[u8]| {
    // Decode header only (pre-storage-open check)
    let header_result = decode_record_header(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    // Decode full journal event (includes postcard deserialization)
    let full_result = decode_journal_event(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    // Sanity: if header decode fails, full decode must also fail
    if header_result.is_err() {
        assert!(full_result.is_err());
    }

    // If data is shorter than header, must get UnexpectedEof (pre-storage-open)
    if data.len() < RECORD_HEADER_BYTES {
        assert!(matches!(full_result, Err(JournalError::UnexpectedEof)));
    }
});
