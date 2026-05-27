#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-missing-journal-kani
use crate::kani_vb_om21_model::{Metadata, Mode, Outcome, classify};

#[kani::proof]
fn vb_om21_missing_journal_harness() {
    let max_seq: u64 = kani::any();
    let outcome = classify(Mode::RecoveryRequiresJournal, false, max_seq, Metadata::Missing);
    assert!(matches!(outcome, Outcome::MissingJournal));
}
