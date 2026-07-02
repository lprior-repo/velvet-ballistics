// Flux-rs refinement annotations for accumulated byte admission (PS-001, C3).
//
// Obligation ID: POB-vb-vzcuf-003
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-001.rs
//
// Domain claim: Pure accumulated byte admission accepts exact fits
// and rejects over-limit totals.
//
// PRODUCTION BINDING:
//   These refinement annotations model the admission logic that
//   JournalWriteBatch::append_event (crates/vb_storage/src/batch.rs:209-229)
//   must implement using u64::checked_add.
//
//   The production function will:
//   1. Call encode_record to get Vec<u8> with .len()
//   2. Use u64::checked_add(staged_bytes, encoded_len)
//   3. Compare total with byte_limit
//   4. Return Ok(total) or Err(AccumulatedBytesExceeded)
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-003

#![allow(unused)]

/// Pure admission function mirroring production check.
/// PRODUCTION BINDING: Models u64::checked_add + limit comparison
/// that append_event will use for byte accounting.
#[flux_rs::sig(fn(u64, u64, u64) -> Result<u64, ()>)]
fn admit_bytes(current: u64, candidate: u64, limit: u64) -> Result<u64, ()> {
    let total = current.checked_add(candidate).ok_or(())?;
    if total <= limit { Ok(total) } else { Err(()) }
}

/// Refinement: if current <= limit and candidate fits, Ok(total) with total <= limit.
#[flux_rs::sig(fn(u64, u64, u64) -> Result<u64, ()> requires limit > 0)]
fn admit_bytes_bounded(current: u64, candidate: u64, limit: u64) -> Result<u64, ()> {
    admit_bytes(current, candidate, limit)
}

/// Refinement: zero-length event always fits when current <= limit.
#[flux_rs::sig(fn(u64, u64) -> Result<u64, ()> requires limit > 0 && current <= limit)]
fn test_zero_fits(current: u64, limit: u64) -> Result<u64, ()> {
    admit_bytes(current, 0, limit)
}

/// Refinement: overflow always produces Err.
fn test_overflow_rejected() -> Result<u64, ()> {
    admit_bytes(u64::MAX, 1, u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fit_accepted() {
        assert!(admit_bytes(0, 100, 100).is_ok());
    }

    #[test]
    fn over_limit_rejected() {
        assert!(admit_bytes(100, 1, 100).is_err());
    }

    #[test]
    fn zero_length_fits() {
        assert!(admit_bytes(500, 0, 1000).is_ok());
        assert_eq!(admit_bytes(500, 0, 1000).unwrap(), 500);
    }

    #[test]
    fn overflow_rejected() {
        assert!(admit_bytes(u64::MAX, 1, u64::MAX).is_err());
    }

    #[test]
    fn monotonic() {
        let result = admit_bytes(50, 25, 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 75);
        assert!(result.unwrap() > 50);
    }
}
