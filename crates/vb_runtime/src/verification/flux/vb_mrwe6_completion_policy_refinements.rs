#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-completion-policy-flux-021.
//! Bound to crate::mrwe6_seams production resolution classifiers. Residual
//! support boundary: Flux refines the seam-view atom; Kani/bridge evidence calls
//! production JournalEvent resolution helpers.

use crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind};
use flux_rs::attrs::*;

#[refined_by(kind: int)]
pub enum Mrwe6ResolutionAtom {
    #[variant(Mrwe6ResolutionAtom[0])]
    EventOnly,
    #[variant(Mrwe6ResolutionAtom[1])]
    EventAndRemovePending,
}

#[sig(fn(atom: Mrwe6ResolutionAtom{v: v == 1}) -> bool[true])]
pub fn resolution_atom_removes_pending(atom: Mrwe6ResolutionAtom) -> bool {
    match atom {
        Mrwe6ResolutionAtom::EventAndRemovePending => true,
        Mrwe6ResolutionAtom::EventOnly => false,
    }
}

pub fn resolution_atom_from_production_seam(
    class: Mrwe6EventClass,
    required: Mrwe6IntentKind,
) -> Mrwe6ResolutionAtom {
    match (class, required) {
        (Mrwe6EventClass::Resolution, Mrwe6IntentKind::RemovePending) => {
            Mrwe6ResolutionAtom::EventAndRemovePending
        }
        _ => Mrwe6ResolutionAtom::EventOnly,
    }
}
