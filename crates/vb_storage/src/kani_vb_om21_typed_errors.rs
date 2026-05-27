#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-typed-errors-kani
use crate::kani_vb_om21_model::{any_metadata, any_mode, classify, Metadata, Outcome};

#[kani::proof]
fn vb_om21_typed_errors_harness() {
    let mode = any_mode(kani::any());
    let seen: bool = kani::any();
    let max_seq: u64 = kani::any();
    let metadata = any_metadata(kani::any());
    let outcome = classify(mode, seen, max_seq, metadata);
    kani::assert(!(!seen && matches!(mode, crate::kani_vb_om21_model::Mode::RecoveryRequiresJournal))
        || matches!(outcome, Outcome::MissingJournal),
        "typed-errors: no run_event keys seen in recovery mode yields MissingJournal");
    kani::assert(!(seen && max_seq < u64::MAX && metadata == Metadata::Below)
        || matches!(outcome, Outcome::TailMismatch),
        "typed-errors: seen below declared tail yields TailMismatch");
    kani::assert(!(seen && max_seq == u64::MAX)
        || matches!(outcome, Outcome::TailOverflow),
        "typed-errors: seen max u64 sequence yields TailOverflow");
    kani::cover!(matches!(outcome, Outcome::MissingJournal), "typed-errors-missing-journal");
    kani::cover!(matches!(outcome, Outcome::TailMismatch), "typed-errors-tail-mismatch");
    kani::cover!(matches!(outcome, Outcome::TailOverflow), "typed-errors-tail-overflow");
}
