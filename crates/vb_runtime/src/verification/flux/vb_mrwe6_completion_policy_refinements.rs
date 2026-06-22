/// Flux refinement artifact for obl-vb-mrwe-6-completion-policy-flux-021.
/// Bound to crate::mrwe6_seams production resolution classifiers. Residual
/// support boundary: Flux refines the seam-view atom; Kani/bridge evidence calls
/// production JournalEvent resolution helpers.
///
/// NOTE: `Mrwe6EventClass` and `Mrwe6IntentKind` are imported once at the crate
/// level (see `vb_mrwe6_atomic_index_refinements.rs`); the flux-rs crate-level
/// scan of `src/verification/flux/vb_mrwe6_*_refinements.rs` merges the three
/// files into a single module, so re-importing the same names here would cause
/// E0252 "name defined multiple times" errors. All references below use the
/// fully-qualified path `crate::mrwe6_seams::{Mrwe6EventClass, Mrwe6IntentKind}`.
#[flux_rs::refined_by(kind: int)]
pub enum Mrwe6ResolutionAtom {
    #[flux_rs::variant(Mrwe6ResolutionAtom[0])]
    EventOnly,
    #[flux_rs::variant(Mrwe6ResolutionAtom[1])]
    EventAndRemovePending,
}

#[flux_rs::sig(fn(atom: Mrwe6ResolutionAtom{v: v == 1}) -> bool[true])]
pub fn resolution_atom_removes_pending(atom: Mrwe6ResolutionAtom) -> bool {
    match atom {
        Mrwe6ResolutionAtom::EventAndRemovePending => true,
        Mrwe6ResolutionAtom::EventOnly => false,
    }
}

#[flux_rs::sig(fn(atom: Mrwe6ResolutionAtom{v: v == 0}) -> bool[false])]
pub fn invalid_resolution_event_only_rejected(atom: Mrwe6ResolutionAtom) -> bool {
    match atom {
        Mrwe6ResolutionAtom::EventOnly => false,
        Mrwe6ResolutionAtom::EventAndRemovePending => true,
    }
}

#[flux_rs::sig(fn(crate::mrwe6_seams::Mrwe6EventClass, crate::mrwe6_seams::Mrwe6IntentKind) -> Mrwe6ResolutionAtom)]
pub fn resolution_atom_from_production_seam(
    class: crate::mrwe6_seams::Mrwe6EventClass,
    required: crate::mrwe6_seams::Mrwe6IntentKind,
) -> Mrwe6ResolutionAtom {
    match (class, required) {
        (crate::mrwe6_seams::Mrwe6EventClass::Resolution, crate::mrwe6_seams::Mrwe6IntentKind::RemovePending) => {
            Mrwe6ResolutionAtom::EventAndRemovePending
        }
        _ => Mrwe6ResolutionAtom::EventOnly,
    }
}

#[cfg(feature = "vb-mrwe6-flux-negative-probes")]
#[flux_rs::sig(fn() -> bool[true])]
pub fn negative_probe_resolution_event_only_is_rejected() -> bool {
    invalid_resolution_event_only_rejected(Mrwe6ResolutionAtom::EventOnly)
}
