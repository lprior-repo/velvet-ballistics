#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-atomic-index-flux-003.
//! Bound to crate::mrwe6_seams production classifiers. Residual support
//! boundary: Flux refines the seam-view atom; Rust bridge tests call actual
//! JournalEvent constructors and seam functions.

use crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind};
#[flux_rs::refined_by(kind: int)]
pub enum Mrwe6ScheduleAtom {
    #[flux_rs::variant(Mrwe6ScheduleAtom[0])]
    EventOnly,
    #[flux_rs::variant(Mrwe6ScheduleAtom[1])]
    EventAndIndex,
}

#[flux_rs::sig(fn(atom: Mrwe6ScheduleAtom{v: v == 1}) -> bool[true])]
pub fn scheduled_atom_has_index(atom: Mrwe6ScheduleAtom) -> bool {
    match atom {
        Mrwe6ScheduleAtom::EventAndIndex => true,
        Mrwe6ScheduleAtom::EventOnly => false,
    }
}

#[flux_rs::sig(fn(atom: Mrwe6ScheduleAtom{v: v == 0}) -> bool[false])]
pub fn invalid_scheduled_event_only_rejected(atom: Mrwe6ScheduleAtom) -> bool {
    match atom {
        Mrwe6ScheduleAtom::EventOnly => false,
        Mrwe6ScheduleAtom::EventAndIndex => true,
    }
}

#[flux_rs::sig(fn(Mrwe6EventClass, Mrwe6IntentKind) -> Mrwe6ScheduleAtom)]
pub fn schedule_atom_from_production_seam(
    class: Mrwe6EventClass,
    required: Mrwe6IntentKind,
) -> Mrwe6ScheduleAtom {
    match (class, required) {
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending) => {
            Mrwe6ScheduleAtom::EventAndIndex
        }
        _ => Mrwe6ScheduleAtom::EventOnly,
    }
}

#[cfg(feature = "vb-mrwe6-flux-negative-probes")]
#[flux_rs::sig(fn() -> bool[true])]
pub fn negative_probe_event_only_schedule_is_rejected() -> bool {
    invalid_scheduled_event_only_rejected(Mrwe6ScheduleAtom::EventOnly)
}
