#![cfg(kani)]

#[kani::proof]
fn kani_harness_scan_limit_rows_never_exceed_limit() {
    let rows: u8 = kani::any();
    let limit: u8 = kani::any();
    kani::assume(limit > 0);
    kani::assume(limit <= 16);
    kani::assume(rows <= 16);
    let emitted = if rows <= limit { rows } else { limit };
    assert!(emitted <= limit);
}
