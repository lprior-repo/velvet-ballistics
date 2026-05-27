#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-tail-overflow-kani
use crate::kani_vb_om21_model::{Outcome, tail_after_max};

#[kani::proof]
fn vb_om21_tail_overflow_harness() {
    let max_seq: u64 = kani::any();
    let outcome = tail_after_max(max_seq);
    kani::assert(max_seq != u64::MAX || matches!(outcome, Outcome::TailOverflow),
        "tail-overflow: u64::MAX yields typed TailOverflow, never wraps to zero");
    kani::assert(max_seq == u64::MAX || matches!(outcome, Outcome::Ok { tail } if tail == max_seq + 1),
        "tail-overflow: non-MAX sequence yields Ok successor tail+1");
    kani::cover!(max_seq == u64::MAX, "tail-overflow-edge");
    kani::cover!(max_seq == 0, "tail-overflow-zero-edge");
}
