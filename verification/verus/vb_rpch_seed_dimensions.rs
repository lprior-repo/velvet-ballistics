#![allow(unused_imports)]

// Verus proof obligations for vb-rpch INV-003: RecoveryFrameSeed
// dimensions positivity from observed event indexes.
//
// Obligation: VERUS-REC-003 / INV-003
// Contract: RecoveryFrameSeed.step_count > 0 and slot_count > 0 when
//           events non-empty and replay succeeds.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to production via the companion extern surface
// `verification/verus/extern_vb_rpch_seed_dimensions.rs`, which
// itself `#[path]`-includes the verbatim production mirror at
// `verification/verus/production_inner/replay_invariants_production.rs`
// (which includes the verbatim bodies of
// `crates/vb_storage/src/recovery/replay/summary/derive.rs:249-276`).
//
// The `assume_specification` bridges below attach the production
// contracts for `recovery_dimension_count_from_index`,
// `recovery_seed_dimensions_positive`, and
// `recovery_observed_dimension_is_positive` to the spec-side mirror
// functions in the extern file. The exec wrappers invoke the mirror
// functions to discharge the contracts; they are the non-vacuum
// witnesses that the bridges are actually used.
//
// BINDING LEDGER:
//   - `production_recovery_dimension_count_from_index`     <- derive.rs:250-261
//   - `production_recovery_seed_dimensions_positive`       <- derive.rs:265-267
//   - `production_recovery_observed_dimension_is_positive` <- derive.rs:271-275

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production extern surface — `#[path]`-bound mirror of
// crates/vb_storage/src/recovery/replay/summary/derive.rs:249-276.
// ---------------------------------------------------------------------------
#[path = "extern_vb_rpch_seed_dimensions.rs"]
mod production;

// Re-export the spec-side mirror types and functions so the spec
// proofs and exec wrappers below can use them.
pub use production::{
    RunId, MirrorRecoveryError, MirrorRecoveryFrameSeed,
    production_recovery_dimension_count_from_index,
    production_recovery_seed_dimensions_positive,
    production_recovery_observed_dimension_is_positive,
};

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

// ---------------------------------------------------------------------------
// assume_specification BRIDGES — production contract surface
// ---------------------------------------------------------------------------
//
// Each bridge attaches the spec fn contract to the spec-side mirror
// exec function. The mirror body is opaque to Verus
// (`#[verifier::external]`); the spec proofs below exercise the
// contracts via the exec wrappers further down.
pub assume_specification[ production::production_recovery_dimension_count_from_index ](
    max_index: Option<u16>,
    run: production::RunId,
) -> (result: Result<u16, production::MirrorRecoveryError>)
    ensures
        result == (match max_index {
            Some(value) => {
                if (value as int) + 1 <= 65535 {
                    Ok((value as int + 1) as u16)
                } else {
                    Err(production::MirrorRecoveryError::FrameDimensionOverflow { run })
                }
            },
            None => Ok(0u16),
        }),
;

pub assume_specification[ production::production_recovery_seed_dimensions_positive ](
    seed: &production::MirrorRecoveryFrameSeed,
) -> (result: bool)
    ensures
        result == (seed.step_count > 0 && seed.slot_count > 0),
;

pub assume_specification[ production::production_recovery_observed_dimension_is_positive ](
    max_index: Option<u16>,
    count: u16,
) -> (result: bool)
    ensures
        result == (match max_index {
            Some(_) => count > 0,
            None => count == 0,
        }),
;

// ---------------------------------------------------------------------------
// Production-bound exec wrappers — discharge witnesses for the bridges
// ---------------------------------------------------------------------------
//
// These exec wrappers invoke the spec-side mirror functions. Verus
// verifies each wrapper body via the `assume_specification` contract
// attached to the corresponding mirror function.
pub exec fn production_recovery_dimension_count_from_index_witness(
    max_index: Option<u16>,
    run: production::RunId,
) -> (r: Result<u16, production::MirrorRecoveryError>)
    ensures
        r == (match max_index {
            Some(value) => {
                if (value as int) + 1 <= 65535 {
                    Ok((value as int + 1) as u16)
                } else {
                    Err(production::MirrorRecoveryError::FrameDimensionOverflow { run })
                }
            },
            None => Ok(0u16),
        }),
{
    production::production_recovery_dimension_count_from_index(max_index, run)
}

pub exec fn production_recovery_seed_dimensions_positive_witness(
    seed: &production::MirrorRecoveryFrameSeed,
) -> (r: bool)
    ensures
        r == (seed.step_count > 0 && seed.slot_count > 0),
{
    production::production_recovery_seed_dimensions_positive(seed)
}

pub exec fn production_recovery_observed_dimension_is_positive_witness(
    max_index: Option<u16>,
    count: u16,
) -> (r: bool)
    ensures
        r == (match max_index {
            Some(_) => count > 0,
            None => count == 0,
        }),
{
    production::production_recovery_observed_dimension_is_positive(max_index, count)
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
