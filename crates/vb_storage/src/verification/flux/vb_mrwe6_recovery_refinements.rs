#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-recovery-reliance-flux-027.
//! Bound to crate::mrwe6_seams::Mrwe6RecoveryOutcome. Residual support boundary:
//! Flux refines the seam-view outcome; Kani/bridge evidence calls the production
//! recovery classifier with JournalEvent values.

use crate::mrwe6_seams::Mrwe6RecoveryOutcome;
use flux_rs::attrs::*;

#[refined_by(kind: int)]
pub enum Mrwe6RecoveryView {
    #[variant(Mrwe6RecoveryView[0])]
    PendingInventory,
    #[variant(Mrwe6RecoveryView[1])]
    ParityDefect,
    #[variant(Mrwe6RecoveryView[2])]
    LegacyFallback,
}

#[sig(fn(view: Mrwe6RecoveryView{v: v == 1}) -> bool[true])]
pub fn non_legacy_mismatch_is_defect(view: Mrwe6RecoveryView) -> bool {
    match view {
        Mrwe6RecoveryView::ParityDefect => true,
        Mrwe6RecoveryView::PendingInventory | Mrwe6RecoveryView::LegacyFallback => false,
    }
}

pub fn recovery_view_from_production_seam(outcome: Mrwe6RecoveryOutcome) -> Mrwe6RecoveryView {
    match outcome {
        Mrwe6RecoveryOutcome::PendingInventory => Mrwe6RecoveryView::PendingInventory,
        Mrwe6RecoveryOutcome::ResolvedNoPending | Mrwe6RecoveryOutcome::ParityDefect => {
            Mrwe6RecoveryView::ParityDefect
        }
        Mrwe6RecoveryOutcome::LegacyFallback => Mrwe6RecoveryView::LegacyFallback,
    }
}
