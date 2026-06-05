#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-atomic-index-flux-003.
//! Bound to crate::mrwe6_seams production classifiers. Residual support
//! boundary: Flux refines the seam-view atom; Rust bridge tests call actual
//! JournalEvent constructors and seam functions.

use crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind};
use flux_rs::attrs::*;

#[refined_by(kind: int)]
pub enum Mrwe6ScheduleAtom {
    #[variant(Mrwe6ScheduleAtom[0])]
    EventOnly,
    #[variant(Mrwe6ScheduleAtom[1])]
    EventAndIndex,
}

#[sig(fn(atom: Mrwe6ScheduleAtom{v: v == 1}) -> bool[true])]
pub fn scheduled_atom_has_index(atom: Mrwe6ScheduleAtom) -> bool {
    match atom {
        Mrwe6ScheduleAtom::EventAndIndex => true,
        Mrwe6ScheduleAtom::EventOnly => false,
    }
}

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
