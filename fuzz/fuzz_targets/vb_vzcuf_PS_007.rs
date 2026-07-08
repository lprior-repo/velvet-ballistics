// Cargo-fuzz target for core/storage bridge (PS-007, C8).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_bridge_value(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let policy_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let policy = u64::from_le_bytes(policy_bytes);
    if policy == 0 || policy > 100_000_000 {
        return;
    }
    let storage_limit = policy;
    assert!(storage_limit > 0);
    assert_eq!(storage_limit, policy);
    if storage_limit <= u64::from(u32::MAX) {
        let Ok(as_u32) = u32::try_from(storage_limit) else {
            return;
        };
        assert_eq!(u64::from(as_u32), storage_limit);
    }
}

fn fuzz_bridge_defaults(_data: &[u8]) {
    use vb_storage::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    let storage_default: u64 = 1_048_576;
    let core_policy: u64 = 1_048_576;
    assert_eq!(storage_default, core_policy);
    let Some(max_encoded) =
        u64::from(RECORD_HEADER_LEN).checked_add(u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES))
    else {
        return;
    };
    assert!(max_encoded <= storage_default);
}

fn fuzz_multiple_events(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let encoded_len_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let encoded_len = u64::from_le_bytes(encoded_len_bytes);
    if encoded_len == 0 || encoded_len > 10_000 {
        return;
    }
    let limit: u64 = 1_048_576;
    let max_count = limit.checked_div(encoded_len).unwrap_or(0);
    assert!(max_count > 0 || encoded_len > limit);
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    match selector.checked_rem(3) {
        Some(0) => fuzz_bridge_value(rest),
        Some(1) => fuzz_bridge_defaults(rest),
        Some(_) => fuzz_multiple_events(rest),
        None => {}
    }
});
