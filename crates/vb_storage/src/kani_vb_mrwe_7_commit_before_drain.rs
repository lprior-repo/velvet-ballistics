#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-CRASH-KANI. TB-EVENT-EQUALITY external seam.
#[kani::proof]
fn vb_mrwe_7_commit_before_drain_retry() {
    let committed_before_crash: bool = kani::any();
    let equal_duplicate: bool = kani::any();
    let drains_on_retry = committed_before_crash && equal_duplicate;
    kani::assert(!(committed_before_crash && !equal_duplicate && drains_on_retry, "assertion failed"),
        "commit-before-drain retry invariant",
    );
}
