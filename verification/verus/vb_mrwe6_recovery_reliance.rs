use vstd::prelude::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-recovery-reliance-verus-025.
// Production seam: vb_storage::mrwe6_seams::mrwe6_recovery_outcome in
// crates/vb_storage/src/journal/append.rs:298-335. Residual support boundary:
// event shape/key equality are finite booleans; Kani/bridge evidence calls the
// actual recovery classifier.

pub enum Mrwe6RecoveryOutcomeView { PendingInventory, ResolvedNoPending, ParityDefect, LegacyFallback }

pub open spec fn seam_recovery_outcome(scheduled_event_is_schedule: bool, has_resolution: bool, marker_present: bool, legacy_profile: bool, same_key_resolution: bool) -> Mrwe6RecoveryOutcomeView {
    if !scheduled_event_is_schedule { Mrwe6RecoveryOutcomeView::ParityDefect }
    else if has_resolution {
        if same_key_resolution { Mrwe6RecoveryOutcomeView::ResolvedNoPending } else { Mrwe6RecoveryOutcomeView::ParityDefect }
    } else if marker_present { Mrwe6RecoveryOutcomeView::PendingInventory }
    else if legacy_profile { Mrwe6RecoveryOutcomeView::LegacyFallback }
    else { Mrwe6RecoveryOutcomeView::ParityDefect }
}

pub proof fn non_legacy_missing_marker_is_defect()
    ensures seam_recovery_outcome(true, false, false, false, false) == Mrwe6RecoveryOutcomeView::ParityDefect,
{
}

pub proof fn valid_pending_inventory_requires_marker_when_no_resolution(legacy_profile: bool)
    ensures seam_recovery_outcome(true, false, true, legacy_profile, false) == Mrwe6RecoveryOutcomeView::PendingInventory,
{
}

pub proof fn same_key_resolution_is_not_pending_inventory(marker_present: bool, legacy_profile: bool)
    ensures seam_recovery_outcome(true, true, marker_present, legacy_profile, true) == Mrwe6RecoveryOutcomeView::ResolvedNoPending,
{
}

} // verus!
