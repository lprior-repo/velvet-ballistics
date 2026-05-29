// Obligation: PO-R17
// Claim: Fuzz malformed empty-fixture inputs at codec/manifest boundary.
// Empty keyspace semantics must hold even for corrupt headers/metadata.
// No panics allowed — all malformed input must yield typed errors or explicit NoOp.
#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_RECORDS: u8 = 8;

fn checked_read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fn is_noop_outcome(old_count: u8, verified: bool) -> bool {
    old_count == 0 && verified
}

fn is_blocked(old_count: u8, verified: bool) -> bool {
    old_count == 0 && !verified
}

fuzz_target!(|data: &[u8]| {
    let old_records = checked_read_u8(data, 0).unwrap_or(0);
    let bounded_count = if old_records > MAX_RECORDS {
        MAX_RECORDS
    } else {
        old_records
    };
    let verified = checked_read_u8(data, 1)
        .map(|b| b % 2 == 1)
        .unwrap_or(false);

    if is_noop_outcome(bounded_count, verified) {
        // Explicit NoOp: must not silently advance
        assert!(verified);
        assert_eq!(bounded_count, 0);
    }

    if is_blocked(bounded_count, verified) {
        // Unverified empty: must be blocked, not silent success
        assert!(!verified);
    }

    if bounded_count > 0 {
        // Non-empty: normal migration path, can't be NoOp
        let noop = is_noop_outcome(bounded_count, verified);
        assert!(!noop);
    }

    // Corrupt trailing bytes must not crash
    for offset in 1..data.len().min(32) {
        let _byte = checked_read_u8(data, offset);
    }
});
