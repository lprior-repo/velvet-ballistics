#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-queue-intent-flux-009.
//! Bound to crate::mrwe6_seams production class/intent classifiers. Residual
//! support boundary: Flux refines the seam-view intent; bridge tests exercise
//! production JournalEvent values.

use crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind};
use flux_rs::attrs::*;

#[refined_by(kind: int)]
pub enum Mrwe6QueuedIntent {
    #[variant(Mrwe6QueuedIntent[0])]
    None,
    #[variant(Mrwe6QueuedIntent[1])]
    PutPending,
    #[variant(Mrwe6QueuedIntent[2])]
    RemovePending,
}

#[sig(fn(intent: Mrwe6QueuedIntent{v: v > 0}) -> bool[true])]
pub fn queued_relevant_event_has_intent(intent: Mrwe6QueuedIntent) -> bool {
    match intent {
        Mrwe6QueuedIntent::PutPending | Mrwe6QueuedIntent::RemovePending => true,
        Mrwe6QueuedIntent::None => false,
    }
}

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
