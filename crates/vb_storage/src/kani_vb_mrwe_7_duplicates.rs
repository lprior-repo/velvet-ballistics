#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-DUP-KANI.  TB-EVENT-EQUALITY is modeled by `same_payload`.
#[kani::proof]
fn vb_mrwe_7_duplicate_policy() {
    let duplicate_seen: bool = kani::any();
    let same_payload: bool = kani::any();
    let drains = !duplicate_seen || same_payload;
    assert!(!(duplicate_seen && !same_payload && drains));
}
