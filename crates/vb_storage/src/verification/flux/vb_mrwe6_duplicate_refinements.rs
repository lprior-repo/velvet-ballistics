#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-duplicate-flux-015.
//! Bound to crate::mrwe6_seams::Mrwe6DuplicateRetryDecision. Residual support
//! boundary: Flux refines the seam-view decision; Kani/bridge evidence calls the
//! production classifier with JournalEvent values.

use crate::mrwe6_seams::Mrwe6DuplicateRetryDecision;
#[flux_rs::refined_by(kind: int)]
pub enum Mrwe6DuplicateRetry {
    #[flux_rs::variant(Mrwe6DuplicateRetry[0])]
    EqualIdempotent,
    #[flux_rs::variant(Mrwe6DuplicateRetry[1])]
    DivergentConflict,
    #[flux_rs::variant(Mrwe6DuplicateRetry[2])]
    MissingExpectedIndexState,
    #[flux_rs::variant(Mrwe6DuplicateRetry[3])]
    UnsupportedDuplicateClassRejected,
}

#[flux_rs::sig(fn(result: Mrwe6DuplicateRetry{v: v == 1}) -> bool[true])]
pub fn divergent_retry_is_conflict(result: Mrwe6DuplicateRetry) -> bool {
    match result {
        Mrwe6DuplicateRetry::DivergentConflict => true,
        Mrwe6DuplicateRetry::EqualIdempotent
        | Mrwe6DuplicateRetry::MissingExpectedIndexState
        | Mrwe6DuplicateRetry::UnsupportedDuplicateClassRejected => false,
    }
}

#[flux_rs::sig(fn(result: Mrwe6DuplicateRetry{v: v == 0}) -> bool[false])]
pub fn invalid_divergent_duplicate_success_rejected(result: Mrwe6DuplicateRetry) -> bool {
    match result {
        Mrwe6DuplicateRetry::EqualIdempotent => false,
        Mrwe6DuplicateRetry::DivergentConflict
        | Mrwe6DuplicateRetry::MissingExpectedIndexState
        | Mrwe6DuplicateRetry::UnsupportedDuplicateClassRejected => true,
    }
}

#[flux_rs::sig(fn(result: Mrwe6DuplicateRetry{v: v == 3}) -> bool[true])]
pub fn unsupported_retry_is_rejected(result: Mrwe6DuplicateRetry) -> bool {
    match result {
        Mrwe6DuplicateRetry::UnsupportedDuplicateClassRejected => true,
        Mrwe6DuplicateRetry::EqualIdempotent
        | Mrwe6DuplicateRetry::DivergentConflict
        | Mrwe6DuplicateRetry::MissingExpectedIndexState => false,
    }
}

#[flux_rs::sig(fn(result: Mrwe6DuplicateRetry{v: v == 2}) -> bool[true])]
pub fn missing_marker_retry_is_rejected(result: Mrwe6DuplicateRetry) -> bool {
    match result {
        Mrwe6DuplicateRetry::MissingExpectedIndexState => true,
        Mrwe6DuplicateRetry::EqualIdempotent
        | Mrwe6DuplicateRetry::DivergentConflict
        | Mrwe6DuplicateRetry::UnsupportedDuplicateClassRejected => false,
    }
}

#[flux_rs::sig(fn(Mrwe6DuplicateRetryDecision) -> Mrwe6DuplicateRetry)]
pub fn duplicate_retry_from_production_seam(
    decision: Mrwe6DuplicateRetryDecision,
) -> Mrwe6DuplicateRetry {
    match decision {
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry => Mrwe6DuplicateRetry::EqualIdempotent,
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict => {
            Mrwe6DuplicateRetry::DivergentConflict
        }
        Mrwe6DuplicateRetryDecision::MissingExpectedIndexState => {
            Mrwe6DuplicateRetry::MissingExpectedIndexState
        }
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected => {
            Mrwe6DuplicateRetry::UnsupportedDuplicateClassRejected
        }
    }
}

#[cfg(feature = "vb-mrwe6-flux-negative-probes")]
#[flux_rs::sig(fn() -> bool[true])]
pub fn negative_probe_divergent_duplicate_success_is_rejected() -> bool {
    invalid_divergent_duplicate_success_rejected(Mrwe6DuplicateRetry::EqualIdempotent)
}

#[cfg(feature = "vb-mrwe6-flux-negative-probes")]
#[flux_rs::sig(fn() -> bool[true])]
pub fn negative_probe_unsupported_duplicate_success_is_rejected() -> bool {
    unsupported_retry_is_rejected(Mrwe6DuplicateRetry::EqualIdempotent)
}
