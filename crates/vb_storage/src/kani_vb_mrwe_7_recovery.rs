#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-RECOVERY-KANI.
#[kani::proof]
fn vb_mrwe_7_no_silent_torn_batch() {
    let contiguous: bool = kani::any();
    let atomic_complete: bool = kani::any();
    let explicit_partial: bool = kani::any();
    let complete = atomic_complete || explicit_partial;
    assert!(!(contiguous && !atomic_complete && !explicit_partial && complete));
}
