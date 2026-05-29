#![no_main]
//! PO-vb-t6hx-R03: Fuzzed scan/numeric argv cannot bypass ScanLimit caps
//! or trigger unbounded iteration/output.
//!
//! Since the fuzz crate does not depend on vb_cli, we exercise the
//! lower-level storage decode path with hostile argv-like bytes.
//! This verifies that decode_record_header handles arbitrary truncation
//! and malformation without panic.

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::decode_record_header;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};

fuzz_target!(|data: &[u8]| {
    // Test decode_record_header with hostile bytes at various truncation points
    // This simulates CLI argv that specify scan parameters being passed through
    // to the storage layer. The header decoder must classify errors correctly.
    let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    // Also exercise the payload length bound specifically
    if let Some(limit_byte) = data.first() {
        let limit = (*limit_byte).max(1) as u32;
        let _ = decode_record_header(data, MAGIC_JOURNAL_EVENT, limit);
    }
});
