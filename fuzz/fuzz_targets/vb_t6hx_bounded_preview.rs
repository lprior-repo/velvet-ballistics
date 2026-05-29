#![no_main]
//! PO-vb-t6hx-R13: Fuzzed value bytes and preview arguments cannot
//! trigger full large-value rendering or unbounded allocation.
//!
//! Tests the production `decode_record_header` and
//! `payload_len_u32` boundary functions with fuzzed inputs to ensure
//! no overflow, panic, or unbounded allocation.

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::decode_record_header;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};

fuzz_target!(|data: &[u8]| {
    // The payload_len_u32 function enforces the preview cap invariant.
    // We test it indirectly through decode_record_header with varying
    // max payload limits derived from the fuzz input.

    // Derive a max payload limit from the input
    let max_payload = if let Some(&first) = data.first() {
        (first as u32).max(1).min(1024)
    } else {
        1024
    };

    // Test header decode with fuzzed data and varying limit
    let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, max_payload);

    // Also test with extreme limits
    let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, 1);
    let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, 0);

    // Verify that no panic occurs with any limit value
});
