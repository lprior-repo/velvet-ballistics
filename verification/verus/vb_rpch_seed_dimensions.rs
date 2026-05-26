#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-002 / INV-003.
/// Bridge model for State-11 production proof surfaces in
/// crates/vb_storage/src/recovery/replay/summary.rs:
/// recovery_dimension_count_from_index, recovery_seed_dimensions_positive, and
/// recovery_observed_dimension_is_positive.
pub open spec fn fits_u16(n: int) -> bool { 0 <= n && n <= 65535 }

pub open spec fn positive_dimension(n: int) -> bool { 0 < n && fits_u16(n) }

pub open spec fn successful_seed_dimensions(events_len: int, step_count: int, slot_count: int) -> bool {
    events_len > 0 && positive_dimension(step_count) && positive_dimension(slot_count)
}

pub open spec fn overflow_typed_error(step_count: int, slot_count: int) -> bool {
    step_count > 65535 || slot_count > 65535
}

pub open spec fn production_dimension_count_from_index(max_index: int) -> int {
    max_index + 1
}

pub open spec fn production_observed_dimension_is_positive(max_index_present: bool, count: int) -> bool {
    if max_index_present { count > 0 } else { count == 0 }
}

pub open spec fn production_seed_dimensions_positive(step_count: int, slot_count: int) -> bool {
    step_count > 0 && slot_count > 0
}

pub proof fn proof_checked_index_derives_positive_count(max_index: int)
    requires 0 <= max_index < 65535,
    ensures
        0 < production_dimension_count_from_index(max_index) <= 65535,
        production_observed_dimension_is_positive(true, production_dimension_count_from_index(max_index)),
{}

pub proof fn proof_success_constructor_derives_positive_bounded(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, 0 < step_count <= 65535, 0 < slot_count <= 65535,
    ensures
        successful_seed_dimensions(events_len, step_count, slot_count),
        production_seed_dimensions_positive(step_count, slot_count),
{}

pub proof fn proof_zero_dimension_cannot_succeed(events_len: int, step_count: int, slot_count: int)
    requires events_len > 0, step_count <= 0 || slot_count <= 0,
    ensures !successful_seed_dimensions(events_len, step_count, slot_count),
{}

pub proof fn proof_u16_overflow_maps_to_error(step_count: int, slot_count: int)
    requires overflow_typed_error(step_count, slot_count),
    ensures !positive_dimension(step_count) || !positive_dimension(slot_count),
{}

}
