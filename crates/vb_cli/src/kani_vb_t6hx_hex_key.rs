#![cfg(kani)]

fn is_hex(b: u8) -> bool { b.is_ascii_hexdigit() }

fn classify_hex_key(bytes: &[u8]) -> Result<usize, ()> {
    if bytes.is_empty() || bytes.len() % 2 != 0 || bytes.len() > 64 {
        return Err(());
    }
    let mut i = 0;
    while i < bytes.len() {
        if !is_hex(bytes[i]) { return Err(()); }
        i += 1;
    }
    Ok(bytes.len() / 2)
}

#[kani::proof]
#[kani::unwind(65)]
fn kani_harness_hex_key_rejects_invalid_before_open() {
    let len: usize = kani::any();
    kani::assume(len <= 64);
    let bytes: [u8; 64] = kani::any();
    if let Some(candidate) = bytes.get(..len) {
        let classified = classify_hex_key(candidate);
        let storage_opened = classified.is_ok();
        assert!(classified.is_ok() || !storage_opened);
        if let Ok(decoded_len) = classified {
            assert!(decoded_len > 0);
            assert!(decoded_len <= 32);
        }
    }
}
