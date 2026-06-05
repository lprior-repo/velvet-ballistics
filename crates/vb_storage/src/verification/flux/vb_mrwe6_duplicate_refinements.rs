#![cfg(flux)]

//! Flux refinement artifact for obl-vb-mrwe-6-duplicate-flux-015.
//! Bound to crate::mrwe6_seams::Mrwe6DuplicateRetryDecision. Residual support
//! boundary: Flux refines the seam-view decision; Kani/bridge evidence calls the
//! production classifier with JournalEvent values.

use crate::mrwe6_seams::Mrwe6DuplicateRetryDecision;
use flux_rs::attrs::*;

#[refined_by(kind: int)]
pub enum Mrwe6DuplicateRetry {
    #[variant(Mrwe6DuplicateRetry[0])]
    EqualIdempotent,
    #[variant(Mrwe6DuplicateRetry[1])]
    DivergentConflict,
}

#[sig(fn(result: Mrwe6DuplicateRetry{v: v == 1}) -> bool[true])]
pub fn divergent_retry_is_conflict(result: Mrwe6DuplicateRetry) -> bool {
    match result {
        Mrwe6DuplicateRetry::DivergentConflict => true,
        Mrwe6DuplicateRetry::EqualIdempotent => false,
    }
}

pub fn duplicate_retry_from_production_seam(
    decision: Mrwe6DuplicateRetryDecision,
) -> Mrwe6DuplicateRetry {
    match decision {
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry => Mrwe6DuplicateRetry::EqualIdempotent,
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
        | Mrwe6DuplicateRetryDecision::MissingExpectedIndexState => {
            Mrwe6DuplicateRetry::DivergentConflict
        }
    }
}
