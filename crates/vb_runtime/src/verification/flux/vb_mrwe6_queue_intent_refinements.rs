#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-queue-intent-flux-009.
//! Bound to crate::mrwe6_seams production class/intent classifiers. Residual
//! support boundary: Flux refines the seam-view intent; bridge tests exercise
//! production JournalEvent values.

use crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind};
#[flux_rs::refined_by(kind: int)]
pub enum Mrwe6QueuedIntent {
    #[flux_rs::variant(Mrwe6QueuedIntent[0])]
    None,
    #[flux_rs::variant(Mrwe6QueuedIntent[1])]
    PutPending,
    #[flux_rs::variant(Mrwe6QueuedIntent[2])]
    RemovePending,
}

#[flux_rs::sig(fn(intent: Mrwe6QueuedIntent{v: v > 0}) -> bool[true])]
pub fn queued_relevant_event_has_intent(intent: Mrwe6QueuedIntent) -> bool {
    match intent {
        Mrwe6QueuedIntent::PutPending | Mrwe6QueuedIntent::RemovePending => true,
        Mrwe6QueuedIntent::None => false,
    }
}

#[flux_rs::sig(fn(intent: Mrwe6QueuedIntent{v: v == 0}) -> bool[false])]
pub fn invalid_queued_relevant_event_without_intent_rejected(intent: Mrwe6QueuedIntent) -> bool {
    match intent {
        Mrwe6QueuedIntent::None => false,
        Mrwe6QueuedIntent::PutPending | Mrwe6QueuedIntent::RemovePending => true,
    }
}

#[flux_rs::sig(fn(Mrwe6EventClass, Mrwe6IntentKind) -> Mrwe6QueuedIntent)]
pub fn queued_intent_from_production_seam(
    class: Mrwe6EventClass,
    required: Mrwe6IntentKind,
) -> Mrwe6QueuedIntent {
    match (class, required) {
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending) => Mrwe6QueuedIntent::PutPending,
        (Mrwe6EventClass::Resolution, Mrwe6IntentKind::RemovePending) => {
            Mrwe6QueuedIntent::RemovePending
        }
        _ => Mrwe6QueuedIntent::None,
    }
}

#[cfg(feature = "vb-mrwe6-flux-negative-probes")]
#[flux_rs::sig(fn() -> bool[true])]
pub fn negative_probe_queued_relevant_event_without_intent_is_rejected() -> bool {
    invalid_queued_relevant_event_without_intent_rejected(Mrwe6QueuedIntent::None)
}
