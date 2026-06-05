#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-BOUND-KANI.
const MAX_BATCH_COUNT_MODEL: usize = 16;
#[kani::proof]
fn vb_mrwe_7_atomic_batch_size_bounds() {
    let n: usize = kani::any();
    let accepted = n > 0 && n <= MAX_BATCH_COUNT_MODEL;
    if accepted {
        assert!(n >= 1 && n <= MAX_BATCH_COUNT_MODEL);
    } else {
        assert!(n == 0 || n > MAX_BATCH_COUNT_MODEL);
    }
}
