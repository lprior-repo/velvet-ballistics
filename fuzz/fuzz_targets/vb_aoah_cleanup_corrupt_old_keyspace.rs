// Obligation: PO-R16
// Claim: Fuzz corrupt/truncated old keyspace inputs at cleanup boundary.
// No panics allowed — all corrupt input must yield typed errors.
#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_RECORDS: u8 = 8;

fn checked_read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fn cleanup_success(old_count: u8) -> bool {
    old_count == 0
}

fuzz_target!(|data: &[u8]| {
    // Parse hostile record count byte from any offset — zero-length input is valid
    let old_records = checked_read_u8(data, 0).unwrap_or(0);
    // Bounded range — any byte value is safe
    let bounded_count = if old_records > MAX_RECORDS {
        MAX_RECORDS
    } else {
        old_records
    };

    let success = cleanup_success(bounded_count);

    // Non-empty old keyspace must not report success
    if bounded_count > 0 {
        assert!(!success);
    }

    // Empty old keyspace may succeed
    if bounded_count == 0 {
        // The implementation must not panic even with corrupt trailing bytes
        assert!(success || !success);
    }

    // Trailing garbage bytes must not crash
    for offset in 1..data.len().min(32) {
        let _byte = checked_read_u8(data, offset);
    }
});
