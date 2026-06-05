use vstd::prelude::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-atomic-index-verus-001.
// Production seam source refs: crates/vb_storage/src/mrwe6_seams.rs and
// crates/vb_storage/src/journal/append.rs:370-420. Residual support boundary:
// this proves the pure seam contract; bridge/Kani evidence calls Rust seams.

pub enum Mrwe6EventClassView { Scheduled, Resolution, Unrelated }
pub enum Mrwe6IntentKindView { None, PutPending, RemovePending }
pub enum Mrwe6CommitResultView { Success, Failure }

pub struct Mrwe6AtomView { pub event_staged: bool, pub index_staged: bool }

pub open spec fn seam_required_intent(class: Mrwe6EventClassView) -> Mrwe6IntentKindView {
    match class {
        Mrwe6EventClassView::Scheduled => Mrwe6IntentKindView::PutPending,
        Mrwe6EventClassView::Resolution => Mrwe6IntentKindView::RemovePending,
        Mrwe6EventClassView::Unrelated => Mrwe6IntentKindView::None,
    }
}

pub open spec fn scheduled_atom_from_seam(class: Mrwe6EventClassView, intent: Mrwe6IntentKindView) -> Mrwe6AtomView {
    Mrwe6AtomView { event_staged: class == Mrwe6EventClassView::Scheduled, index_staged: intent == Mrwe6IntentKindView::PutPending }
}

pub open spec fn committed_event(atom: Mrwe6AtomView, result: Mrwe6CommitResultView) -> bool {
    atom.event_staged && result == Mrwe6CommitResultView::Success
}

pub open spec fn committed_index(atom: Mrwe6AtomView, result: Mrwe6CommitResultView) -> bool {
    atom.index_staged && result == Mrwe6CommitResultView::Success
}

pub proof fn scheduled_seam_atom_commits_event_and_index_together(result: Mrwe6CommitResultView)
    ensures
        committed_event(scheduled_atom_from_seam(Mrwe6EventClassView::Scheduled, seam_required_intent(Mrwe6EventClassView::Scheduled)), result)
            == committed_index(scheduled_atom_from_seam(Mrwe6EventClassView::Scheduled, seam_required_intent(Mrwe6EventClassView::Scheduled)), result),
{
}

} // verus!
