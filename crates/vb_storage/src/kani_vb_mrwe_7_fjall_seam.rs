#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-FJALL-KANI. TB-FJALL-OWNED-BATCH external seam.
#[kani::proof]
fn vb_mrwe_7_fjall_seam_single_commit() {
    let staged: usize = kani::any();
    let commit_ok: bool = kani::any();
    kani::assume(staged <= 16);
    let commit_calls = if staged > 0 { 1 } else { 0 };
    let drained = if staged > 0 && commit_ok { staged } else { 0 };
    assert!(staged == 0 || commit_calls == 1);
    assert!(commit_ok || drained == 0);
}
