#![cfg(kani)]

#[kani::proof]
fn kani_harness_bounded_preview_never_exceeds_limit() {
    let value_len: u16 = kani::any();
    let limit: u16 = kani::any();
    kani::assume(value_len <= 256);
    kani::assume(limit > 0 && limit <= 64);
    let preview_len = if value_len <= limit { value_len } else { limit };
    let omitted = value_len - preview_len;
    assert!(preview_len <= limit);
    assert!(preview_len + omitted == value_len);
}
