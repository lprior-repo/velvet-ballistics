use vstd::prelude::*;

mod vb_mrwe6_kernel_binding;
use vb_mrwe6_kernel_binding::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-queue-intent-verus-007.
// Bound to MRWE6 seam names exported from vb_runtime/vb_storage mrwe6_seams.
// Residual support boundary: queue capacity and Fjall commit are modeled;
// bridge/Kani evidence calls production classifiers.

pub open spec fn valid_queue_item(class: Mrwe6EventClassView, intent: Mrwe6IntentKindView) -> bool {
    spec_intent_kind_matches_event_class(class, intent)
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
