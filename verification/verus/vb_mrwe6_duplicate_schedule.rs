use vstd::prelude::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-duplicate-verus-013.
// Production seam: vb_storage::mrwe6_seams::mrwe6_duplicate_retry_decision
// in crates/vb_storage/src/journal/append.rs:207-227. Residual support
// boundary: equality and marker presence are finite inputs; Kani/bridge tests
// call the classifier with real JournalEvent values.

pub enum Mrwe6DuplicateRetryDecisionView { IdempotentEqualRetry, DivergentDuplicateConflict, MissingExpectedIndexState }

pub open spec fn seam_duplicate_decision(equal_payload: bool, index_marker_present: bool, is_schedule: bool) -> Mrwe6DuplicateRetryDecisionView {
    if !equal_payload { Mrwe6DuplicateRetryDecisionView::DivergentDuplicateConflict }
    else if !is_schedule || index_marker_present { Mrwe6DuplicateRetryDecisionView::IdempotentEqualRetry }
    else { Mrwe6DuplicateRetryDecisionView::MissingExpectedIndexState }
}

pub proof fn divergent_duplicate_never_idempotent(index_marker_present: bool, is_schedule: bool)
    ensures seam_duplicate_decision(false, index_marker_present, is_schedule) == Mrwe6DuplicateRetryDecisionView::DivergentDuplicateConflict,
{
}

pub proof fn equal_scheduled_duplicate_with_marker_is_idempotent()
    ensures seam_duplicate_decision(true, true, true) == Mrwe6DuplicateRetryDecisionView::IdempotentEqualRetry,
{
}

} // verus!
