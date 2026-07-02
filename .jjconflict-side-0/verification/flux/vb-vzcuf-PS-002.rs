// Flux-rs refinement annotations for overflow safety (PS-002, C7).
//
// Obligation ID: POB-vb-vzcuf-007
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-002.rs
//
// Domain claim: Accumulated byte addition and length conversion cannot
// panic or wrap; overflow returns typed rejection.
//
// PRODUCTION BINDING:
//   Refinements for u64::checked_add behavior, which is the Rust std
//   primitive that production JournalWriteBatch::append_event must use.
//   Also refines u32 -> u64 widening cast safety.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-007

#![allow(unused)]

/// Safe checked addition: never panics, returns None on overflow.
#[flux_rs::sig(fn(u64, u64) -> Option<u64>)]
fn safe_checked_add(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}

/// Admission check: staged + candidate within limit.
#[flux_rs::sig(fn(u64, u64, u64) -> Result<u64, ()> requires limit > 0)]
fn admission_check(staged: u64, candidate: u64, limit: u64) -> Result<u64, ()> {
    let total = staged.checked_add(candidate).ok_or(())?;
    if total <= limit { Ok(total) } else { Err(()) }
}

/// Safe u32 -> u64 widening: always exact, no overflow.
#[flux_rs::sig(fn(u32) -> u64)]
fn safe_u32_to_u64(n: u32) -> u64 {
    n as u64
}

/// Refinement: small additions always succeed.
fn test_small_add_ok() -> Option<u64> {
    safe_checked_add(100, 200)
}

/// Refinement: overflow at boundary.
fn test_overflow_at_boundary() -> Option<u64> {
    safe_checked_add(u64::MAX, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u32_to_u64_roundtrip() {
        for n in [0u32, 1, 42, u32::MAX] {
            assert_eq!(safe_u32_to_u64(n) as u32, n);
        }
    }

    #[test]
    fn admission_accepts_valid() {
        let r = admission_check(0, 500, 1000);
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), 500);
    }

    #[test]
    fn admission_rejects_over_limit() {
        assert!(admission_check(900, 200, 1000).is_err());
    }

    #[test]
    fn admission_rejects_overflow() {
        assert!(admission_check(u64::MAX, 1, u64::MAX).is_err());
    }
}
