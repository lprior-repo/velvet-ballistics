use vstd::prelude::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-queue-intent-verus-007.
// Bound to MRWE6 seam names exported from vb_runtime/vb_storage mrwe6_seams.
// Residual support boundary: queue capacity and Fjall commit are modeled;
// bridge/Kani evidence calls production classifiers.

pub enum Mrwe6EventClassView { Scheduled, Resolution, Unrelated }
pub enum Mrwe6IntentKindView { None, PutPending, RemovePending }

pub open spec fn seam_required_intent(class: Mrwe6EventClassView) -> Mrwe6IntentKindView {
    match class {
        Mrwe6EventClassView::Scheduled => Mrwe6IntentKindView::PutPending,
        Mrwe6EventClassView::Resolution => Mrwe6IntentKindView::RemovePending,
        Mrwe6EventClassView::Unrelated => Mrwe6IntentKindView::None,
    }
}

pub open spec fn valid_queue_item(class: Mrwe6EventClassView, intent: Mrwe6IntentKindView) -> bool {
    intent == seam_required_intent(class)
}

pub proof fn queued_schedule_requires_put_pending(intent: Mrwe6IntentKindView)
    requires valid_queue_item(Mrwe6EventClassView::Scheduled, intent),
    ensures intent == Mrwe6IntentKindView::PutPending, intent != Mrwe6IntentKindView::None,
{
}

pub proof fn queued_resolution_requires_remove_pending(intent: Mrwe6IntentKindView)
    requires valid_queue_item(Mrwe6EventClassView::Resolution, intent),
    ensures intent == Mrwe6IntentKindView::RemovePending, intent != Mrwe6IntentKindView::None,
{
}

} // verus!
