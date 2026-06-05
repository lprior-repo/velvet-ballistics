use vstd::prelude::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-completion-policy-verus-019.
// Production seam: vb_storage::mrwe6_seams::mrwe6_resolution_commit_decision
// in crates/vb_storage/src/journal/append.rs:262-285. Residual support
// boundary: key equality and commit success are finite inputs; Kani/bridge
// evidence calls production JournalEvent classifiers.

pub enum Mrwe6ResolutionCommitDecisionView { CommittedAndMarkerRemoved, CommitFailedMarkerRetained, MismatchedResolutionRejected, NonResolutionRejected }

pub open spec fn seam_resolution_decision(is_resolution: bool, same_key: bool, commit_success: bool) -> Mrwe6ResolutionCommitDecisionView {
    if !is_resolution { Mrwe6ResolutionCommitDecisionView::NonResolutionRejected }
    else if !same_key { Mrwe6ResolutionCommitDecisionView::MismatchedResolutionRejected }
    else if commit_success { Mrwe6ResolutionCommitDecisionView::CommittedAndMarkerRemoved }
    else { Mrwe6ResolutionCommitDecisionView::CommitFailedMarkerRetained }
}

pub open spec fn marker_present_after_resolution(is_resolution: bool, same_key: bool, commit_success: bool) -> bool {
    !(is_resolution && same_key && commit_success)
}

pub proof fn successful_same_key_resolution_removes_marker()
    ensures
        seam_resolution_decision(true, true, true) == Mrwe6ResolutionCommitDecisionView::CommittedAndMarkerRemoved,
        marker_present_after_resolution(true, true, true) == false,
{
}

pub proof fn failed_same_key_resolution_retains_marker()
    ensures
        seam_resolution_decision(true, true, false) == Mrwe6ResolutionCommitDecisionView::CommitFailedMarkerRetained,
        marker_present_after_resolution(true, true, false) == true,
{
}

} // verus!
