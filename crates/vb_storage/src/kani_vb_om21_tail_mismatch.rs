#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-tail-mismatch-kani
use crate::kani_vb_om21_model::{Metadata, Mode, Outcome, classify};

#[kani::proof]
fn vb_om21_tail_mismatch_harness() {
    let max_seq: u64 = kani::any();
    kani::assume(max_seq < u64::MAX);
    let outcome = classify(Mode::RecoveryRequiresJournal, true, max_seq, Metadata::Below);
    kani::assert(matches!(outcome, Outcome::TailMismatch),
        "tail-mismatch: declared tail metadata below reconstructed tail is rejected as TailMismatch");
    kani::cover!(max_seq == 0, "tail-mismatch-zero-tail");
}
