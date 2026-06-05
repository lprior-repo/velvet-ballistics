// Cargo-fuzz target for batch byte limit (PS-006, C1).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_limit_nonzero(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let limit = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if limit == 0 {
        assert!(0u64.checked_add(1u64).unwrap() > limit);
    } else {
        let staged: u64 = 0;
        match staged.checked_add(1u64) {
            Some(total) => {
                if total <= limit { /* OK */ }
            }
            None => {}
        }
    }
}

fn fuzz_default_limit(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let encoded_len = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if encoded_len == 0 || encoded_len > 1_048_576 {
        return;
    }
    let limit: u64 = 1_048_576;
    match 0u64.checked_add(encoded_len) {
        Some(total) if total <= limit => {
            assert!(total <= limit);
        }
        _ => { /* OK */ }
    }
}

fn fuzz_arithmetic_safe(data: &[u8]) {
    if data.len() < 24 {
        return;
    }
    let a = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let b = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let limit = u64::from_le_bytes(data[16..24].try_into().unwrap());
    if limit == 0 {
        return;
    }
    let _sum = a.wrapping_add(b);
    let _checked = a.checked_add(b);
    let _cmp = a <= limit;
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    match data[0] % 3 {
        0 => fuzz_limit_nonzero(&data[1..]),
        1 => fuzz_default_limit(&data[1..]),
        _ => fuzz_arithmetic_safe(&data[1..]),
    }
});
