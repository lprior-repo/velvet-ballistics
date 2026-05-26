#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-003 / INV-004.
/// Bridge model for crates/vb_storage/src/recovery/types.rs
/// ActionReplayTracker::{has_completed, has_failed, is_resolved,
/// mark_completed, mark_failed}.
pub type ActionId = int;
pub type StepIdx = int;

pub open spec fn is_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx)) -> bool {
    completed.contains(key) || failed.contains(key)
}

pub open spec fn production_has_completed(completed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx)) -> bool {
    completed.contains(key)
}

pub open spec fn production_has_failed(failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx)) -> bool {
    failed.contains(key)
}

pub open spec fn production_is_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx)) -> bool {
    production_has_completed(completed, key) || production_has_failed(failed, key)
}

pub proof fn proof_mark_completed_makes_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx))
    ensures production_is_resolved(completed.insert(key), failed, key),
{}

pub proof fn proof_mark_failed_makes_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx))
    ensures production_is_resolved(completed, failed.insert(key), key),
{}

pub proof fn proof_resolution_monotone_under_completed_insert(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, old_key: (ActionId, StepIdx), new_key: (ActionId, StepIdx))
    requires production_is_resolved(completed, failed, old_key),
    ensures production_is_resolved(completed.insert(new_key), failed, old_key),
{}

pub proof fn proof_resolution_monotone_under_failed_insert(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, old_key: (ActionId, StepIdx), new_key: (ActionId, StepIdx))
    requires production_is_resolved(completed, failed, old_key),
    ensures production_is_resolved(completed, failed.insert(new_key), old_key),
{}

pub proof fn proof_resolution_equivalence_to_production_surface(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx))
    ensures is_resolved(completed, failed, key) == production_is_resolved(completed, failed, key),
{}

}
