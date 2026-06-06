use vstd::prelude::*;

mod vb_mrwe6_kernel_binding;
use vb_mrwe6_kernel_binding::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-completion-policy-verus-019.
// Production seam: vb_storage::mrwe6_seams::mrwe6_resolution_commit_decision
// in crates/vb_storage/src/journal/append.rs:262-285. Residual support
// boundary: key equality and commit success are finite inputs; Kani/bridge
// evidence calls production JournalEvent classifiers.

pub open spec fn marker_present_after_resolution(is_resolution: bool, same_key: bool, commit_success: bool) -> bool {
    !(is_resolution && same_key && commit_success)
}

pub proof fn successful_same_key_resolution_removes_marker()
    ensures
        spec_resolution_commit_decision_from_facts(true, true, true) == Mrwe6ResolutionCommitDecisionView::CommittedAndMarkerRemoved,
        marker_present_after_resolution(true, true, true) == false,
{
}

pub proof fn failed_same_key_resolution_retains_marker()
    ensures
        spec_resolution_commit_decision_from_facts(true, true, false) == Mrwe6ResolutionCommitDecisionView::CommitFailedMarkerRetained,
        marker_present_after_resolution(true, true, false) == true,
{
}

} // verus!
