#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-006 / PRE-002.
/// Bridge model for hydrate_events_preconditions(events) and
/// hydrate_dimensions_positive(step_count, slot_count).
pub open spec fn production_hydrate_events_preconditions(events_len: int) -> bool {
    events_len > 0
}

pub open spec fn production_hydrate_dimensions_positive(step_count: int, slot_count: int) -> bool {
    step_count > 0 && slot_count > 0
}

pub open spec fn valid_hydrate_events_preconditions(events_len: int, step_count: int, slot_count: int) -> bool {
    production_hydrate_events_preconditions(events_len)
        && production_hydrate_dimensions_positive(step_count, slot_count)
        && step_count <= 65535
        && slot_count <= 65535
}

pub proof fn proof_events_success_requires_nonempty_positive_bounded(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, 0 < step_count <= 65535, 0 < slot_count <= 65535,
    ensures
        valid_hydrate_events_preconditions(events_len, step_count, slot_count),
        production_hydrate_events_preconditions(events_len),
        production_hydrate_dimensions_positive(step_count, slot_count),
{}

pub proof fn proof_empty_events_rejected(step_count: int, slot_count: int)
    ensures !production_hydrate_events_preconditions(0), !valid_hydrate_events_preconditions(0, step_count, slot_count),
{}

pub proof fn proof_nonpositive_dimensions_rejected(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, step_count <= 0 || slot_count <= 0,
    ensures !valid_hydrate_events_preconditions(events_len, step_count, slot_count),
{}

}
