use vstd::prelude::*;

mod vb_mrwe6_kernel_binding;
use vb_mrwe6_kernel_binding::*;

verus! {

// Verus artifact for obl-vb-mrwe-6-recovery-reliance-verus-025.
// Production seam: vb_storage::mrwe6_seams::mrwe6_recovery_outcome in
// crates/vb_storage/src/journal/append.rs:298-335. Residual support boundary:
// event shape/key equality are finite booleans; Kani/bridge evidence calls the
// actual recovery classifier.

pub proof fn non_legacy_missing_marker_is_defect()
    ensures spec_recovery_outcome_from_facts(true, false, false, false, false) == Mrwe6RecoveryOutcomeView::ParityDefect,
{
}

pub proof fn valid_pending_inventory_requires_marker_when_no_resolution(legacy_profile: bool)
    ensures spec_recovery_outcome_from_facts(true, false, false, true, legacy_profile) == Mrwe6RecoveryOutcomeView::PendingInventory,
{
}

pub proof fn same_key_resolution_is_not_pending_inventory(marker_present: bool, legacy_profile: bool)
    ensures spec_recovery_outcome_from_facts(true, true, true, marker_present, legacy_profile) == Mrwe6RecoveryOutcomeView::ResolvedNoPending,
{
}

} // verus!
