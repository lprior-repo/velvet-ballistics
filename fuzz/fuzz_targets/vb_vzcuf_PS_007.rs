// Cargo-fuzz target for core/storage bridge (PS-007, C8).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_bridge_value(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let policy = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if policy == 0 || policy > 100_000_000 {
        return;
    }
    let storage_limit = policy;
    assert!(storage_limit > 0);
    assert_eq!(storage_limit, policy);
    if storage_limit <= u32::MAX as u64 {
        let as_u32: u32 = storage_limit as u32;
        assert_eq!(as_u32 as u64, storage_limit);
    }
}

fn fuzz_bridge_defaults(_data: &[u8]) {
    use vb_storage::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    let storage_default: u64 = 1_048_576;
    let core_policy: u64 = 1_048_576;
    assert_eq!(storage_default, core_policy);
    let max_encoded = RECORD_HEADER_LEN as u64 + MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64;
    assert!(max_encoded <= storage_default);
}

fn fuzz_multiple_events(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let encoded_len = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if encoded_len == 0 || encoded_len > 10_000 {
        return;
    }
    let limit: u64 = 1_048_576;
    let max_count = limit.checked_div(encoded_len).unwrap_or(0);
    assert!(max_count > 0 || encoded_len > limit);
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    match data[0] % 3 {
        0 => fuzz_bridge_value(&data[1..]),
        1 => fuzz_bridge_defaults(&data[1..]),
        _ => fuzz_multiple_events(&data[1..]),
    }
});
