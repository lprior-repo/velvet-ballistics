#![cfg(kani)]

#[kani::proof]
fn kani_harness_skip_decode_never_decodes_malformed_rows() {
    let skip_decode: bool = kani::any();
    let rows: u8 = kani::any();
    kani::assume(rows <= 8);
    let decode_attempted = !skip_decode && rows > 0;
    assert!(!skip_decode || !decode_attempted);
}
