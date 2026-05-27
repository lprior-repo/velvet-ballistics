#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-zero-tail-query-kani
use crate::kani_vb_om21_model::{Metadata, Mode, Outcome, classify};

#[kani::proof]
fn vb_om21_zero_tail_query_harness() {
    let outcome = classify(Mode::QueryAllowsEmpty, false, 0, Metadata::Missing);
    assert!(matches!(outcome, Outcome::Ok { tail } if tail == 1));
}
