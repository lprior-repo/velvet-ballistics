#![forbid(unsafe_code)]

use vb_core::RunId;

use crate::recovery::types::{RecoveryError, RecoveryFrameSeed, RecoveryResult};

/// Production proof surface for turning a maximum zero-based dimension into a count.
pub fn recovery_dimension_count_from_index(
    max_index: Option<u16>,
    run: RunId,
) -> RecoveryResult<u16> {
    max_index
        .map(|value| {
            value
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .map_or(Ok(0), |result| result)
}

/// Production proof surface for successful non-empty/evidence-bearing seed dimensions.
#[must_use]
pub const fn recovery_seed_dimensions_positive(seed: &RecoveryFrameSeed) -> bool {
    seed.step_count > 0 && seed.slot_count > 0
}

/// Production proof surface for an observed dimension requiring positive count.
#[must_use]
pub const fn recovery_observed_dimension_is_positive(max_index: Option<u16>, count: u16) -> bool {
    match max_index {
        Some(_) => count > 0,
        None => count == 0,
    }
}
