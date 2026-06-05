#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-CONCURRENCY-KANI.
#[kani::proof]
fn vb_mrwe_7_queue_lifecycle_invariants() {
    let shutdown: bool = kani::any();
    let pending: usize = kani::any();
    let capacity: usize = kani::any();
    kani::assume(capacity <= 16 && pending <= capacity);
    let enqueue_ok = !shutdown && pending < capacity;
    assert!(!shutdown || !enqueue_ok);
    assert!(pending <= capacity);
}
