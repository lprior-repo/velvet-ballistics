#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-single-event-tail-kani
use crate::kani_vb_om21_model::{Outcome, tail_after_max};

#[kani::proof]
fn vb_om21_single_event_tail_harness() {
    let outcome = tail_after_max(0);
    assert!(matches!(outcome, Outcome::Ok { tail } if tail == 1));
}
