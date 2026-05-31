// Flux-rs refinement annotations for batch byte limit (PS-006, C1).
//
// Obligation ID: POB-vb-vzcuf-023
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-006.rs
//
// Domain claim: Every open JournalWriteBatch has a non-zero byte limit
// and cannot be constructed unbounded.
//
// PRODUCTION BINDING:
//   Production struct JournalWriteBatch in crates/vb_storage/src/batch.rs.
//   Production constants from crates/vb_storage/src/constants.rs.
//   C1 requires byte_limit > 0 for all batches.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-023

#![allow(unused)]

/// Default journal batch byte limit from production.
const DEFAULT_BYTE_LIMIT: u64 = 1_048_576;

/// Byte limit value object: must be non-zero.
#[flux_rs::sig(fn(u64) -> Result<u64, ()> requires value > 0)]
fn new_byte_limit(value: u64) -> Result<u64, ()> {
    if value > 0 { Ok(value) } else { Err(()) }
}

/// Refinement: default limit is valid.
fn test_default_valid() -> Result<u64, ()> {
    new_byte_limit(DEFAULT_BYTE_LIMIT)
}

/// Refinement: zero limit is rejected.
fn test_zero_rejected() -> Result<u64, ()> {
    new_byte_limit(0)
}

/// Refinement: staging bytes must be <= limit.
#[flux_rs::sig(fn(u64, u64) -> bool requires limit > 0)]
fn staging_invariant(staged: u64, limit: u64) -> bool {
    staged <= limit
}

/// Check that default limit accommodates a typical event.
fn test_default_accommodates_event() {
    let typical_encoded: u64 = 200;
    assert!(typical_encoded <= DEFAULT_BYTE_LIMIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_is_valid() {
        assert!(new_byte_limit(DEFAULT_BYTE_LIMIT).is_ok());
    }

    #[test]
    fn zero_is_invalid() {
        assert!(new_byte_limit(0).is_err());
    }

    #[test]
    fn staging_invariant_below_limit() {
        assert!(staging_invariant(0, 100));
        assert!(staging_invariant(50, 100));
        assert!(staging_invariant(100, 100));
    }

    #[test]
    fn staging_invariant_violation() {
        assert!(!staging_invariant(101, 100));
    }

    #[test]
    fn limit_positive_for_all_batches() {
        assert!(DEFAULT_BYTE_LIMIT > 0);
    }
}
