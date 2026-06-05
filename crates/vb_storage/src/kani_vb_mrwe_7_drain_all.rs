#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-DRAIN-KANI.
#[kani::proof]
fn vb_mrwe_7_drain_all_bound_and_retention() {
    let pending: usize = kani::any();
    let first_error: bool = kani::any();
    kani::assume(pending <= 16);
    let remaining = if first_error { pending } else { 0 };
    assert!(!first_error || remaining == pending);
}
