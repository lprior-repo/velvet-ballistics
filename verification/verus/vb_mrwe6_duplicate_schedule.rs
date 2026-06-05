use vstd::prelude::*;

mod vb_mrwe6_kernel_binding;
use vb_mrwe6_kernel_binding::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-duplicate-verus-013.
// Production seam: vb_storage::mrwe6_seams::mrwe6_duplicate_retry_decision
// in crates/vb_storage/src/journal/append.rs:207-227. Residual support
// boundary: equality and marker presence are finite inputs; Kani/bridge tests
// call the classifier with real JournalEvent values.

pub proof fn divergent_duplicate_never_idempotent(index_marker_present: bool, retry_class: Mrwe6EventClassView)
    ensures spec_duplicate_retry_decision_from_facts(false, retry_class, index_marker_present) == Mrwe6DuplicateRetryDecisionView::DivergentDuplicateConflict,
{
}

pub proof fn equal_scheduled_duplicate_with_marker_is_idempotent()
    ensures spec_duplicate_retry_decision_from_facts(true, Mrwe6EventClassView::Scheduled, true) == Mrwe6DuplicateRetryDecisionView::IdempotentEqualRetry,
{
}

pub proof fn equal_scheduled_duplicate_without_marker_is_missing_state()
    ensures spec_duplicate_retry_decision_from_facts(true, Mrwe6EventClassView::Scheduled, false) == Mrwe6DuplicateRetryDecisionView::MissingExpectedIndexState,
{
}

pub proof fn equal_resolution_duplicate_is_unsupported(index_marker_present: bool)
    ensures spec_duplicate_retry_decision_from_facts(true, Mrwe6EventClassView::Resolution, index_marker_present) == Mrwe6DuplicateRetryDecisionView::UnsupportedDuplicateClassRejected,
{
}

pub proof fn equal_unrelated_duplicate_is_unsupported(index_marker_present: bool)
    ensures spec_duplicate_retry_decision_from_facts(true, Mrwe6EventClassView::Unrelated, index_marker_present) == Mrwe6DuplicateRetryDecisionView::UnsupportedDuplicateClassRejected,
{
}

} // verus!
